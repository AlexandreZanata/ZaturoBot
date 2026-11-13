//! Outbound worker transport and durable, policy-checked diagnostic execution.
pub mod config;
mod runtime;
pub use runtime::{connect_once, run};

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
