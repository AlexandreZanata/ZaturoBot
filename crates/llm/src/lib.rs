//! Bounded model routing and provider transport; no planner, SQL, or tool execution.
mod cancellation;
pub mod config;
mod contract;
pub mod fake;
mod opencode;
mod router;

pub use cancellation::Cancellation;
pub use config::{DEEPSEEK_MODEL, ModelsConfig};
pub use contract::{
    BudgetPort, CallContext, Error, FutureResult, LlmClient, Message, MessageRole, ModelRequest,
    ModelResponse, PrivacyClass,
};
pub use opencode::{GO_CHAT_ENDPOINT, OpenCodeGoClient};
pub use router::{Clock, Router, SystemClock};
