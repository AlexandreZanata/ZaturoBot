//! Bounded coordination records. Reading an instruction never authorizes its execution.
use serde::{Deserialize, Serialize};

pub fn ordinary_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= max
        && !value.contains('\0')
        && !["oc_sk_", "-----BEGIN ", "AGE-SECRET-"]
            .iter()
            .any(|s| value.contains(s))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedState {
    Pending,
    Claimed,
    Blocked,
    Done,
}
impl SharedState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::Blocked => "blocked",
            Self::Done => "done",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedCreate {
    pub idempotency_key: String,
    pub target_worker: String,
    pub title: String,
    pub instructions: String,
    #[serde(default)]
    pub transfer_ids: Vec<String>,
}
impl SharedCreate {
    pub fn validates(&self) -> bool {
        crate::validate_identifier(&self.idempotency_key).is_ok()
            && crate::validate_identifier(&self.target_worker).is_ok()
            && ordinary_text(&self.title, 160)
            && ordinary_text(&self.instructions, 8192)
            && self.transfer_ids.len() <= 8
            && self
                .transfer_ids
                .iter()
                .all(|id| crate::service::canonical_operation_id(id))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedUpdate {
    pub idempotency_key: String,
    pub expected_revision: u32,
    pub state: SharedState,
    pub note: String,
}
impl SharedUpdate {
    pub fn validates(&self) -> bool {
        crate::validate_identifier(&self.idempotency_key).is_ok()
            && self.expected_revision > 0
            && ordinary_text(&self.note, 4096)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedEntry {
    pub id: String,
    pub target_worker: String,
    pub title: String,
    pub instructions: String,
    pub transfer_ids: Vec<String>,
    pub author: String,
    pub state: SharedState,
    pub revision: u32,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedNote {
    pub revision: u32,
    pub author: String,
    pub state: SharedState,
    pub note: String,
    pub created_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedDetail {
    pub task: SharedEntry,
    pub notes: Vec<SharedNote>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSummary {
    pub id: String,
    pub target_worker: String,
    pub title: String,
    pub state: SharedState,
    pub revision: u32,
    pub updated_at: i64,
}
impl From<SharedEntry> for SharedSummary {
    fn from(e: SharedEntry) -> Self {
        Self {
            id: e.id,
            target_worker: e.target_worker,
            title: e.title,
            state: e.state,
            revision: e.revision,
            updated_at: e.updated_at,
        }
    }
}
