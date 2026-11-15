//! Compose the installed executor with the existing journal and aggregate Personal budget.
use crate::{config::WorkerConfig, now_ms};
use std::sync::Arc;
use zaturo_core::{Assignment, FailureCode, OpenCodeWorkOrder, ServiceRequest, ServiceResult};
use zaturo_db::{ServiceIntent, WorkerJournal};
use zaturo_llm::Cancellation;
use zaturo_protocol::ClientMessage;

pub async fn execute(
    work: Assignment,
    journal: Arc<WorkerJournal>,
    config: WorkerConfig,
    cancel: Cancellation,
) -> ClientMessage {
    match run(&work, journal, &config, cancel).await {
        Ok(result) => ClientMessage::CompletedService {
            task_id: work.task.id,
            lease_token: work.lease_token,
            result,
        },
        Err(code) => ClientMessage::Failed {
            task_id: work.task.id,
            lease_token: work.lease_token,
            code,
        },
    }
}
async fn run(
    work: &Assignment,
    journal: Arc<WorkerJournal>,
    config: &WorkerConfig,
    cancel: Cancellation,
) -> Result<ServiceResult, FailureCode> {
    let Some(
        request @ (ServiceRequest::YoutubeOpenCodeGenerate { .. }
        | ServiceRequest::AssistantRoute { .. }),
    ) = &work.task.service
    else {
        return Err(FailureCode::PolicyDenied);
    };
    let generation = matches!(request, ServiceRequest::YoutubeOpenCodeGenerate { .. });
    if !request.validates()
        || work.task.kind != zaturo_core::TaskKind::Service
        || work.task.data_policy.is_some()
        || work.task.target_worker != config.worker_id
        || work.task.assigned_worker.as_deref() != Some(&config.worker_id)
        || work.task.lease_token.as_deref() != Some(&work.lease_token)
        || work.lease_expires_at <= now_ms()
        || work.task.deadline_at <= now_ms()
        || work.task.required_capabilities != [request.capability()]
        || !config
            .capabilities
            .iter()
            .any(|c| c == request.capability())
    {
        return Err(FailureCode::PolicyDenied);
    }
    let executor = config.opencode.clone().ok_or(FailureCode::PolicyDenied)?;
    let youtube = if generation {
        Some(
            zaturo_youtube::YoutubeClient::new(
                config
                    .youtube
                    .clone()
                    .filter(|c| c.allow_generate)
                    .ok_or(FailureCode::PolicyDenied)?,
            )
            .map_err(|_| FailureCode::ExecutionFailed)?,
        )
    } else {
        None
    };
    let operation = work.task.operation_id.clone();
    let task = work.task.id.clone();
    let local = journal.clone();
    let id = operation.clone();
    let expected = request.clone();
    let intent =
        tokio::task::spawn_blocking(move || local.begin_service(&id, &task, &expected, now_ms()))
            .await
            .map_err(|_| FailureCode::ExecutionFailed)?
            .map_err(|_| FailureCode::ReconciliationRequired)?;
    if let ServiceIntent::Cached(result) = &intent {
        return if result.validates(&operation, request) {
            Ok(result.clone())
        } else {
            Err(FailureCode::ReconciliationRequired)
        };
    }
    let order = OpenCodeWorkOrder {
        operation_id: operation.clone(),
        task_id: work.task.id.clone(),
        target_worker: config.worker_id.clone(),
        work: match request {
            ServiceRequest::YoutubeOpenCodeGenerate { channel, count } => {
                zaturo_core::OpenCodeWork::Generate {
                    channel: channel.clone(),
                    count: *count,
                }
            }
            ServiceRequest::AssistantRoute {
                context,
                planner_worker,
                ..
            } => zaturo_core::OpenCodeWork::Interpret {
                context: context.clone(),
                planner_worker: planner_worker.clone(),
            },
            _ => unreachable!(),
        },
        owner_request: work.task.request.clone(),
        deadline_at: work.task.deadline_at,
        allowed_operations: if generation {
            ["generate", "status", "inspect_artifacts", "cancel_owned"]
                .map(str::to_owned)
                .to_vec()
        } else {
            vec!["select_registered_action".into()]
        },
        max_model_calls: 1,
        max_tool_calls: 1,
        max_output_bytes: 65536,
        runtime_seconds: 150,
    };
    let outcome = match intent {
        ServiceIntent::Reconcile => {
            zaturo_opencode::reconcile(&executor, &order, youtube.as_ref()).await
        }
        ServiceIntent::Execute => {
            let models = executor.models().map_err(|_| FailureCode::PolicyDenied)?;
            let budget = Arc::new(crate::assistant::AssistantBudget {
                journal: journal.clone(),
                limits: models.budget,
            });
            zaturo_opencode::dispatch(executor, order, youtube, budget, cancel).await
        }
        ServiceIntent::Cached(_) => unreachable!(),
    };
    let result=outcome.unwrap_or_else(|_|ServiceResult{operation_id:operation.clone(),action:request.name().into(),outcome:"blocked".into(),summary:"O executor instalado do OpenCode não pôde ser verificado ou reconciliado. O UUID foi preservado; nenhum lote será reenviado automaticamente. Verifique o executável, o perfil privado e o serviço de usuário neste PC.".into(),data:serde_json::json!({"code":"opencode_local_unverified"})});
    if !result.validates(&operation, request) {
        return Err(FailureCode::ReconciliationRequired);
    }
    let local = journal;
    let saved = result.clone();
    tokio::task::spawn_blocking(move || local.record_service(&operation, &saved, now_ms()))
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?
        .map_err(|_| FailureCode::ReconciliationRequired)?;
    Ok(result)
}
