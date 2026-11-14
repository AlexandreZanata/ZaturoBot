//! One bounded Console chat call. It cannot invoke tools or promote text to authorization.
use crate::{config::WorkerConfig, now_ms};
use std::sync::Arc;
use zaturo_core::{
    Assignment, FailureCode, ModelReservation, ModelUsage, ReservationStatus, ServiceRequest,
    ServiceResult,
};
use zaturo_db::{ServiceIntent, WorkerJournal};
use zaturo_llm::{
    BudgetPort, CallContext, Cancellation, Error, FutureResult, ModelsConfig, OpenCodeGoClient,
    PrivacyClass, Router,
};
use zaturo_protocol::ClientMessage;

pub fn load(config: &WorkerConfig) -> anyhow::Result<Option<Arc<Router>>> {
    let Some(path) = &config.assistant_models_config else {
        return Ok(None);
    };
    let models: ModelsConfig = toml::from_str(&std::fs::read_to_string(path)?)
        .map_err(|_| anyhow::anyhow!("invalid assistant model profile"))?;
    models.validate()?;
    anyhow::ensure!(
        models.provider == "opencode-console"
            && models.allowed_privacy == [PrivacyClass::Personal]
            && models.max_attempts == 1,
        "assistant requires explicit Console personal route without retry"
    );
    if !models.enabled {
        return Ok(None);
    }
    let key = std::env::var("OPENCODE_GO_API_KEY")
        .map_err(|_| anyhow::anyhow!("assistant provider key unavailable"))?;
    let client = Arc::new(OpenCodeGoClient::new_console(&models, key)?);
    Ok(Some(Arc::new(Router::new(models, client)?)))
}
pub async fn execute(
    work: Assignment,
    journal: Arc<WorkerJournal>,
    config: WorkerConfig,
    cancel: Cancellation,
) -> ClientMessage {
    let router = load(&config).unwrap_or_default();
    let result = tokio::select! {biased;_=cancel.cancelled()=>Err(FailureCode::ReconciliationRequired),result=run(&work,journal,router,&cancel)=>result};
    match result {
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
pub(crate) async fn run(
    work: &Assignment,
    journal: Arc<WorkerJournal>,
    router: Option<Arc<Router>>,
    cancel: &Cancellation,
) -> Result<ServiceResult, FailureCode> {
    let request = work
        .task
        .service
        .as_ref()
        .ok_or(FailureCode::PolicyDenied)?;
    let ServiceRequest::AssistantChat {
        message,
        context: history,
    } = request
    else {
        return Err(FailureCode::PolicyDenied);
    };
    if !request.validates()
        || work.task.kind != zaturo_core::TaskKind::Service
        || work.task.data_policy.is_some()
        || work.task.assigned_worker.as_deref() != Some(&work.task.target_worker)
        || work.task.lease_token.as_deref() != Some(&work.lease_token)
        || work.task.deadline_at <= now_ms()
        || work.lease_expires_at <= now_ms()
        || work.task.required_capabilities != ["assistant.chat.v1"]
    {
        return Err(FailureCode::PolicyDenied);
    }
    let operation = work.task.operation_id.clone();
    let task = work.task.id.clone();
    let expected = request.clone();
    let local = journal.clone();
    let op = operation.clone();
    match tokio::task::spawn_blocking(move || local.begin_service(&op, &task, &expected, now_ms()))
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?
        .map_err(|_| FailureCode::ReconciliationRequired)?
    {
        ServiceIntent::Cached(result) => {
            return if result.validates(&operation, request) {
                Ok(result)
            } else {
                Err(FailureCode::ReconciliationRequired)
            };
        }
        ServiceIntent::Reconcile => return Err(FailureCode::ReconciliationRequired),
        ServiceIntent::Execute => {}
    }
    let mut result=ServiceResult{operation_id:operation.clone(),action:request.name().into(),outcome:"blocked".into(),summary:"A IA de conversa não está configurada neste worker. O Codex deve ativar o perfil Console; a rota de código Go permanece separada.".into(),data:serde_json::json!({"code":"assistant_unavailable"})};
    if let Some(router) = router {
        let budget = AssistantBudget {
            journal: journal.clone(),
            limits: router.config().budget.clone(),
        };
        let context = CallContext {
            task_id: work.task.id.clone(),
            operation_id: operation.clone(),
            step: 1,
            deadline_at: work.task.deadline_at,
            privacy: PrivacyClass::Personal,
        };
        match zaturo_orchestrator::chat(&router, &context, message, history, &budget, cancel).await
        {
            Ok(response) if zaturo_core::ordinary_text(&response.content, 1500) => {
                result.outcome = "completed".into();
                result.summary = response.content;
                result.data = serde_json::json!({"model":response.model,"usage":response.usage,"model_calls":response.model_calls});
            }
            Ok(_) => {
                result.summary="A resposta da IA excedeu o contrato de texto seguro. Nenhuma ação foi executada.".into();
                result.data = serde_json::json!({"code":"invalid_response"});
            }
            Err(error) => {
                let (code, text) = failure(&error);
                result.summary = text.into();
                result.data = serde_json::json!({"code":code});
            }
        }
    }
    if !result.validates(&operation, request) {
        return Err(FailureCode::ModelRejected);
    }
    let local = journal;
    let saved = result.clone();
    tokio::task::spawn_blocking(move || local.record_service(&operation, &saved, now_ms()))
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?
        .map_err(|_| FailureCode::ReconciliationRequired)?;
    Ok(result)
}
pub(crate) fn failure(error: &Error) -> (&'static str, &'static str) {
    match error {
        Error::Authentication => (
            "provider_authentication",
            "O provedor recusou a chave ou a permissão da API Console. Verifique a configuração privada no computador; não envie a chave no chat.",
        ),
        Error::Budget => (
            "budget_exceeded",
            "O limite de tokens da IA foi atingido. A solicitação não será repetida automaticamente.",
        ),
        Error::Timeout | Error::Transport | Error::Reconciliation => (
            "provider_uncertain",
            "A conexão com a IA foi interrompida e a cobrança pode ter ocorrido. Não repetirei a chamada automaticamente; é necessário verificar o resultado.",
        ),
        Error::RateLimited { .. } => (
            "provider_rate_limited",
            "O provedor limitou as chamadas. A mensagem foi mantida; nenhuma ação foi executada nem repetida automaticamente.",
        ),
        Error::Rejected(402) => (
            "provider_balance",
            "A API Console não tem saldo ou permissão de cobrança disponível. Verifique o Console; o plano Go de código não será usado como alternativa.",
        ),
        _ => (
            "provider_unavailable",
            "A IA não concluiu uma resposta válida. Nenhuma ação foi executada; verifique o estado do provedor no computador.",
        ),
    }
}
pub(crate) struct AssistantBudget {
    pub(crate) journal: Arc<WorkerJournal>,
    pub(crate) limits: zaturo_core::ModelBudgetLimits,
}
impl BudgetPort for AssistantBudget {
    fn reserve<'a>(&'a self, reservation: &'a ModelReservation) -> FutureResult<'a, ()> {
        Box::pin(async move {
            let journal = self.journal.clone();
            let limits = self.limits.clone();
            let value = reservation.clone();
            match tokio::task::spawn_blocking(move || {
                journal.reserve_assistant_call(&value, &limits, now_ms())
            })
            .await
            {
                Ok(Ok(ReservationStatus::Granted)) => Ok(()),
                Ok(Err(zaturo_db::Error::BudgetExceeded)) => Err(Error::Budget),
                _ => Err(Error::Reconciliation),
            }
        })
    }
    fn settle<'a>(&'a self, id: &'a str, usage: &'a ModelUsage) -> FutureResult<'a, ()> {
        Box::pin(async move {
            let journal = self.journal.clone();
            let id = id.to_owned();
            let usage = usage.clone();
            match tokio::task::spawn_blocking(move || {
                journal.settle_assistant_call(&id, &usage, now_ms())
            })
            .await
            {
                Ok(Ok(())) => Ok(()),
                Ok(Err(zaturo_db::Error::BudgetExceeded)) => Err(Error::Budget),
                _ => Err(Error::Reconciliation),
            }
        })
    }
    fn uncertain<'a>(&'a self, id: &'a str) -> FutureResult<'a, ()> {
        Box::pin(async move {
            let journal = self.journal.clone();
            let id = id.to_owned();
            tokio::task::spawn_blocking(move || journal.uncertain_assistant_call(&id, now_ms()))
                .await
                .map_err(|_| Error::Reconciliation)?
                .map_err(|_| Error::Reconciliation)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zaturo_core::{ExecutionBudget, Task, TaskKind, TaskState};
    use zaturo_llm::{
        ModelResponse,
        fake::{FakeClient, FakeReply},
    };
    fn assignment() -> Assignment {
        let now = now_ms();
        let lease = "synthetic-owner-chat-lease".to_owned();
        Assignment {
            task: Task {
                id: "synthetic-chat".into(),
                operation_id: "synthetic-chat-operation".into(),
                kind: TaskKind::Service,
                request: "Synthetic owner conversation".into(),
                target_worker: "home".into(),
                state: TaskState::Running,
                required_capabilities: vec!["assistant.chat.v1".into()],
                priority: 0,
                budget: ExecutionBudget::default(),
                max_attempts: 1,
                attempts: 1,
                created_at: now,
                updated_at: now,
                deadline_at: now + 60000,
                assigned_worker: Some("home".into()),
                lease_token: Some(lease.clone()),
                lease_expires_at: Some(now + 30000),
                result: None,
                data_policy: None,
                orchestration: None,
                failure: None,
                service: Some(ServiceRequest::AssistantChat {
                    message: "Synthetic owner conversation".into(),
                    context: vec![],
                }),
                service_result: None,
            },
            lease_token: lease,
            lease_expires_at: now + 30000,
        }
    }
    fn router(fake: Arc<FakeClient>) -> Arc<Router> {
        let mut config: ModelsConfig = toml::from_str(include_str!(
            "../../../config/assistant-models.example.toml"
        ))
        .unwrap();
        config.enabled = true;
        config.allow_global_processing = true;
        Arc::new(Router::new(config, fake).unwrap())
    }
    #[tokio::test]
    async fn completed_reply_is_cached_across_reopen_without_another_paid_call() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("worker.db");
        let journal = Arc::new(WorkerJournal::open(&path).unwrap());
        let fake = Arc::new(FakeClient::new([FakeReply::Answer(ModelResponse {
            model: zaturo_llm::DEEPSEEK_MODEL.into(),
            content: "Resposta sintética, nenhuma ação executada.".into(),
            usage: ModelUsage {
                input_tokens: 10,
                output_tokens: 10,
                cost_microusd: None,
            },
            finish_reason: "stop".into(),
            duration_ms: 1,
            model_calls: 1,
            usage_complete: true,
        })]));
        let router = router(fake.clone());
        let task = assignment();
        let first = run(&task, journal, Some(router.clone()), &Cancellation::new())
            .await
            .unwrap();
        assert_eq!(first.outcome, "completed");
        let reopened = Arc::new(WorkerJournal::open(&path).unwrap());
        let second = run(&task, reopened, Some(router), &Cancellation::new())
            .await
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(fake.calls(), 1);
    }
    #[tokio::test]
    async fn an_unknown_provider_outcome_is_reported_once_and_never_reissued() {
        let temp = tempfile::tempdir().unwrap();
        let journal = Arc::new(WorkerJournal::open(temp.path().join("worker.db")).unwrap());
        let fake = Arc::new(FakeClient::new([FakeReply::Error(Error::Transport)]));
        let router = router(fake.clone());
        let task = assignment();
        let first = run(
            &task,
            journal.clone(),
            Some(router.clone()),
            &Cancellation::new(),
        )
        .await
        .unwrap();
        assert_eq!(first.outcome, "blocked");
        assert_eq!(first.data["code"], "provider_uncertain");
        let second = run(&task, journal, Some(router), &Cancellation::new())
            .await
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(fake.calls(), 1);
    }
}
