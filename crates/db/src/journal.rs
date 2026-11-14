use crate::{Error, connection};
use rusqlite::{Connection, OptionalExtension, params};
use std::{path::Path, sync::Mutex};
use zaturo_core::{AssistedResult, DiagnosticResult};

/// Local diagnostic intents and results; never synchronize the underlying file.
pub struct WorkerJournal {
    pub(crate) connection: Mutex<Connection>,
}

impl WorkerJournal {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Self {
            connection: Mutex::new(connection(
                path,
                6,
                concat!(
                    include_str!("../migrations/0002_worker.sql"),
                    "\n",
                    include_str!("../migrations/0003_worker_model_budget.sql"),
                    "\n",
                    include_str!("../migrations/0006_worker_services.sql")
                ),
            )?),
        })
    }

    /// Existing unresolved intents may be repeated only because this journal is diagnostic-only.
    pub fn begin(
        &self,
        operation_id: &str,
        task_id: &str,
        now: i64,
    ) -> Result<Option<DiagnosticResult>, Error> {
        let mut connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let existing: Option<(String, String, Option<String>)> = transaction
            .query_row(
                "SELECT task_id,kind,result_json FROM operations WHERE operation_id=?1",
                [operation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let result = match existing {
            Some((owner, kind, result)) => {
                if owner != task_id || kind != "diagnostic" {
                    return Err(Error::OperationConflict);
                }
                result.map(|body| serde_json::from_str(&body)).transpose()?
            }
            None => {
                transaction.execute("INSERT INTO operations(operation_id,task_id,started_at,updated_at) VALUES (?1,?2,?3,?3)",params![operation_id,task_id,now])?;
                None
            }
        };
        transaction.commit()?;
        Ok(result)
    }

    pub fn record_result(
        &self,
        operation_id: &str,
        result: &DiagnosticResult,
        now: i64,
    ) -> Result<(), Error> {
        if !result.validates(operation_id) {
            return Err(Error::OperationConflict);
        }
        let mut connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let existing: Option<Option<String>> = transaction
            .query_row(
                "SELECT result_json FROM operations WHERE operation_id=?1",
                [operation_id],
                |row| row.get(0),
            )
            .optional()?;
        match existing {
            None => return Err(Error::OperationConflict),
            Some(Some(body)) if serde_json::from_str::<DiagnosticResult>(&body)? != *result => {
                return Err(Error::OperationConflict);
            }
            _ => {}
        }
        transaction.execute(
            "UPDATE operations SET result_json=?2,updated_at=?3 WHERE operation_id=?1",
            params![operation_id, serde_json::to_string(result)?, now],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn result(&self, operation_id: &str) -> Result<Option<DiagnosticResult>, Error> {
        let connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let body: Option<Option<String>> = connection
            .query_row(
                "SELECT result_json FROM operations WHERE operation_id=?1",
                [operation_id],
                |row| row.get(0),
            )
            .optional()?;
        body.flatten()
            .map(|body| serde_json::from_str(&body))
            .transpose()
            .map_err(Error::from)
    }

    /// A final assisted result is replayable; pending model reservations are not new authorization.
    pub fn begin_assisted(
        &self,
        operation_id: &str,
        task_id: &str,
        now: i64,
    ) -> Result<Option<AssistedResult>, Error> {
        let mut connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let existing: Option<(String, String, Option<String>)> = transaction
            .query_row(
                "SELECT task_id,kind,assisted_json FROM operations WHERE operation_id=?1",
                [operation_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let cached = match existing {
            Some((owner, kind, result)) => {
                if owner != task_id || kind != "assisted_diagnostic" {
                    return Err(Error::OperationConflict);
                }
                result.map(|s| serde_json::from_str(&s)).transpose()?
            }
            None => {
                transaction.execute("INSERT INTO operations(operation_id,task_id,kind,started_at,updated_at) VALUES (?1,?2,'assisted_diagnostic',?3,?3)",params![operation_id,task_id,now])?;
                None
            }
        };
        transaction.commit()?;
        Ok(cached)
    }

    pub fn record_assisted(
        &self,
        operation_id: &str,
        result: &AssistedResult,
        now: i64,
    ) -> Result<(), Error> {
        if !result.validates(operation_id) || serde_json::to_vec(result)?.len() > 4096 {
            return Err(Error::OperationConflict);
        }
        let mut connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let existing:Option<(Option<String>,Option<String>)>=transaction.query_row("SELECT result_json,assisted_json FROM operations WHERE operation_id=?1 AND kind='assisted_diagnostic'",[operation_id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        let Some((Some(diagnostic), assisted)) = existing else {
            return Err(Error::OperationConflict);
        };
        if serde_json::from_str::<DiagnosticResult>(&diagnostic)? != result.diagnostic {
            return Err(Error::OperationConflict);
        }
        if let Some(body) = assisted
            && serde_json::from_str::<AssistedResult>(&body)? != *result
        {
            return Err(Error::OperationConflict);
        }
        transaction.execute(
            "UPDATE operations SET assisted_json=?2,updated_at=?3 WHERE operation_id=?1",
            params![operation_id, serde_json::to_string(result)?, now],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(operation: &str) -> DiagnosticResult {
        DiagnosticResult {
            operation_id: operation.into(),
            tool: "diagnostic.v1".into(),
            os: "linux".into(),
            arch: "x86_64".into(),
            success: true,
            summary: "All diagnostic checks passed".into(),
        }
    }

    #[test]
    fn interrupted_diagnostic_reuses_durable_result_after_restart() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("worker.db");
        {
            let journal = WorkerJournal::open(&path).unwrap();
            assert_eq!(journal.begin("operation", "task", 0).unwrap(), None);
        }
        let journal = WorkerJournal::open(&path).unwrap();
        assert_eq!(journal.begin("operation", "task", 1).unwrap(), None);
        journal
            .record_result("operation", &report("operation"), 2)
            .unwrap();
        drop(journal);
        let journal = WorkerJournal::open(&path).unwrap();
        assert_eq!(
            journal.begin("operation", "task", 3).unwrap(),
            Some(report("operation"))
        );
        assert!(journal.begin("operation", "another-task", 3).is_err());
        let mut conflict = report("operation");
        conflict.summary = "different result".into();
        assert!(journal.record_result("operation", &conflict, 4).is_err());
    }

    #[test]
    fn journal_never_accepts_result_without_durable_intent() {
        let journal = WorkerJournal::open(":memory:").unwrap();
        assert!(
            journal
                .record_result("operation", &report("operation"), 0)
                .is_err()
        );
    }
}
