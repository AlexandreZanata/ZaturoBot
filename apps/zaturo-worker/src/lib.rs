//! Outbound worker transport and durable, policy-checked diagnostic execution.
mod android_actions;
mod assistant;
mod computer;
pub mod config;
mod desktop_private;
mod execution;
pub mod files;
pub mod mercadolivre;
mod private_views;
mod runtime;
mod service_execution;
mod service_jobs;
pub mod telegram;
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

mod opencode;
