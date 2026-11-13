use serde::{Deserialize, Serialize};
use std::{fmt, future::Future, pin::Pin};
use zaturo_core::{ModelReservation, ModelUsage};

pub type FutureResult<'a, T> = Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("model routing is disabled")]
    Disabled,
    #[error("invalid model configuration: {0}")]
    InvalidConfig(&'static str),
    #[error("model policy denied: {0}")]
    Denied(&'static str),
    #[error("invalid bounded model request")]
    InvalidRequest,
    #[error("operation cancelled")]
    Cancelled,
    #[error("operation deadline reached")]
    Deadline,
    #[error("provider request timed out; usage is uncertain")]
    Timeout,
    #[error("provider transport failed; usage is uncertain")]
    Transport,
    #[error("provider authentication failed")]
    Authentication,
    #[error("provider rate limited the request")]
    RateLimited { retry_after_seconds: Option<u64> },
    #[error("provider server failure ({0})")]
    Server(u16),
    #[error("provider rejected request ({0})")]
    Rejected(u16),
    #[error("provider response violates the bounded contract")]
    InvalidResponse,
    #[error("provider response exceeded its byte limit")]
    ResponseTooLarge,
    #[error("durable model budget denied the request")]
    Budget,
    #[error("previous model request requires reconciliation")]
    Reconciliation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyClass {
    PublicCoding,
    PrivateCoding,
    Personal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallContext {
    pub task_id: String,
    pub operation_id: String,
    pub step: u32,
    pub deadline_at: i64,
    pub privacy: PrivacyClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}
impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}
impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Message")
            .field("role", &self.role)
            .field("content", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub model: String,
    pub context: CallContext,
    pub request_id: String,
    pub messages: Vec<Message>,
    pub max_output_tokens: u32,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ModelResponse {
    pub model: String,
    pub content: String,
    pub usage: ModelUsage,
    pub finish_reason: String,
    pub duration_ms: u64,
    pub model_calls: u32,
    pub usage_complete: bool,
}
impl fmt::Debug for ModelResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModelResponse")
            .field("model", &self.model)
            .field("content", &"[REDACTED]")
            .field("usage", &self.usage)
            .field("finish_reason", &self.finish_reason)
            .finish()
    }
}

pub trait LlmClient: Send + Sync {
    fn complete(&self, request: ModelRequest) -> FutureResult<'_, ModelResponse>;
}

/// A durable implementation must admit only a newly-created reservation. Existing
/// reserved, settled, uncertain, or overrun records cannot authorize another send.
pub trait BudgetPort: Send + Sync {
    fn reserve<'a>(&'a self, reservation: &'a ModelReservation) -> FutureResult<'a, ()>;
    fn settle<'a>(&'a self, request_id: &'a str, usage: &'a ModelUsage) -> FutureResult<'a, ()>;
    fn uncertain<'a>(&'a self, request_id: &'a str) -> FutureResult<'a, ()>;
}
