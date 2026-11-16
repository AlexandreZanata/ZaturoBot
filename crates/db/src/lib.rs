//! SQLite repositories. State-machine decisions belong to services.

mod activity;
mod computer;
mod mercadolivre;
mod service_journal;
mod shared;
pub use service_journal::{PendingServiceJob, ServiceIntent};
mod assistant_budget;
mod journal;
mod model_budget;
mod repository;
mod transfers;

pub use journal::WorkerJournal;
pub use repository::{HubStore, Repository};

use rusqlite::Connection;
use std::{path::Path, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database operation failed: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("stored record is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("database lock is poisoned")]
    Poisoned,
    #[error("database schema version is unsupported")]
    SchemaVersion,
    #[error("operation belongs to another task or has a conflicting result")]
    OperationConflict,
    #[error("model reservation or usage exceeds a configured budget")]
    BudgetExceeded,
}

fn connection(
    path: impl AsRef<Path>,
    expected_version: i64,
    migration: &str,
) -> Result<Connection, Error> {
    let connection = Connection::open(path)?;
    connection.busy_timeout(Duration::from_secs(2))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    connection.pragma_update(None, "cache_size", -2048)?;
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version == 0 {
        connection.execute_batch(&format!("BEGIN IMMEDIATE;\n{migration}\nCOMMIT;"))?;
    } else if expected_version == 10 && matches!(version, 1 | 4 | 5 | 7 | 9) {
        let base = if version == 1 {
            include_str!("../migrations/0004_hub_transfers.sql")
        } else {
            ""
        };
        let activity = if matches!(version, 1 | 4) {
            include_str!("../migrations/0005_hub_activity.sql")
        } else {
            ""
        };
        let shared = if matches!(version, 1 | 4 | 5) {
            include_str!("../migrations/0007_hub_shared.sql")
        } else {
            ""
        };
        let computer = if version != 9 {
            include_str!("../migrations/0009_hub_computer.sql")
        } else {
            ""
        };
        connection.execute_batch(&format!(
            "BEGIN IMMEDIATE;\n{base}\n{activity}\n{shared}\n{computer}\n{}\nCOMMIT;",
            include_str!("../migrations/0010_hub_mercadolivre_commerce.sql")
        ))?;
    } else if expected_version == 8 && matches!(version, 2 | 3 | 6) {
        let base = if version == 2 {
            include_str!("../migrations/0003_worker_model_budget.sql")
        } else {
            ""
        };
        connection.execute_batch(&format!(
            "BEGIN IMMEDIATE;\n{base}\n{}\n{}\nCOMMIT;",
            if version == 6 {
                ""
            } else {
                include_str!("../migrations/0006_worker_services.sql")
            },
            include_str!("../migrations/0008_worker_assistant.sql")
        ))?;
    } else if version != expected_version {
        return Err(Error::SchemaVersion);
    }
    Ok(connection)
}
