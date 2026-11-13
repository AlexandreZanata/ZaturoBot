//! Worker-local dispatch, policy, durable model accounting, and result caching.
use crate::{config::WorkerConfig, now_ms};
use std::sync::Arc;
use zaturo_core::{
    Assignment, AssistedResult, DataPolicy, DiagnosticResult, FailureCode, ModelBudgetLimits,
    ModelReservation, ModelUsage, ReservationStatus, TaskKind,
};
use zaturo_db::WorkerJournal;
use zaturo_llm::{
    BudgetPort, Cancellation, Error as ModelError, FutureResult, ModelsConfig, OpenCodeGoClient,
    PrivacyClass, Router,
};
use zaturo_protocol::ClientMessage;
use zaturo_security::{ActionClass, permits};

pub fn load_router(config: &WorkerConfig) -> anyhow::Result<Option<Arc<Router>>> {
    let Some(path) = &config.models_config else {
        return Ok(None);
    };
    let models: ModelsConfig = toml::from_str(&std::fs::read_to_string(path)?).map_err(|_| {
        anyhow::anyhow!("invalid model configuration; source values are not logged")
    })?;
    models.validate()?;
    if !models.enabled {
        anyhow::ensure!(
            !config
                .capabilities
                .iter()
                .any(|c| c == "agent.diagnostic.v1"),
            "disabled model configuration cannot advertise assisted capability"
        );
        return Ok(None);
    }
    anyhow::ensure!(
        models.allow_global_processing,
        "the selected model requires explicit Global processing authorization"
    );
    let key = std::env::var("OPENCODE_GO_API_KEY").map_err(|_| {
        anyhow::anyhow!("OPENCODE_GO_API_KEY is required for enabled model routing")
    })?;
    let client = Arc::new(OpenCodeGoClient::new(&models, key)?);
    Ok(Some(Arc::new(Router::new(models, client)?)))
}

pub async fn execute(
    assignment: Assignment,
    journal: Arc<WorkerJournal>,
    router: Option<Arc<Router>>,
    cancellation: Cancellation,
) -> ClientMessage {
    let result = execute_task(&assignment, journal, router, &cancellation).await;
    match result {
        Ok(result) => ClientMessage::CompletedAssisted {
            task_id: assignment.task.id,
            lease_token: assignment.lease_token,
            result,
        },
        Err(code) => ClientMessage::Failed {
            task_id: assignment.task.id,
            lease_token: assignment.lease_token,
            code,
        },
    }
}

pub async fn diagnostic(
    assignment: Assignment,
    journal: Arc<WorkerJournal>,
    cancellation: Cancellation,
) -> ClientMessage {
    let operation = assignment.task.operation_id.clone();
    let task = assignment.task.id.clone();
    let result = tokio::task::spawn_blocking(move || {
        if cancellation.is_cancelled() {
            return Err(FailureCode::ExecutionFailed);
        }
        let cached = journal
            .begin(&operation, &task, now_ms())
            .map_err(|_| FailureCode::ExecutionFailed)?;
        let result = cached.unwrap_or_else(|| zaturo_tools::diagnostic(&operation));
        if cancellation.is_cancelled() {
            return Err(FailureCode::ExecutionFailed);
        }
        journal
            .record_result(&operation, &result, now_ms())
            .map_err(|_| FailureCode::ExecutionFailed)?;
        Ok(result)
    })
    .await;
    match result {
        Ok(Ok(result)) => ClientMessage::Completed {
            task_id: assignment.task.id,
            lease_token: assignment.lease_token,
            result,
        },
        _ => ClientMessage::Failed {
            task_id: assignment.task.id,
            lease_token: assignment.lease_token,
            code: FailureCode::ExecutionFailed,
        },
    }
}

async fn execute_task(
    assignment: &Assignment,
    journal: Arc<WorkerJournal>,
    router: Option<Arc<Router>>,
    cancellation: &Cancellation,
) -> Result<AssistedResult, FailureCode> {
    if assignment.task.kind != TaskKind::AssistedDiagnostic
        || assignment.task.data_policy != Some(DataPolicy::PublicCoding)
    {
        return Err(FailureCode::PolicyDenied);
    }
    let operation = assignment.task.operation_id.clone();
    let task = assignment.task.id.clone();
    let local = journal.clone();
    let cached =
        tokio::task::spawn_blocking(move || local.begin_assisted(&operation, &task, now_ms()))
            .await
            .map_err(|_| FailureCode::ExecutionFailed)?
            .map_err(|_| FailureCode::ReconciliationRequired)?;
    if let Some(cached) = cached {
        return Ok(cached);
    }
    let router = router.ok_or(FailureCode::ProviderUnavailable)?;
    let budget = JournalBudget {
        journal: journal.clone(),
        limits: router.config().budget.clone(),
    };
    let executor = JournalDiagnostic {
        journal: journal.clone(),
        cancellation: cancellation.clone(),
    };
    let input = zaturo_orchestrator::RunInput {
        task_id: assignment.task.id.clone(),
        operation_id: assignment.task.operation_id.clone(),
        owner_request: assignment.task.request.clone(),
        deadline_at: assignment.task.deadline_at,
        privacy: PrivacyClass::PublicCoding,
    };
    let outcome = zaturo_orchestrator::run(&router, &input, &budget, &executor, cancellation)
        .await
        .map_err(classify)?;
    let result = AssistedResult {
        diagnostic: outcome.diagnostic,
        orchestration: outcome.report,
    };
    let operation = assignment.task.operation_id.clone();
    let saved = result.clone();
    tokio::task::spawn_blocking(move || journal.record_assisted(&operation, &saved, now_ms()))
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?
        .map_err(|_| FailureCode::ExecutionFailed)?;
    Ok(result)
}

fn classify(error: zaturo_orchestrator::Error) -> FailureCode {
    use zaturo_orchestrator::Error;
    match error {
        Error::Model(ModelError::Budget) => FailureCode::BudgetExceeded,
        Error::Model(ModelError::Reconciliation | ModelError::Timeout | ModelError::Transport) => {
            FailureCode::ReconciliationRequired
        }
        Error::Model(
            ModelError::Disabled
            | ModelError::Authentication
            | ModelError::RateLimited { .. }
            | ModelError::Server(_)
            | ModelError::Rejected(_),
        ) => FailureCode::ProviderUnavailable,
        Error::PolicyDenied | Error::InvalidInput | Error::Model(ModelError::Denied(_)) => {
            FailureCode::PolicyDenied
        }
        Error::InvalidProposal
        | Error::Model(ModelError::InvalidResponse | ModelError::ResponseTooLarge) => {
            FailureCode::ModelRejected
        }
        _ => FailureCode::ExecutionFailed,
    }
}

pub struct JournalBudget {
    pub journal: Arc<WorkerJournal>,
    pub limits: ModelBudgetLimits,
}
impl BudgetPort for JournalBudget {
    fn reserve<'a>(&'a self, reservation: &'a ModelReservation) -> FutureResult<'a, ()> {
        Box::pin(async move {
            let journal = self.journal.clone();
            let limits = self.limits.clone();
            let reservation = reservation.clone();
            match tokio::task::spawn_blocking(move || {
                journal.reserve_model_call(&reservation, &limits, now_ms())
            })
            .await
            {
                Ok(Ok(ReservationStatus::Granted)) => Ok(()),
                Ok(Ok(ReservationStatus::Existing(_))) => Err(ModelError::Reconciliation),
                Ok(Err(zaturo_db::Error::BudgetExceeded)) => Err(ModelError::Budget),
                _ => Err(ModelError::Reconciliation),
            }
        })
    }
    fn settle<'a>(&'a self, request_id: &'a str, usage: &'a ModelUsage) -> FutureResult<'a, ()> {
        Box::pin(async move {
            let journal = self.journal.clone();
            let request = request_id.to_owned();
            let usage = usage.clone();
            match tokio::task::spawn_blocking(move || {
                journal.settle_model_call(&request, &usage, now_ms())
            })
            .await
            {
                Ok(Ok(())) => Ok(()),
                Ok(Err(zaturo_db::Error::BudgetExceeded)) => Err(ModelError::Budget),
                _ => Err(ModelError::Reconciliation),
            }
        })
    }
    fn uncertain<'a>(&'a self, request_id: &'a str) -> FutureResult<'a, ()> {
        Box::pin(async move {
            let journal = self.journal.clone();
            let request = request_id.to_owned();
            tokio::task::spawn_blocking(move || {
                journal.mark_model_call_uncertain(&request, now_ms())
            })
            .await
            .map_err(|_| ModelError::Reconciliation)?
            .map_err(|_| ModelError::Reconciliation)
        })
    }
}

struct JournalDiagnostic {
    journal: Arc<WorkerJournal>,
    cancellation: Cancellation,
}
impl zaturo_orchestrator::DiagnosticExecutor for JournalDiagnostic {
    fn execute<'a>(
        &'a self,
        operation_id: &'a str,
    ) -> zaturo_orchestrator::FutureResult<'a, DiagnosticResult> {
        Box::pin(async move {
            if self.cancellation.is_cancelled()
                || !permits(
                    "diagnostic.v1",
                    ActionClass::ReadOnly,
                    &["diagnostic.v1".into()],
                )
            {
                return Err(zaturo_orchestrator::Error::PolicyDenied);
            }
            let journal = self.journal.clone();
            let operation = operation_id.to_owned();
            let cancellation = self.cancellation.clone();
            tokio::task::spawn_blocking(move || {
                if cancellation.is_cancelled() {
                    return Err(zaturo_orchestrator::Error::Cancelled);
                }
                if let Some(cached) = journal
                    .result(&operation)
                    .map_err(|_| zaturo_orchestrator::Error::ToolFailed)?
                {
                    return Ok(cached);
                }
                let result = zaturo_tools::diagnostic(&operation);
                journal
                    .record_result(&operation, &result, now_ms())
                    .map_err(|_| zaturo_orchestrator::Error::ToolFailed)?;
                Ok(result)
            })
            .await
            .map_err(|_| zaturo_orchestrator::Error::ToolFailed)?
        })
    }
}
