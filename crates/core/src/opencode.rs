//! Approved generation work orders and metadata-only installed-executor evidence.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenCodeWorkOrder {
    pub operation_id: String,
    pub task_id: String,
    pub target_worker: String,
    pub work: OpenCodeWork,
    pub owner_request: String,
    pub deadline_at: i64,
    pub allowed_operations: Vec<String>,
    pub max_model_calls: u32,
    pub max_tool_calls: u32,
    pub max_output_bytes: usize,
    pub runtime_seconds: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum OpenCodeWork {
    Generate {
        channel: String,
        count: u32,
    },
    Interpret {
        context: Vec<crate::ConversationTurn>,
        planner_worker: String,
    },
}
impl OpenCodeWorkOrder {
    pub fn generation(&self) -> Option<(&str, u32)> {
        match &self.work {
            OpenCodeWork::Generate { channel, count } => Some((channel, *count)),
            _ => None,
        }
    }
    pub fn interpret(&self) -> bool {
        matches!(self.work, OpenCodeWork::Interpret { .. })
    }

    pub fn validates(&self, now: i64) -> bool {
        super::service::canonical_operation_id(&self.operation_id)
            && super::service::canonical_operation_id(&self.task_id)
            && super::validate_identifier(&self.target_worker).is_ok()
            && match &self.work {
                OpenCodeWork::Generate { channel, count } => {
                    super::canonical_youtube_channel(channel) == Some(channel.as_str())
                        && (1..=10).contains(count)
                        && self.allowed_operations
                            == ["generate", "status", "inspect_artifacts", "cancel_owned"]
                }
                OpenCodeWork::Interpret {
                    context,
                    planner_worker,
                } => {
                    super::validate_identifier(planner_worker).is_ok()
                        && context.len() <= 4
                        && context.iter().all(|t| t.validates())
                        && context.iter().map(|t| t.text.len()).sum::<usize>() <= 1024
                        && self.allowed_operations == ["select_registered_action"]
                }
            }
            && super::ordinary_text(&self.owner_request, 2048)
            && self.deadline_at > now
            && self.deadline_at <= now.saturating_add(180_000)
            && self.max_model_calls == 1
            && self.max_tool_calls == 1
            && self.max_output_bytes == 65536
            && (1..=150).contains(&self.runtime_seconds)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenCodeEvidence {
    pub operation_id: String,
    pub session_id: String,
    pub executable_sha256: String,
    pub executable_version: String,
    pub process_id: u32,
    pub process_start: String,
    pub unit: String,
    pub target_worker: String,
    pub channel: String,
    pub count: u32,
    pub mode: String,
    pub invoked_tool: String,
    pub model: String,
    pub model_calls: u32,
    pub billing_source: String,
    pub usage: super::ModelUsage,
}
impl OpenCodeEvidence {
    pub fn validates(&self, operation: &str, channel: &str, count: u32) -> bool {
        self.operation_id == operation
            && super::service::canonical_operation_id(operation)
            && self.session_id.starts_with("ses_")
            && self.session_id.len() <= 64
            && self
                .session_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            && self.executable_sha256.len() == 64
            && self
                .executable_sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && self.executable_version == "2.3.0"
            && self.process_id > 1
            && !self.process_start.is_empty()
            && self.process_start.len() <= 32
            && self.process_start.bytes().all(|b| b.is_ascii_digit())
            && self.unit == format!("zaturo-opencode-{operation}.service")
            && super::validate_identifier(&self.target_worker).is_ok()
            && self.channel == channel
            && self.count == count
            && self.mode == "generate"
            && self.invoked_tool == "youtube_generate"
            && self.model == "deepseek-v4.1-flash"
            && self.model_calls == 1
            && matches!(self.billing_source.as_str(), "go" | "go-plus")
            && self.usage.validates()
            && self.usage.cost_microusd == Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn work_orders_cannot_add_operations_calls_or_an_unknown_channel() {
        let mut order = OpenCodeWorkOrder {
            operation_id: "00000000-0000-4000-8000-000000000001".into(),
            task_id: "00000000-0000-4000-8000-000000000002".into(),
            target_worker: "work-pc".into(),
            work: OpenCodeWork::Generate {
                channel: "fallem-prince".into(),
                count: 2,
            },
            owner_request: "faça 2 videos para o falllem price".into(),
            deadline_at: 120000,
            allowed_operations: ["generate", "status", "inspect_artifacts", "cancel_owned"]
                .map(str::to_owned)
                .to_vec(),
            max_model_calls: 1,
            max_tool_calls: 1,
            max_output_bytes: 65536,
            runtime_seconds: 120,
        };
        assert!(order.validates(1000));
        order.allowed_operations.push("shell".into());
        assert!(!order.validates(1000));
        order.allowed_operations.pop();
        order.max_model_calls = 2;
        assert!(!order.validates(1000));
        order.max_model_calls = 1;
        order.work = OpenCodeWork::Generate {
            channel: "unregistered".into(),
            count: 2,
        };
        assert!(!order.validates(1000));
    }
}
