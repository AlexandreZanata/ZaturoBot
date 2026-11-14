//! Fixed service dispatch with durable admission/result fences, independent of model output.
use crate::now_ms;
use std::{path::PathBuf, sync::Arc};
use zaturo_core::{Assignment, FailureCode, ServiceRequest, ServiceResult, TaskKind};
use zaturo_db::{ServiceIntent, WorkerJournal};
use zaturo_llm::Cancellation;
use zaturo_protocol::ClientMessage;
use zaturo_youtube::{YoutubeClient, YoutubeConfig};

pub async fn execute(
    assignment: Assignment,
    journal: Arc<WorkerJournal>,
    youtube: Option<YoutubeConfig>,
    android_executable: Option<PathBuf>,
    cancellation: Cancellation,
) -> ClientMessage {
    let cancelled_code = if assignment
        .task
        .service
        .as_ref()
        .is_some_and(ServiceRequest::side_effecting)
    {
        FailureCode::ReconciliationRequired
    } else {
        FailureCode::ExecutionFailed
    };
    let result = tokio::select! {biased;
        _=cancellation.cancelled()=>Err(cancelled_code),
        result=execute_task(&assignment,journal,youtube,android_executable)=>result,
    };
    match result {
        Ok(result) => ClientMessage::CompletedService {
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
async fn execute_task(
    assignment: &Assignment,
    journal: Arc<WorkerJournal>,
    youtube: Option<YoutubeConfig>,
    android_executable: Option<PathBuf>,
) -> Result<ServiceResult, FailureCode> {
    if assignment.task.kind != TaskKind::Service
        || assignment.task.data_policy.is_some()
        || assignment.task.deadline_at <= now_ms()
        || assignment.lease_expires_at <= now_ms()
        || assignment.task.assigned_worker.as_deref() != Some(&assignment.task.target_worker)
        || assignment.task.lease_token.as_deref() != Some(&assignment.lease_token)
    {
        return Err(FailureCode::PolicyDenied);
    }
    let request = assignment
        .task
        .service
        .as_ref()
        .filter(|r| r.validates())
        .ok_or(FailureCode::PolicyDenied)?
        .clone();
    if assignment.task.required_capabilities != [request.capability()] {
        return Err(FailureCode::PolicyDenied);
    }
    let operation = assignment.task.operation_id.clone();
    let task = assignment.task.id.clone();
    let expected = request.clone();
    let local = journal.clone();
    let id = operation.clone();
    let intent =
        tokio::task::spawn_blocking(move || local.begin_service(&id, &task, &expected, now_ms()))
            .await
            .map_err(|_| FailureCode::ExecutionFailed)?
            .map_err(|_| FailureCode::ReconciliationRequired)?;
    let result = match intent {
        ServiceIntent::Cached(result) => {
            if !result.validates(&operation, &request) {
                return Err(FailureCode::ReconciliationRequired);
            }
            return Ok(result);
        }
        ServiceIntent::Reconcile => {
            let config = youtube.ok_or(FailureCode::ReconciliationRequired)?;
            let client =
                YoutubeClient::new(config).map_err(|_| FailureCode::ReconciliationRequired)?;
            client
                .reconcile(&operation, &request)
                .await
                .map_err(classify)?
        }
        ServiceIntent::Execute => match &request {
            ServiceRequest::AndroidDevices => {
                let executable = android_executable.ok_or(FailureCode::PolicyDenied)?;
                let adb = zaturo_android::AdbClient::new(&executable)
                    .map_err(|_| FailureCode::ExecutionFailed)?;
                let devices = adb
                    .discover(now_ms())
                    .await
                    .map_err(|_| FailureCode::ExecutionFailed)?;
                let count = devices.len();
                let mut result = ServiceResult {
                    operation_id: operation.clone(),
                    action: request.name().into(),
                    outcome: "completed".into(),
                    summary: format!(
                        "Observed {count} Android device(s); discovery does not grant screen or input access."
                    ),
                    data: serde_json::json!({"devices":devices,"total":count,"truncated":false}),
                };
                while serde_json::to_vec(&result)
                    .map_err(|_| FailureCode::ExecutionFailed)?
                    .len()
                    > 4096
                {
                    let values = result.data["devices"]
                        .as_array_mut()
                        .ok_or(FailureCode::ExecutionFailed)?;
                    if values.pop().is_none() {
                        return Err(FailureCode::ExecutionFailed);
                    }
                    result.data["truncated"] = serde_json::json!(true);
                }
                result
            }
            ServiceRequest::YoutubeProfiles
            | ServiceRequest::YoutubeRuns { .. }
            | ServiceRequest::YoutubeSubmit { .. }
            | ServiceRequest::YoutubeStatus { .. } => {
                let client = YoutubeClient::new(youtube.ok_or(FailureCode::PolicyDenied)?)
                    .map_err(classify)?;
                client
                    .execute(&operation, &request)
                    .await
                    .map_err(|error| classify_request(error, &request))?
            }
            _ => return Err(FailureCode::PolicyDenied),
        },
    };
    if !result.validates(&operation, &request) {
        return Err(FailureCode::ExecutionFailed);
    }
    let saved = result.clone();
    let local = journal;
    tokio::task::spawn_blocking(move || local.record_service(&operation, &saved, now_ms()))
        .await
        .map_err(|_| FailureCode::ExecutionFailed)?
        .map_err(|_| FailureCode::ReconciliationRequired)?;
    Ok(result)
}
fn classify(error: zaturo_youtube::Error) -> FailureCode {
    match error {
        zaturo_youtube::Error::PolicyDenied | zaturo_youtube::Error::InvalidRequest => {
            FailureCode::PolicyDenied
        }
        zaturo_youtube::Error::ReconciliationRequired => FailureCode::ReconciliationRequired,
        _ => FailureCode::ExecutionFailed,
    }
}

fn classify_request(error: zaturo_youtube::Error, request: &ServiceRequest) -> FailureCode {
    if request.side_effecting()
        && matches!(
            error,
            zaturo_youtube::Error::Timeout
                | zaturo_youtube::Error::OutputLimit
                | zaturo_youtube::Error::InvalidOutput
                | zaturo_youtube::Error::Rejected
        )
    {
        FailureCode::ReconciliationRequired
    } else {
        classify(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path, time::Duration};
    use zaturo_core::{ExecutionBudget, Task, TaskState};
    fn test_id() -> String {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        format!(
            "00000000-0000-4000-8000-{:012x}",
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        )
    }
    fn assignment(request: ServiceRequest) -> Assignment {
        let now = now_ms();
        let lease = "synthetic-service-lease".to_owned();
        Assignment {
            task: Task {
                service: Some(request.clone()),
                service_result: None,
                id: test_id(),
                operation_id: test_id(),
                kind: TaskKind::Service,
                request: "Execute the explicitly selected fixture service.".into(),
                target_worker: "work".into(),
                state: TaskState::Running,
                required_capabilities: vec![request.capability().into()],
                priority: 0,
                budget: ExecutionBudget::default(),
                max_attempts: 3,
                attempts: 1,
                created_at: now,
                updated_at: now,
                deadline_at: now + 300_000,
                assigned_worker: Some("work".into()),
                lease_token: Some(lease.clone()),
                lease_expires_at: Some(now + 30_000),
                result: None,
                data_policy: None,
                orchestration: None,
                failure: None,
            },
            lease_token: lease,
            lease_expires_at: now + 30_000,
        }
    }
    fn project(root: &Path) -> YoutubeConfig {
        let package = root.join("tools/ytbot");
        fs::create_dir_all(&package).unwrap();
        fs::write(root.join("tools/__init__.py"), b"").unwrap();
        fs::write(package.join("__init__.py"), b"").unwrap();
        fs::write(
            package.join("service.py"),
            include_str!("../../../crates/youtube/tests/fixtures/service.py"),
        )
        .unwrap();
        YoutubeConfig {
            python: "/usr/bin/python3".into(),
            project_root: root.to_owned(),
            allow_schedule: true,
        }
    }
    fn submit() -> ServiceRequest {
        ServiceRequest::YoutubeSubmit {
            channel: "channel".into(),
            account: "account".into(),
            count: 1,
            approved_schedule: true,
        }
    }
    #[tokio::test]
    async fn completed_service_result_is_cached_without_restarting_python() {
        let temp = tempfile::tempdir().unwrap();
        let config = project(temp.path());
        let journal = Arc::new(WorkerJournal::open(temp.path().join("journal.db")).unwrap());
        let task = assignment(ServiceRequest::YoutubeProfiles);
        let first = execute(
            task.clone(),
            journal.clone(),
            Some(config.clone()),
            None,
            Cancellation::new(),
        )
        .await;
        let ClientMessage::CompletedService { result: first, .. } = first else {
            panic!("fixture profile read failed")
        };
        fs::write(
            temp.path().join("tools/ytbot/service.py"),
            "raise RuntimeError('must not execute a cached service')",
        )
        .unwrap();
        let second = execute(task, journal, Some(config), None, Cancellation::new()).await;
        let ClientMessage::CompletedService { result: second, .. } = second else {
            panic!("cached result not reported")
        };
        assert_eq!(first, second);
        assert_eq!(
            fs::read_to_string(temp.path().join("calls.jsonl"))
                .unwrap()
                .lines()
                .count(),
            1
        );
    }
    #[tokio::test]
    async fn unknown_submission_only_reconciles_and_never_submits_again() {
        let temp = tempfile::tempdir().unwrap();
        let config = project(temp.path());
        let journal = Arc::new(WorkerJournal::open(temp.path().join("journal.db")).unwrap());
        let task = assignment(submit());
        fs::write(temp.path().join("mode"), "lost-response").unwrap();
        let first = execute(
            task.clone(),
            journal.clone(),
            Some(config.clone()),
            None,
            Cancellation::new(),
        )
        .await;
        assert!(matches!(
            first,
            ClientMessage::Failed {
                code: FailureCode::ReconciliationRequired,
                ..
            }
        ));
        fs::write(temp.path().join("mode"), "normal").unwrap();
        let second = execute(
            task,
            journal.clone(),
            Some(config),
            None,
            Cancellation::new(),
        )
        .await;
        let ClientMessage::CompletedService { result, .. } = second else {
            panic!("same-operation status was not reconciled")
        };
        assert_eq!(result.outcome, "accepted");
        let calls = fs::read_to_string(temp.path().join("calls.jsonl")).unwrap();
        assert_eq!(
            calls.lines().filter(|line| line.contains("submit")).count(),
            1
        );
        assert_eq!(
            calls.lines().filter(|line| line.contains("status")).count(),
            1
        );
        assert_eq!(journal.pending_service_jobs(20).unwrap().len(), 1);
    }
    #[tokio::test]
    async fn cancellation_of_uncertain_side_effect_keeps_the_journal_fence() {
        let temp = tempfile::tempdir().unwrap();
        let config = project(temp.path());
        let journal = Arc::new(WorkerJournal::open(temp.path().join("journal.db")).unwrap());
        let task = assignment(submit());
        fs::write(temp.path().join("mode"), "slow").unwrap();
        let cancel = Cancellation::new();
        let running = {
            let task = task.clone();
            let journal = journal.clone();
            let config = config.clone();
            let cancel = cancel.clone();
            tokio::spawn(async move { execute(task, journal, Some(config), None, cancel).await })
        };
        tokio::time::timeout(Duration::from_secs(2), async {
            while !temp.path().join("calls.jsonl").exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_secs(1), running)
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            result,
            ClientMessage::Failed {
                code: FailureCode::ReconciliationRequired,
                ..
            }
        ));
        fs::write(temp.path().join("mode"), "normal").unwrap();
        let result = execute(task, journal, Some(config), None, Cancellation::new()).await;
        assert!(matches!(
            result,
            ClientMessage::Failed {
                code: FailureCode::ReconciliationRequired,
                ..
            }
        ));
        let calls = fs::read_to_string(temp.path().join("calls.jsonl")).unwrap();
        assert_eq!(
            calls.lines().filter(|line| line.contains("submit")).count(),
            1
        );
    }
    #[tokio::test]
    async fn invalid_lease_or_missing_local_configuration_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let journal = Arc::new(WorkerJournal::open(temp.path().join("journal.db")).unwrap());
        let mut task = assignment(ServiceRequest::YoutubeProfiles);
        task.lease_expires_at = 0;
        let result = execute(task, journal.clone(), None, None, Cancellation::new()).await;
        assert!(matches!(
            result,
            ClientMessage::Failed {
                code: FailureCode::PolicyDenied,
                ..
            }
        ));
        let result = execute(
            assignment(ServiceRequest::AndroidDevices),
            journal,
            None,
            None,
            Cancellation::new(),
        )
        .await;
        assert!(matches!(
            result,
            ClientMessage::Failed {
                code: FailureCode::PolicyDenied,
                ..
            }
        ));
    }
}
