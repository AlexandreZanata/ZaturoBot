//! Outbound worker transport and durable, policy-checked diagnostic execution.
pub mod config;
mod execution;
pub mod files;
mod private_views;
mod runtime;
mod service_execution;
mod service_jobs;
pub use runtime::{connect_once, connect_once_with_router, run};

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(i64::MAX)
}
