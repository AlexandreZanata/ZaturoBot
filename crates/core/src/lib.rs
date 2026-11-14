//! Canonical domain contracts and deterministic task rules.

use serde::{Deserialize, Serialize};
mod files;
pub use files::*;

mod model;
pub use model::{
    AssistedResult, DataPolicy, FailureCode, ModelBudgetLimits, ModelBudgetSnapshot,
    ModelCallState, ModelReservation, ModelUsage, OrchestrationResult, ReservationStatus,
};

pub const TASK_STATES: &[&str] = &[
    "CREATED",
    "QUEUED",
    "ASSIGNED",
    "RUNNING",
    "WAITING_APPROVAL",
    "PAUSED",
    "RETRY_PENDING",
    "COMPLETED",
    "FAILED",
    "CANCELLED",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskState {
    Created,
    Queued,
    Assigned,
    Running,
    WaitingApproval,
    Paused,
    RetryPending,
    Completed,
    Failed,
    Cancelled,
}

impl TaskState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        use TaskState::*;
        matches!(
            (self, next),
            (Created, Queued | Cancelled)
                | (Queued, Assigned | Paused | Cancelled | Failed)
                | (
                    Assigned,
                    Running | RetryPending | Paused | Cancelled | Failed
                )
                | (
                    Running,
                    WaitingApproval | Paused | RetryPending | Completed | Failed | Cancelled
                )
                | (WaitingApproval, Queued | Paused | Cancelled | Failed)
                | (Paused, Queued | Cancelled | Failed)
                | (RetryPending, Queued | Paused | Cancelled | Failed)
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "CREATED",
            Self::Queued => "QUEUED",
            Self::Assigned => "ASSIGNED",
            Self::Running => "RUNNING",
            Self::WaitingApproval => "WAITING_APPROVAL",
            Self::Paused => "PAUSED",
            Self::RetryPending => "RETRY_PENDING",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    #[default]
    Diagnostic,
    AssistedDiagnostic,
}

impl TaskKind {
    pub fn capability(self) -> &'static str {
        match self {
            Self::Diagnostic => "diagnostic.v1",
            Self::AssistedDiagnostic => "agent.diagnostic.v1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewTask {
    pub kind: TaskKind,
    pub request: String,
    pub target_worker: String,
    pub idempotency_key: String,
    pub priority: i32,
    pub max_attempts: u32,
    pub timeout_seconds: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_policy: Option<DataPolicy>,
}

impl NewTask {
    pub fn diagnostic(
        target_worker: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Self {
        Self {
            kind: TaskKind::Diagnostic,
            request: "Run the safe deterministic diagnostic".into(),
            target_worker: target_worker.into(),
            idempotency_key: idempotency_key.into(),
            priority: 0,
            max_attempts: 3,
            timeout_seconds: 300,
            data_policy: None,
        }
    }

    /// The caller explicitly chooses public coding data; this is not inferred from request text.
    pub fn assisted_diagnostic(
        target_worker: impl Into<String>,
        idempotency_key: impl Into<String>,
    ) -> Self {
        Self {
            kind: TaskKind::AssistedDiagnostic,
            data_policy: Some(DataPolicy::PublicCoding),
            ..Self::diagnostic(target_worker, idempotency_key)
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.request.trim().is_empty() || self.request.len() > 4096 {
            return Err("request must contain 1 to 4096 bytes".into());
        }
        validate_identifier(&self.target_worker)?;
        if self.idempotency_key.is_empty() || self.idempotency_key.len() > 128 {
            return Err("idempotency key must contain 1 to 128 bytes".into());
        }
        if !(-100..=100).contains(&self.priority) {
            return Err("priority must be between -100 and 100".into());
        }
        if !(1..=3).contains(&self.max_attempts) {
            return Err("max_attempts must be between 1 and 3".into());
        }
        if !(1..=300).contains(&self.timeout_seconds) {
            return Err("timeout_seconds must be between 1 and 300".into());
        }
        if (self.kind == TaskKind::AssistedDiagnostic)
            != (self.data_policy == Some(DataPolicy::PublicCoding))
        {
            return Err("assisted diagnostics require explicit public_coding data policy; diagnostics accept none".into());
        }
        Ok(())
    }
}

pub fn validate_identifier(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("identifier must contain 1 to 64 ASCII letters, digits, '-' or '_'".into());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionBudget {
    pub max_tool_calls: u32,
    pub max_output_bytes: u32,
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self {
            max_tool_calls: 1,
            max_output_bytes: 4096,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticResult {
    pub operation_id: String,
    pub tool: String,
    pub os: String,
    pub arch: String,
    pub success: bool,
    pub summary: String,
}

impl DiagnosticResult {
    pub fn validates(&self, operation_id: &str) -> bool {
        self.operation_id == operation_id
            && self.tool == "diagnostic.v1"
            && self.success
            && self.os == "linux"
            && !self.arch.is_empty()
            && self.arch.len() <= 64
            && !self.summary.is_empty()
            && self.summary.len() <= 1024
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub operation_id: String,
    pub kind: TaskKind,
    pub request: String,
    pub target_worker: String,
    pub state: TaskState,
    pub required_capabilities: Vec<String>,
    pub priority: i32,
    pub budget: ExecutionBudget,
    pub max_attempts: u32,
    pub attempts: u32,
    pub created_at: i64,
    pub updated_at: i64,
    pub deadline_at: i64,
    pub assigned_worker: Option<String>,
    pub lease_token: Option<String>,
    pub lease_expires_at: Option<i64>,
    pub result: Option<DiagnosticResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_policy: Option<DataPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orchestration: Option<OrchestrationResult>,
    pub failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    pub task: Task,
    pub lease_token: String,
    pub lease_expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskEvent {
    pub sequence: i64,
    pub task_id: String,
    pub at: i64,
    pub state: TaskState,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Worker {
    pub id: String,
    pub capabilities: Vec<String>,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlAction {
    Pause,
    Resume,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub max_queued_tasks: u32,
    pub max_page_size: u32,
    pub lease_seconds: u64,
    pub max_workers: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_queued_tasks: 100,
            max_page_size: 100,
            lease_seconds: 30,
            max_workers: 8,
        }
    }
}

impl Limits {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=10_000).contains(&self.max_queued_tasks)
            || !(1..=1000).contains(&self.max_page_size)
            || !(2..=300).contains(&self.lease_seconds)
            || !(1..=64).contains(&self.max_workers)
        {
            return Err("limits exceed foundation bounds".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_states_cannot_transition() {
        for state in [
            TaskState::Completed,
            TaskState::Failed,
            TaskState::Cancelled,
        ] {
            for next in [TaskState::Queued, TaskState::Running, TaskState::Completed] {
                assert!(!state.can_transition_to(next));
            }
        }
    }

    #[test]
    fn lifecycle_does_not_skip_execution() {
        assert!(TaskState::Created.can_transition_to(TaskState::Queued));
        assert!(TaskState::Queued.can_transition_to(TaskState::Assigned));
        assert!(TaskState::Assigned.can_transition_to(TaskState::Running));
        assert!(TaskState::Running.can_transition_to(TaskState::Completed));
        assert!(!TaskState::Queued.can_transition_to(TaskState::Completed));
        assert!(!TaskState::Paused.can_transition_to(TaskState::Running));
    }

    #[test]
    fn invalid_task_limits_are_rejected() {
        let mut request = NewTask::diagnostic("worker", "key");
        assert!(request.validate().is_ok());
        request.max_attempts = 0;
        assert!(request.validate().is_err());
        assert!(validate_identifier("../worker").is_err());
        assert!(
            Limits {
                max_page_size: 0,
                ..Limits::default()
            }
            .validate()
            .is_err()
        );
    }
}
