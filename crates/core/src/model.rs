//! Explicit disclosure grants, bounded model evidence, and durable accounting contracts.
use crate::DiagnosticResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataPolicy {
    PublicCoding,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: Option<u64>,
}
impl ModelUsage {
    pub fn total_tokens(&self) -> Option<u64> {
        self.input_tokens.checked_add(self.output_tokens)
    }
    pub fn validates(&self) -> bool {
        self.total_tokens().is_some_and(|n| n <= 1_000_000_000)
            && self.cost_microusd.is_none_or(|n| n <= 1_000_000_000_000)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrchestrationResult {
    pub model: String,
    pub steps: u32,
    pub summary: String,
    pub usage: ModelUsage,
    pub model_calls: u32,
    #[serde(default)]
    pub usage_complete: bool,
}
impl OrchestrationResult {
    pub fn validates(&self) -> bool {
        !self.model.is_empty()
            && self.model.len() <= 128
            && self
                .model
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-/:".contains(&b))
            && (1..=16).contains(&self.steps)
            && (1..=16).contains(&self.model_calls)
            && !self.summary.trim().is_empty()
            && self.summary.len() <= 1024
            && self.usage.validates()
            && (self.usage_complete || self.usage.cost_microusd.is_none())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistedResult {
    pub diagnostic: DiagnosticResult,
    pub orchestration: OrchestrationResult,
}
impl AssistedResult {
    pub fn validates(&self, operation_id: &str) -> bool {
        self.diagnostic.validates(operation_id) && self.orchestration.validates()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    ProviderUnavailable,
    ModelRejected,
    PolicyDenied,
    BudgetExceeded,
    ReconciliationRequired,
    ExecutionFailed,
}
impl FailureCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProviderUnavailable => "provider_unavailable",
            Self::ModelRejected => "model_rejected",
            Self::PolicyDenied => "policy_denied",
            Self::BudgetExceeded => "budget_exceeded",
            Self::ReconciliationRequired => "reconciliation_required",
            Self::ExecutionFailed => "execution_failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelReservation {
    pub request_id: String,
    pub operation_id: String,
    pub task_id: String,
    pub estimated_input_tokens: u64,
    pub max_output_tokens: u64,
}
impl ModelReservation {
    pub fn total_tokens(&self) -> Option<u64> {
        self.estimated_input_tokens
            .checked_add(self.max_output_tokens)
    }
    pub fn validates(&self) -> bool {
        [&self.request_id, &self.operation_id, &self.task_id]
            .iter()
            .all(|v| {
                !v.is_empty()
                    && v.len() <= 192
                    && v.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"._-:".contains(&b))
            })
            && self.estimated_input_tokens > 0
            && self.max_output_tokens > 0
            && self.total_tokens().is_some_and(|n| n <= 1_000_000_000)
    }
    pub fn accepts_usage(&self, usage: &ModelUsage) -> bool {
        usage.validates()
            && usage.input_tokens <= self.estimated_input_tokens
            && usage.output_tokens <= self.max_output_tokens
            && usage
                .total_tokens()
                .zip(self.total_tokens())
                .is_some_and(|(actual, reserved)| actual <= reserved)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelBudgetSnapshot {
    pub task_tokens: u64,
    pub day_tokens: u64,
    pub month_tokens: u64,
    pub task_model_calls: u32,
    pub operation_has_overrun: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelBudgetLimits {
    pub per_request_tokens: u64,
    pub per_task_tokens: u64,
    pub per_day_tokens: u64,
    pub per_month_tokens: u64,
    pub max_task_model_calls: u32,
}
impl Default for ModelBudgetLimits {
    fn default() -> Self {
        Self {
            per_request_tokens: 8192,
            per_task_tokens: 32_768,
            per_day_tokens: 100_000,
            per_month_tokens: 1_000_000,
            max_task_model_calls: 4,
        }
    }
}
impl ModelBudgetLimits {
    pub fn validates(&self) -> bool {
        self.per_request_tokens > 0
            && self.per_request_tokens <= self.per_task_tokens
            && self.per_task_tokens <= self.per_day_tokens
            && self.per_day_tokens <= self.per_month_tokens
            && self.per_month_tokens <= 1_000_000_000
            && (1..=16).contains(&self.max_task_model_calls)
    }
    pub fn permits(&self, reservation: &ModelReservation, snapshot: &ModelBudgetSnapshot) -> bool {
        let Some(reserved) = reservation.total_tokens() else {
            return false;
        };
        let within =
            |spent: u64, maximum: u64| spent.checked_add(reserved).is_some_and(|n| n <= maximum);
        self.validates()
            && reservation.validates()
            && !snapshot.operation_has_overrun
            && reserved <= self.per_request_tokens
            && snapshot.task_model_calls < self.max_task_model_calls
            && within(snapshot.task_tokens, self.per_task_tokens)
            && within(snapshot.day_tokens, self.per_day_tokens)
            && within(snapshot.month_tokens, self.per_month_tokens)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCallState {
    Reserved,
    Settled,
    Uncertain,
    Overrun,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationStatus {
    Granted,
    Existing(ModelCallState),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_coding_requires_an_explicit_task_grant() {
        let mut task = crate::NewTask::assisted_diagnostic("worker", "explicit");
        assert!(task.validate().is_ok());
        task.data_policy = None;
        assert!(task.validate().is_err());
        task = crate::NewTask::diagnostic("worker", "diagnostic");
        task.data_policy = Some(DataPolicy::PublicCoding);
        assert!(task.validate().is_err());
    }
    #[test]
    fn token_math_and_limits_are_bounded() {
        assert!(ModelBudgetLimits::default().validates());
        assert!(
            !ModelUsage {
                input_tokens: u64::MAX,
                output_tokens: 1,
                cost_microusd: None
            }
            .validates()
        );
        assert!(
            !ModelBudgetLimits {
                per_request_tokens: 0,
                ..ModelBudgetLimits::default()
            }
            .validates()
        );
    }
}
