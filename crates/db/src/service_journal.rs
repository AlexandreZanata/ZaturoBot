//! Durable intent/result fencing for fixed service adapters.
use crate::{Error, WorkerJournal};
use rusqlite::{OptionalExtension, params};
use zaturo_core::{ServiceRequest, ServiceResult};
#[derive(Debug, PartialEq, Eq)]
pub struct PendingServiceJob {
    pub operation_id: String,
    pub request: ServiceRequest,
    pub result: ServiceResult,
    pub last_notified_state: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ServiceIntent {
    Execute,
    Cached(ServiceResult),
    Reconcile,
}
impl WorkerJournal {
    pub fn begin_service(
        &self,
        operation: &str,
        task: &str,
        request: &ServiceRequest,
        now: i64,
    ) -> Result<ServiceIntent, Error> {
        let mut c = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let row:Option<(String,String,Option<String>)>=tx.query_row("SELECT task_id,request_json,result_json FROM service_operations WHERE operation_id=?1",[operation],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let result = match row {
            Some((owner, stored, result)) => {
                if owner != task || serde_json::from_str::<ServiceRequest>(&stored)? != *request {
                    return Err(Error::OperationConflict);
                }
                match result {
                    Some(value) => ServiceIntent::Cached(serde_json::from_str(&value)?),
                    None if request.side_effecting() => ServiceIntent::Reconcile,
                    None => ServiceIntent::Execute,
                }
            }
            None => {
                tx.execute("INSERT INTO service_operations(operation_id,task_id,request_json,started_at,updated_at) VALUES (?1,?2,?3,?4,?4)",params![operation,task,serde_json::to_string(request)?,now])?;
                ServiceIntent::Execute
            }
        };
        tx.commit()?;
        Ok(result)
    }
    pub fn record_service(
        &self,
        operation: &str,
        result: &ServiceResult,
        now: i64,
    ) -> Result<(), Error> {
        let mut c = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let row: Option<(String, Option<String>)> = tx
            .query_row(
                "SELECT request_json,result_json FROM service_operations WHERE operation_id=?1",
                [operation],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((request, previous)) = row else {
            return Err(Error::OperationConflict);
        };
        let request: ServiceRequest = serde_json::from_str(&request)?;
        if !result.validates(operation, &request)
            || previous.as_ref().is_some_and(|v| {
                serde_json::from_str::<ServiceResult>(v).map_or(true, |r| r != *result)
            })
        {
            return Err(Error::OperationConflict);
        }
        tx.execute(
            "UPDATE service_operations SET result_json=?2,updated_at=?3 WHERE operation_id=?1",
            params![operation, serde_json::to_string(result)?, now],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn pending_service_jobs(&self, limit: u32) -> Result<Vec<PendingServiceJob>, Error> {
        let c = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let mut s=c.prepare("SELECT operation_id,request_json,result_json,last_notified_state FROM service_operations WHERE json_extract(request_json,'$.action')='youtube_submit' AND json_extract(result_json,'$.outcome')='accepted' AND (last_notified_state IS NULL OR last_notified_state NOT IN ('completed','failed','uncertain')) ORDER BY last_polled_at,operation_id LIMIT ?1")?;
        let rows = s.query_map([limit.min(20)], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })?;
        let mut jobs = Vec::new();
        for row in rows {
            let (id, request, result, state) = row?;
            let result: ServiceResult = serde_json::from_str(&result)?;
            if result.outcome == "accepted" {
                jobs.push(PendingServiceJob {
                    operation_id: id,
                    request: serde_json::from_str(&request)?,
                    result,
                    last_notified_state: state,
                });
            }
        }
        Ok(jobs)
    }
    /// Rotate an observation attempt before network/process work, including unchanged/error states.
    pub fn service_polled(&self, operation: &str, now: i64) -> Result<(), Error> {
        if !(0..i64::MAX).contains(&now) {
            return Err(Error::OperationConflict);
        }
        let c = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let changed = c.execute("UPDATE service_operations SET last_polled_at=MAX(last_polled_at+1,?2) WHERE operation_id=?1 AND json_extract(request_json,'$.action')='youtube_submit' AND json_extract(result_json,'$.outcome')='accepted' AND (last_notified_state IS NULL OR last_notified_state NOT IN ('completed','failed','uncertain'))",params![operation,now])?;
        if changed != 1 {
            return Err(Error::OperationConflict);
        }
        Ok(())
    }
    pub fn service_notified(&self, operation: &str, state: &str, now: i64) -> Result<(), Error> {
        if !matches!(
            state,
            "accepted" | "running" | "completed" | "failed" | "uncertain"
        ) {
            return Err(Error::OperationConflict);
        }
        let c = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let changed = c.execute("UPDATE service_operations SET last_notified_state=?2,updated_at=?3 WHERE operation_id=?1 AND json_extract(request_json,'$.action')='youtube_submit' AND json_extract(result_json,'$.outcome')='accepted' AND (last_notified_state IS NULL OR last_notified_state NOT IN ('completed','failed','uncertain') OR last_notified_state=?2)",params![operation,state,now])?;
        if changed != 1 {
            return Err(Error::OperationConflict);
        }
        Ok(())
    }
}
