//! Atomic worker-local reservations. Uncertain calls retain their maximum charge.
use crate::{Error, WorkerJournal};
use rusqlite::{Connection, OptionalExtension, params};
use zaturo_core::{
    ModelBudgetLimits, ModelBudgetSnapshot, ModelCallState, ModelReservation, ModelUsage,
    ReservationStatus,
};

const DAY_MS: i64 = 86_400_000;

impl WorkerJournal {
    pub fn reserve_model_call(
        &self,
        reservation: &ModelReservation,
        limits: &ModelBudgetLimits,
        now: i64,
    ) -> Result<ReservationStatus, Error> {
        if !reservation.validates() || !limits.validates() || now < 0 {
            return Err(Error::OperationConflict);
        }
        let reserved = reservation.total_tokens().ok_or(Error::BudgetExceeded)?;
        let mut connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let previous: Option<(String, String)> = transaction
            .query_row(
                "SELECT reservation_json,state FROM model_calls WHERE request_id=?1",
                [&reservation.request_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((body, state)) = previous {
            if serde_json::from_str::<ModelReservation>(&body)? != *reservation {
                return Err(Error::OperationConflict);
            }
            return Ok(ReservationStatus::Existing(parse_state(&state)?));
        }
        enforce_clock(&transaction, now)?;
        let operation: Option<(String, String, Option<String>)> = transaction
            .query_row(
                "SELECT task_id,kind,assisted_json FROM operations WHERE operation_id=?1",
                [&reservation.operation_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if !matches!(operation,Some((ref task,ref kind,None)) if task==&reservation.task_id && kind=="assisted_diagnostic")
        {
            return Err(Error::OperationConflict);
        }
        let overrun: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM model_calls WHERE operation_id=?1 AND state='overrun')",
            [&reservation.operation_id],
            |r| r.get(0),
        )?;
        let (day_start, month_start) = period_starts(&transaction, now)?;
        let (task_charge,task_calls):(u64,u32)=transaction.query_row("SELECT COALESCE(SUM(charged_tokens),0),COUNT(*) FROM model_calls WHERE operation_id=?1",[&reservation.operation_id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        let day_charge: u64 = transaction.query_row(
            "SELECT COALESCE(SUM(charged_tokens),0) FROM model_calls WHERE created_at>=?1",
            [day_start],
            |r| r.get(0),
        )?;
        let month_charge: u64 = transaction.query_row(
            "SELECT COALESCE(SUM(charged_tokens),0) FROM model_calls WHERE created_at>=?1",
            [month_start],
            |r| r.get(0),
        )?;
        let snapshot = ModelBudgetSnapshot {
            task_tokens: task_charge,
            day_tokens: day_charge,
            month_tokens: month_charge,
            task_model_calls: task_calls,
            operation_has_overrun: overrun,
        };
        if !limits.permits(reservation, &snapshot) {
            return Err(Error::BudgetExceeded);
        }
        transaction.execute("INSERT INTO model_calls(request_id,operation_id,task_id,reservation_json,state,reserved_tokens,charged_tokens,created_at,updated_at) VALUES (?1,?2,?3,?4,'reserved',?5,?5,?6,?6)",params![reservation.request_id,reservation.operation_id,reservation.task_id,serde_json::to_string(reservation)?,reserved,now])?;
        transaction.commit()?;
        Ok(ReservationStatus::Granted)
    }

    /// Actual usage can increase the charge; an overrun persists before returning denial.
    pub fn settle_model_call(
        &self,
        request_id: &str,
        usage: &ModelUsage,
        now: i64,
    ) -> Result<(), Error> {
        if !usage.validates() || now < 0 {
            return Err(Error::OperationConflict);
        }
        let actual = usage.total_tokens().ok_or(Error::BudgetExceeded)?;
        let mut connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let record: Option<(String, Option<String>)> = transaction
            .query_row(
                "SELECT state,usage_json FROM model_calls WHERE request_id=?1",
                [request_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let Some((state, body)) = record else {
            return Err(Error::OperationConflict);
        };
        if matches!(state.as_str(), "settled" | "overrun") {
            if body
                .map(|s| serde_json::from_str::<ModelUsage>(&s))
                .transpose()?
                .as_ref()
                != Some(usage)
            {
                return Err(Error::OperationConflict);
            }
            return if state == "overrun" {
                Err(Error::BudgetExceeded)
            } else {
                Ok(())
            };
        }
        enforce_clock(&transaction, now)?;
        let reservation: ModelReservation =
            serde_json::from_str(&transaction.query_row::<String, _, _>(
                "SELECT reservation_json FROM model_calls WHERE request_id=?1",
                [request_id],
                |r| r.get(0),
            )?)?;
        let overrun = !reservation.accepts_usage(usage);
        transaction.execute("UPDATE model_calls SET state=?2,charged_tokens=?3,usage_json=?4,updated_at=?5 WHERE request_id=?1",params![request_id,if overrun{"overrun"}else{"settled"},actual,serde_json::to_string(usage)?,now])?;
        transaction.commit()?;
        if overrun {
            Err(Error::BudgetExceeded)
        } else {
            Ok(())
        }
    }

    pub fn mark_model_call_uncertain(&self, request_id: &str, now: i64) -> Result<(), Error> {
        if now < 0 {
            return Err(Error::OperationConflict);
        }
        let mut connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        enforce_clock(&transaction, now)?;
        let state: Option<String> = transaction
            .query_row(
                "SELECT state FROM model_calls WHERE request_id=?1",
                [request_id],
                |r| r.get(0),
            )
            .optional()?;
        match state.as_deref() {
            Some("reserved") => {
                transaction.execute(
                    "UPDATE model_calls SET state='uncertain',updated_at=?2 WHERE request_id=?1",
                    params![request_id, now],
                )?;
            }
            Some("uncertain") => {}
            _ => return Err(Error::OperationConflict),
        }
        transaction.commit()?;
        Ok(())
    }

    /// Cancellation or transport loss preserves every pending call's full charge.
    pub fn interrupt_model_operation(&self, operation_id: &str, now: i64) -> Result<(), Error> {
        if now < 0 {
            return Err(Error::OperationConflict);
        }
        let mut connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        enforce_clock(&transaction, now)?;
        transaction.execute("UPDATE model_calls SET state='uncertain',updated_at=?2 WHERE operation_id=?1 AND state='reserved'",params![operation_id,now])?;
        transaction.commit()?;
        Ok(())
    }

    pub fn model_call_usage(&self, request_id: &str) -> Result<Option<ModelUsage>, Error> {
        let connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let body: Option<Option<String>> = connection
            .query_row(
                "SELECT usage_json FROM model_calls WHERE request_id=?1",
                [request_id],
                |r| r.get(0),
            )
            .optional()?;
        body.flatten()
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(Error::from)
    }

    pub fn model_call_state(&self, request_id: &str) -> Result<Option<ModelCallState>, Error> {
        let connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let state: Option<String> = connection
            .query_row(
                "SELECT state FROM model_calls WHERE request_id=?1",
                [request_id],
                |r| r.get(0),
            )
            .optional()?;
        state.map(|s| parse_state(&s)).transpose()
    }
}

fn parse_state(state: &str) -> Result<ModelCallState, Error> {
    match state {
        "reserved" => Ok(ModelCallState::Reserved),
        "settled" => Ok(ModelCallState::Settled),
        "uncertain" => Ok(ModelCallState::Uncertain),
        "overrun" => Ok(ModelCallState::Overrun),
        _ => Err(Error::OperationConflict),
    }
}
fn enforce_clock(connection: &Connection, now: i64) -> Result<(), Error> {
    let last: Option<i64> = connection
        .query_row(
            "SELECT updated_at FROM model_calls ORDER BY updated_at DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?;
    if last.is_some_and(|last| now < last) {
        return Err(Error::OperationConflict);
    }
    Ok(())
}
fn period_starts(connection: &Connection, now: i64) -> Result<(i64, i64), Error> {
    let month: Option<i64> = connection.query_row(
        "SELECT CAST(strftime('%s',?1/1000,'unixepoch','start of month') AS INTEGER)*1000",
        [now],
        |r| r.get(0),
    )?;
    Ok((
        now - now.rem_euclid(DAY_MS),
        month.ok_or(Error::OperationConflict)?,
    ))
}
