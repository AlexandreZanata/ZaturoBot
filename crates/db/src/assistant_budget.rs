//! Separate personal-assistant reservations with the same global token admission as coding.
use crate::{Error, WorkerJournal};
use rusqlite::{OptionalExtension, params};
use zaturo_core::{
    ModelBudgetLimits, ModelBudgetSnapshot, ModelCallState, ModelReservation, ModelUsage,
    ReservationStatus, ServiceRequest,
};
impl WorkerJournal {
    pub fn reserve_assistant_call(
        &self,
        reservation: &ModelReservation,
        limits: &ModelBudgetLimits,
        now: i64,
    ) -> Result<ReservationStatus, Error> {
        if !reservation.validates() || !limits.validates() || now < 0 {
            return Err(Error::OperationConflict);
        }
        let mut connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        if let Some((body, state)) = tx
            .query_row(
                "SELECT reservation_json,state FROM assistant_calls WHERE request_id=?1",
                [&reservation.request_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?
        {
            if serde_json::from_str::<ModelReservation>(&body)? != *reservation {
                return Err(Error::OperationConflict);
            }
            return Ok(ReservationStatus::Existing(match state.as_str() {
                "settled" => ModelCallState::Settled,
                "uncertain" => ModelCallState::Uncertain,
                "overrun" => ModelCallState::Overrun,
                _ => ModelCallState::Reserved,
            }));
        }
        let operation:Option<(String,String,Option<String>)>=tx.query_row("SELECT task_id,request_json,result_json FROM service_operations WHERE operation_id=?1",[&reservation.operation_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let Some((task, request, None)) = operation else {
            return Err(Error::OperationConflict);
        };
        if task != reservation.task_id
            || !matches!(
                serde_json::from_str::<ServiceRequest>(&request)?,
                ServiceRequest::AssistantChat { .. }
            )
        {
            return Err(Error::OperationConflict);
        }
        super::model_budget::enforce_clock(&tx, now)?;
        let (day, month) = super::model_budget::period_starts(&tx, now)?;
        let (task_tokens,calls):(u64,u32)=tx.query_row("SELECT COALESCE(SUM(charged_tokens),0),COUNT(*) FROM assistant_calls WHERE operation_id=?1",[&reservation.operation_id],|r|Ok((r.get(0)?,r.get(1)?)))?;
        let total = |start: i64| -> Result<u64, Error> {
            Ok(tx.query_row("SELECT COALESCE(SUM(charged_tokens),0) FROM (SELECT charged_tokens,created_at FROM model_calls UNION ALL SELECT charged_tokens,created_at FROM assistant_calls) WHERE created_at>=?1",[start],|r|r.get(0))?)
        };
        let overrun:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM assistant_calls WHERE operation_id=?1 AND state='overrun')",[&reservation.operation_id],|r|r.get(0))?;
        let snapshot = ModelBudgetSnapshot {
            task_tokens,
            task_model_calls: calls,
            day_tokens: total(day)?,
            month_tokens: total(month)?,
            operation_has_overrun: overrun,
        };
        if !limits.permits(reservation, &snapshot) {
            return Err(Error::BudgetExceeded);
        }
        tx.execute("INSERT INTO assistant_calls(request_id,operation_id,task_id,reservation_json,state,charged_tokens,created_at,updated_at) VALUES(?1,?2,?3,?4,'reserved',?5,?6,?6)",params![reservation.request_id,reservation.operation_id,reservation.task_id,serde_json::to_string(reservation)?,reservation.total_tokens().ok_or(Error::BudgetExceeded)?,now])?;
        tx.commit()?;
        Ok(ReservationStatus::Granted)
    }
    pub fn settle_assistant_call(
        &self,
        id: &str,
        usage: &ModelUsage,
        now: i64,
    ) -> Result<(), Error> {
        if !usage.validates() || now < 0 {
            return Err(Error::OperationConflict);
        }
        let mut connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let row: Option<(String, String, Option<String>)> = tx
            .query_row(
                "SELECT reservation_json,state,usage_json FROM assistant_calls WHERE request_id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let Some((reservation, state, previous)) = row else {
            return Err(Error::OperationConflict);
        };
        if matches!(state.as_str(), "settled" | "overrun") {
            if previous
                .map(|p| serde_json::from_str::<ModelUsage>(&p))
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
        super::model_budget::enforce_clock(&tx, now)?;
        let reservation: ModelReservation = serde_json::from_str(&reservation)?;
        let overrun = !reservation.accepts_usage(usage);
        tx.execute("UPDATE assistant_calls SET state=?2,charged_tokens=?3,usage_json=?4,updated_at=?5 WHERE request_id=?1",params![id,if overrun{"overrun"}else{"settled"},usage.total_tokens().ok_or(Error::BudgetExceeded)?,serde_json::to_string(usage)?,now])?;
        tx.commit()?;
        if overrun {
            Err(Error::BudgetExceeded)
        } else {
            Ok(())
        }
    }
    pub fn uncertain_assistant_call(&self, id: &str, now: i64) -> Result<(), Error> {
        let mut connection = self.connection.lock().map_err(|_| Error::Poisoned)?;
        let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        super::model_budget::enforce_clock(&tx, now)?;
        let changed=tx.execute("UPDATE assistant_calls SET state='uncertain',updated_at=?2 WHERE request_id=?1 AND state IN ('reserved','uncertain')",params![id,now])?;
        if changed != 1 {
            return Err(Error::OperationConflict);
        }
        tx.commit()?;
        Ok(())
    }
}
