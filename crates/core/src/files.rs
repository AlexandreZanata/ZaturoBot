//! Device-scoped file delivery contracts. Payload bytes are never model instructions.
use serde::{Deserialize, Serialize};

pub const MAX_CHUNK_BYTES: usize = 4 * 1024 * 1024;
pub const DEFAULT_CHUNK_BYTES: usize = MAX_CHUNK_BYTES;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TransferLimits {
    pub max_file_bytes: u64,
    pub max_spool_bytes: u64,
    pub max_pending: u32,
    pub ttl_seconds: u64,
}
impl Default for TransferLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 2 * 1024 * 1024 * 1024,
            max_spool_bytes: 8 * 1024 * 1024 * 1024,
            max_pending: 128,
            ttl_seconds: 86400,
        }
    }
}
impl TransferLimits {
    pub fn validates(&self) -> bool {
        self.max_file_bytes > 0
            && self.max_file_bytes <= 4 * 1024 * 1024 * 1024
            && self.max_spool_bytes >= self.max_file_bytes
            && self.max_spool_bytes <= 16 * 1024 * 1024 * 1024
            && (1..=512).contains(&self.max_pending)
            && (60..=604800).contains(&self.ttl_seconds)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CreateTransfer {
    pub recipient: String,
    pub filename: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub idempotency_key: String,
}
impl CreateTransfer {
    pub fn validate(&self, limits: &TransferLimits) -> bool {
        crate::validate_identifier(&self.recipient).is_ok()
            && safe_filename(&self.filename)
            && self.size_bytes > 0
            && self.size_bytes <= limits.max_file_bytes
            && valid_sha256(&self.sha256)
            && crate::validate_identifier(&self.idempotency_key).is_ok()
    }
}
pub fn safe_filename(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 240
        && !matches!(name, "." | "..")
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
}
pub fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferState {
    Uploading,
    Ready,
    Delivered,
    Expired,
}
impl TransferState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Uploading => "uploading",
            Self::Ready => "ready",
            Self::Delivered => "delivered",
            Self::Expired => "expired",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileTransfer {
    pub id: String,
    pub sender: String,
    pub recipient: String,
    pub filename: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub uploaded_bytes: u64,
    pub state: TransferState,
    pub created_at: i64,
    pub updated_at: i64,
    pub expires_at: i64,
}
impl FileTransfer {
    pub fn visible_to(&self, device: &str) -> bool {
        self.sender == device || self.recipient == device
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_names_and_quotas_cannot_escape_or_be_unbounded() {
        for name in ["../secret", "a/b", "a\\b", "..", ".", "a\nfile", ""] {
            assert!(!safe_filename(name));
        }
        assert!(safe_filename("Relatório de trabalho.pdf"));
        assert!(TransferLimits::default().validates());
        assert!(
            !TransferLimits {
                max_pending: 0,
                ..TransferLimits::default()
            }
            .validates()
        );
        assert!(!valid_sha256(&"Z".repeat(64)));
    }
}
