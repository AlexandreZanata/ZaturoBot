//! Durable non-sensitive owner activity. Sensitive screens never enter this contract.
use chrono::{FixedOffset, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedKind {
    Chat,
    Task,
    File,
    Service,
    Android,
}
impl FeedKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chat => "chat",
            Self::Task => "task",
            Self::File => "file",
            Self::Service => "service",
            Self::Android => "android",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedItem {
    pub id: String,
    pub created_at: i64,
    pub kind: FeedKind,
    pub role: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    pub sensitive: bool,
}
impl FeedItem {
    pub fn validates(&self) -> bool {
        !self.sensitive
            && !self.id.is_empty()
            && self.id.len() <= 64
            && self.created_at >= 0
            && matches!(self.role.as_str(), "owner" | "assistant" | "system")
            && !self.text.trim().is_empty()
            && self.text.len() <= 4096
            && self.task_id.as_ref().is_none_or(|v| v.len() <= 64)
            && self.artifact_id.as_ref().is_none_or(|v| v.len() <= 64)
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedQuery {
    pub day: String,
    #[serde(default = "default_offset")]
    pub offset_minutes: i32,
    pub kind: Option<FeedKind>,
    pub q: Option<String>,
    #[serde(default = "page_size")]
    pub limit: u32,
    #[serde(default)]
    pub offset: u32,
}
fn default_offset() -> i32 {
    -240
}
fn page_size() -> u32 {
    20
}
impl FeedQuery {
    pub fn range(&self) -> Result<(i64, i64), String> {
        if self.day.len() != 10
            || !(-720..=840).contains(&self.offset_minutes)
            || self.q.as_ref().is_some_and(|q| q.len() > 128)
        {
            return Err("invalid day, timezone offset or query".into());
        }
        let date =
            NaiveDate::parse_from_str(&self.day, "%Y-%m-%d").map_err(|_| "invalid calendar day")?;
        if date.format("%Y-%m-%d").to_string() != self.day {
            return Err("noncanonical calendar day".into());
        }
        let offset =
            FixedOffset::east_opt(self.offset_minutes * 60).ok_or("invalid timezone offset")?;
        let start = offset
            .from_local_datetime(&date.and_hms_opt(0, 0, 0).ok_or("invalid day")?)
            .single()
            .ok_or("invalid day")?
            .timestamp_millis();
        let end = start.checked_add(86_400_000).ok_or("date overflow")?;
        Ok((start, end))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedPage {
    pub items: Vec<FeedItem>,
    pub has_more: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatAction {
    Diagnostic,
    YoutubeProfiles,
    AndroidDevices,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatRequest {
    pub message: String,
    pub idempotency_key: String,
    #[serde(default)]
    pub target_worker: Option<String>,
    #[serde(default)]
    pub action: Option<ChatAction>,
}
impl ChatRequest {
    pub fn validates(&self) -> bool {
        !self.message.trim().is_empty()
            && self.message.len() <= 4096
            && crate::validate_identifier(&self.idempotency_key).is_ok()
            && self
                .target_worker
                .as_ref()
                .is_none_or(|s| crate::validate_identifier(s).is_ok())
            && !["oc_sk_", "-----BEGIN ", "AGE-SECRET-"]
                .iter()
                .any(|s| self.message.contains(s))
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatReply {
    pub message: FeedItem,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<crate::Task>,
}
