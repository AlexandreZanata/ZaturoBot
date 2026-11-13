//! SQLite repositories. State-machine decisions belong to services.

mod journal;
mod repository;

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
    } else if version != expected_version {
        return Err(Error::SchemaVersion);
    }
    Ok(connection)
}
