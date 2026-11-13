//! Transactional application use cases shared by transport and CLI composition roots.

use std::path::Path;
use thiserror::Error;
use uuid::Uuid;
use zaturo_core::{
    Assignment, AssistedResult, ControlAction, DataPolicy, DiagnosticResult, ExecutionBudget,
    FailureCode, Limits, NewTask, OrchestrationResult, Task, TaskEvent, TaskKind, TaskState,
    Worker, validate_identifier,
};
use zaturo_db::{HubStore, Repository};

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid request: {0}")]
    Invalid(String),
    #[error("record not found")]
    NotFound,
    #[error("state conflict: {0}")]
    Conflict(String),
    #[error("configured capacity reached")]
    Capacity,
    #[error("persistence failed: {0}")]
    Storage(#[from] zaturo_db::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Engine {
    store: HubStore,
    limits: Limits,
}

impl Engine {
    pub fn open(path: impl AsRef<Path>, limits: Limits) -> Result<Self> {
        Self::new(HubStore::open(path)?, limits)
    }

    pub fn new(store: HubStore, limits: Limits) -> Result<Self> {
        limits.validate().map_err(Error::Invalid)?;
        Ok(Self { store, limits })
    }

    pub fn create(&self, request: NewTask, now: i64) -> Result<Task> {
        request.validate().map_err(Error::Invalid)?;
        let deadline_at = now
            .checked_add((request.timeout_seconds * 1000) as i64)
            .ok_or_else(|| Error::Invalid("timestamp overflows".into()))?;
        self.store.transaction(|repository| {
            if let Some((task, previous)) =
                repository.by_idempotency_key(&request.idempotency_key)?
            {
                return if previous == request {
                    Ok(task)
                } else {
                    Err(Error::Conflict(
                        "idempotency key has a different request".into(),
                    ))
                };
            }
            if repository.unfinished_count()? >= self.limits.max_queued_tasks {
                return Err(Error::Capacity);
            }
            let mut task = Task {
                id: Uuid::new_v4().to_string(),
                operation_id: Uuid::new_v4().to_string(),
                kind: request.kind,
                request: request.request.clone(),
                target_worker: request.target_worker.clone(),
                state: TaskState::Created,
                required_capabilities: vec![request.kind.capability().into()],
                priority: request.priority,
                budget: ExecutionBudget::default(),
                max_attempts: request.max_attempts,
                attempts: 0,
                created_at: now,
                updated_at: now,
                deadline_at,
                assigned_worker: None,
                lease_token: None,
                lease_expires_at: None,
                result: None,
                data_policy: request.data_policy,
                orchestration: None,
                failure: None,
            };
            repository.insert_task(&task, &request)?;
            repository.append_event(&task, now, "Task created")?;
            transition(repository, &mut task, TaskState::Queued, now, "Task queued")?;
            Ok(task)
        })
    }

    pub fn get(&self, id: &str) -> Result<Task> {
        self.store
            .read(|repository| repository.task(id)?.ok_or(Error::NotFound))
    }

    pub fn list(&self, limit: u32, offset: u32) -> Result<Vec<Task>> {
        self.page(limit, offset)?;
        self.store
            .read(|repository| Ok(repository.tasks(limit, offset)?))
    }

    pub fn events(&self, id: &str, limit: u32, offset: u32) -> Result<Vec<TaskEvent>> {
        self.page(limit, offset)?;
        self.store.read(|repository| {
            repository.task(id)?.ok_or(Error::NotFound)?;
            Ok(repository.events(id, limit, offset)?)
        })
    }

    pub fn workers(&self, limit: u32, offset: u32) -> Result<Vec<Worker>> {
        self.page(limit, offset)?;
        self.store
            .read(|repository| Ok(repository.workers(limit, offset)?))
    }

    pub fn register(&self, id: &str, capabilities: &[String], now: i64) -> Result<Worker> {
        validate_identifier(id).map_err(Error::Invalid)?;
        if capabilities.len() > 16
            || capabilities.iter().any(|capability| {
                !matches!(capability.as_str(), "diagnostic.v1" | "agent.diagnostic.v1")
            })
        {
            return Err(Error::Invalid(
                "only registered diagnostic and assisted diagnostic capabilities are supported"
                    .into(),
            ));
        }
        let mut capabilities = capabilities.to_vec();
        capabilities.sort();
        capabilities.dedup();
        let worker = Worker {
            id: id.into(),
            capabilities,
            last_seen_at: now,
        };
        self.store.transaction(|repository| {
            if repository.worker(id)?.is_none()
                && repository.worker_count()? >= self.limits.max_workers
            {
                return Err(Error::Capacity);
            }
            repository.save_worker(&worker)?;
            Ok(worker)
        })
    }

    pub fn claim(&self, worker_id: &str, now: i64) -> Result<Option<Assignment>> {
        self.recover(now)?;
        self.store.transaction(|repository| {
            let worker = repository.worker(worker_id)?.ok_or(Error::NotFound)?;
            let task = if let Some(task) = repository.active_for_worker(worker_id)? {
                task
            } else {
                let Some(mut task) = repository.candidate(worker_id, &worker.capabilities, now)?
                else {
                    return Ok(None);
                };
                if !task
                    .required_capabilities
                    .iter()
                    .all(|capability| worker.capabilities.contains(capability))
                {
                    return Ok(None);
                }
                if task.attempts >= task.max_attempts {
                    task.failure = Some("Task attempt budget exhausted".into());
                    transition(
                        repository,
                        &mut task,
                        TaskState::Failed,
                        now,
                        "Attempt budget exhausted",
                    )?;
                    return Ok(None);
                }
                task.attempts += 1;
                task.assigned_worker = Some(worker_id.into());
                task.lease_token = Some(Uuid::new_v4().to_string());
                task.lease_expires_at = Some(lease_deadline(
                    now,
                    self.limits.lease_seconds,
                    task.deadline_at,
                )?);
                transition(
                    repository,
                    &mut task,
                    TaskState::Assigned,
                    now,
                    "Task assigned",
                )?;
                task
            };
            if !task
                .required_capabilities
                .iter()
                .all(|capability| worker.capabilities.contains(capability))
            {
                return Ok(None);
            }
            if task.lease_expires_at.is_none_or(|expires| expires <= now) || task.deadline_at <= now
            {
                return Ok(None);
            }
            Ok(Some(Assignment {
                lease_token: task
                    .lease_token
                    .clone()
                    .ok_or_else(|| Error::Conflict("active task has no lease".into()))?,
                lease_expires_at: task
                    .lease_expires_at
                    .ok_or_else(|| Error::Conflict("active task has no lease expiry".into()))?,
                task,
            }))
        })
    }

    pub fn start(
        &self,
        task_id: &str,
        worker_id: &str,
        lease_token: &str,
        now: i64,
    ) -> Result<Task> {
        self.store.transaction(|repository| {
            let mut task = repository.task(task_id)?.ok_or(Error::NotFound)?;
            check_lease(&task, worker_id, lease_token, now)?;
            match task.state {
                TaskState::Assigned => transition(
                    repository,
                    &mut task,
                    TaskState::Running,
                    now,
                    "Diagnostic execution started",
                )?,
                TaskState::Running => {}
                _ => return Err(Error::Conflict("task is not assigned or running".into())),
            }
            Ok(task)
        })
    }

    pub fn heartbeat(&self, worker_id: &str, lease_token: Option<&str>, now: i64) -> Result<()> {
        self.store.transaction(|repository| {
            let mut worker = repository.worker(worker_id)?.ok_or(Error::NotFound)?;
            if let Some(lease_token) = lease_token {
                let mut task = repository
                    .active_for_worker(worker_id)?
                    .ok_or_else(|| Error::Conflict("worker has no active task".into()))?;
                check_lease(&task, worker_id, lease_token, now)?;
                task.lease_expires_at = Some(lease_deadline(
                    now,
                    self.limits.lease_seconds,
                    task.deadline_at,
                )?);
                task.updated_at = now;
                repository.save_task(&task)?;
            }
            worker.last_seen_at = now;
            repository.save_worker(&worker)?;
            Ok(())
        })
    }

    pub fn complete(
        &self,
        task_id: &str,
        worker_id: &str,
        lease_token: &str,
        result: DiagnosticResult,
        now: i64,
    ) -> Result<Task> {
        self.commit_completion(
            task_id,
            worker_id,
            lease_token,
            Completion {
                result,
                orchestration: None,
            },
            now,
        )
    }

    pub fn complete_assisted(
        &self,
        task_id: &str,
        worker_id: &str,
        lease_token: &str,
        result: DiagnosticResult,
        orchestration: OrchestrationResult,
        now: i64,
    ) -> Result<Task> {
        self.commit_completion(
            task_id,
            worker_id,
            lease_token,
            Completion {
                result,
                orchestration: Some(orchestration),
            },
            now,
        )
    }

    fn commit_completion(
        &self,
        task_id: &str,
        worker_id: &str,
        lease_token: &str,
        completion: Completion,
        now: i64,
    ) -> Result<Task> {
        self.store.transaction(|repository| {
            let mut task = repository.task(task_id)?.ok_or(Error::NotFound)?;
            let assisted = completion.orchestration.is_some();
            if assisted != (task.kind == TaskKind::AssistedDiagnostic)
                || (assisted && task.data_policy != Some(DataPolicy::PublicCoding))
            {
                return Err(Error::Invalid(
                    "completion does not match the authorized task kind and data policy".into(),
                ));
            }
            if task.state == TaskState::Completed
                && task.assigned_worker.as_deref() == Some(worker_id)
                && task.lease_token.as_deref() == Some(lease_token)
                && task.result.as_ref() == Some(&completion.result)
                && task.orchestration == completion.orchestration
            {
                return Ok(task);
            }
            check_lease(&task, worker_id, lease_token, now)?;
            if task.state != TaskState::Running {
                return Err(Error::Conflict("completion requires a running task".into()));
            }
            let evidence_valid = completion.result.validates(&task.operation_id)
                && completion
                    .orchestration
                    .as_ref()
                    .is_none_or(OrchestrationResult::validates);
            let output_size = match &completion.orchestration {
                Some(orchestration) => serde_json::to_vec(&AssistedResult {
                    diagnostic: completion.result.clone(),
                    orchestration: orchestration.clone(),
                }),
                None => serde_json::to_vec(&completion.result),
            }
            .map_err(|_| Error::Invalid("result serialization failed".into()))?
            .len();
            if !evidence_valid || output_size > task.budget.max_output_bytes as usize {
                return Err(Error::Invalid(
                    "result does not satisfy typed acceptance criteria".into(),
                ));
            }
            task.result = Some(completion.result);
            task.orchestration = completion.orchestration;
            transition(
                repository,
                &mut task,
                TaskState::Completed,
                now,
                "Typed task acceptance criteria satisfied",
            )?;
            Ok(task)
        })
    }

    pub fn fail(
        &self,
        task_id: &str,
        worker_id: &str,
        lease_token: &str,
        code: FailureCode,
        now: i64,
    ) -> Result<Task> {
        self.store.transaction(|repository| {
            let mut task = repository.task(task_id)?.ok_or(Error::NotFound)?;
            if task.state == TaskState::Failed
                && task.assigned_worker.as_deref() == Some(worker_id)
                && task.lease_token.as_deref() == Some(lease_token)
                && task.failure.as_deref() == Some(code.as_str())
            {
                return Ok(task);
            }
            check_lease(&task, worker_id, lease_token, now)?;
            if task.state != TaskState::Running {
                return Err(Error::Conflict(
                    "failure reporting requires a running task".into(),
                ));
            }
            task.failure = Some(code.as_str().into());
            transition(
                repository,
                &mut task,
                TaskState::Failed,
                now,
                "Worker reported a bounded failure category",
            )?;
            Ok(task)
        })
    }

    pub fn control(&self, task_id: &str, action: ControlAction, now: i64) -> Result<Task> {
        self.store.transaction(|repository| {
            let mut task = repository.task(task_id)?.ok_or(Error::NotFound)?;
            let (next, message) = match action {
                ControlAction::Pause => (TaskState::Paused, "Owner paused task; lease revoked"),
                ControlAction::Resume => (TaskState::Queued, "Owner resumed task"),
                ControlAction::Cancel => {
                    (TaskState::Cancelled, "Owner cancelled task; lease revoked")
                }
            };
            if task.state == next {
                return Ok(task);
            }
            if matches!(action, ControlAction::Resume) && task.state != TaskState::Paused {
                return Err(Error::Conflict("only paused tasks can resume".into()));
            }
            if task.deadline_at <= now && action != ControlAction::Cancel {
                return Err(Error::Conflict("task deadline expired".into()));
            }
            revoke(&mut task);
            transition(repository, &mut task, next, now, message)?;
            Ok(task)
        })
    }

    /// Process a bounded page; repeated calls eventually reconcile a larger backlog.
    pub fn recover(&self, now: i64) -> Result<usize> {
        self.store.transaction(|repository| {
            let tasks = repository.expired(now, self.limits.max_page_size)?;
            let count = tasks.len();
            for mut task in tasks {
                revoke(&mut task);
                if task.deadline_at <= now {
                    task.failure = Some("Task deadline expired".into());
                    transition(
                        repository,
                        &mut task,
                        TaskState::Failed,
                        now,
                        "Task deadline expired",
                    )?;
                } else if task.attempts >= task.max_attempts {
                    task.failure =
                        Some("Diagnostic retry budget exhausted after lease expiration".into());
                    transition(
                        repository,
                        &mut task,
                        TaskState::Failed,
                        now,
                        "Retry budget exhausted",
                    )?;
                } else {
                    transition(
                        repository,
                        &mut task,
                        TaskState::RetryPending,
                        now,
                        "Lease expired; diagnostic-only safe retry pending",
                    )?;
                    transition(
                        repository,
                        &mut task,
                        TaskState::Queued,
                        now,
                        "Diagnostic requeued with the same operation ID",
                    )?;
                }
            }
            Ok(count)
        })
    }

    fn page(&self, limit: u32, offset: u32) -> Result<()> {
        if limit == 0 || limit > self.limits.max_page_size || offset > 1_000_000 {
            return Err(Error::Invalid(
                "pagination exceeds configured limits".into(),
            ));
        }
        Ok(())
    }
}

struct Completion {
    result: DiagnosticResult,
    orchestration: Option<OrchestrationResult>,
}

fn check_lease(task: &Task, worker_id: &str, lease_token: &str, now: i64) -> Result<()> {
    if !matches!(task.state, TaskState::Assigned | TaskState::Running)
        || task.assigned_worker.as_deref() != Some(worker_id)
        || task.lease_token.as_deref() != Some(lease_token)
        || task.lease_expires_at.is_none_or(|expiry| expiry <= now)
        || task.deadline_at <= now
    {
        return Err(Error::Conflict(
            "lease is stale, expired, revoked, or owned by another worker".into(),
        ));
    }
    Ok(())
}

fn revoke(task: &mut Task) {
    task.lease_token = None;
    task.lease_expires_at = None;
    task.assigned_worker = None;
}

fn lease_deadline(now: i64, seconds: u64, task_deadline: i64) -> Result<i64> {
    now.checked_add((seconds * 1000) as i64)
        .map(|deadline| deadline.min(task_deadline))
        .ok_or_else(|| Error::Invalid("timestamp overflows".into()))
}

fn transition(
    repository: &Repository<'_>,
    task: &mut Task,
    next: TaskState,
    now: i64,
    message: &str,
) -> Result<()> {
    if !task.state.can_transition_to(next) {
        return Err(Error::Conflict(format!(
            "illegal transition from {} to {}",
            task.state.as_str(),
            next.as_str()
        )));
    }
    task.state = next;
    task.updated_at = now;
    repository.save_task(task)?;
    repository.append_event(task, now, message)?;
    Ok(())
}
