use crate::{Error, connection};
use rusqlite::{Connection, OptionalExtension, params};
use std::{path::Path, sync::Mutex};
use zaturo_core::{NewTask, Task, TaskEvent, TaskState, Worker};

const TASK_PAGE_SQL: &str =
    "SELECT body FROM tasks ORDER BY created_at DESC, id DESC LIMIT ?1 OFFSET ?2";
const UNFINISHED_COUNT_SQL: &str = "SELECT count(*) FROM tasks WHERE state IN ('CREATED','QUEUED','ASSIGNED','RUNNING','WAITING_APPROVAL','PAUSED','RETRY_PENDING')";
const EXPIRED_SQL: &str = "SELECT body FROM tasks WHERE state IN ('CREATED','QUEUED','ASSIGNED','RUNNING','WAITING_APPROVAL','PAUSED','RETRY_PENDING') AND (deadline_at<=?1 OR (state IN ('ASSIGNED','RUNNING') AND lease_expires_at<=?1)) ORDER BY deadline_at,id LIMIT ?2";

pub struct HubStore {
    connection: Mutex<Connection>,
}

pub struct Repository<'a> {
    pub(crate) connection: &'a Connection,
}

impl HubStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Self {
            connection: Mutex::new(connection(
                path,
                4,
                concat!(
                    include_str!("../migrations/0001_hub.sql"),
                    "\n",
                    include_str!("../migrations/0004_hub_transfers.sql")
                ),
            )?),
        })
    }

    pub fn read<T, E>(
        &self,
        operation: impl FnOnce(&Repository<'_>) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<Error>,
    {
        let connection = self
            .connection
            .lock()
            .map_err(|_| E::from(Error::Poisoned))?;
        operation(&Repository {
            connection: &connection,
        })
    }

    pub fn transaction<T, E>(
        &self,
        operation: impl FnOnce(&Repository<'_>) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<Error>,
    {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| E::from(Error::Poisoned))?;
        let transaction = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(Error::from)
            .map_err(E::from)?;
        let output = operation(&Repository {
            connection: &transaction,
        })?;
        transaction.commit().map_err(Error::from).map_err(E::from)?;
        Ok(output)
    }
}

impl Repository<'_> {
    pub fn task(&self, id: &str) -> Result<Option<Task>, Error> {
        let body: Option<String> = self
            .connection
            .query_row("SELECT body FROM tasks WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .optional()?;
        body.map(|body| serde_json::from_str(&body))
            .transpose()
            .map_err(Error::from)
    }

    pub fn by_idempotency_key(&self, key: &str) -> Result<Option<(Task, NewTask)>, Error> {
        let record: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT body, request_json FROM tasks WHERE idempotency_key = ?1",
                [key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        record
            .map(|(task, request)| {
                Ok((
                    serde_json::from_str(&task)?,
                    serde_json::from_str(&request)?,
                ))
            })
            .transpose()
    }

    pub fn insert_task(&self, task: &Task, request: &NewTask) -> Result<(), Error> {
        self.connection.execute(
            "INSERT INTO tasks (id,idempotency_key,request_json,state,target_worker,assigned_worker,priority,created_at,deadline_at,lease_expires_at,body) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![task.id,request.idempotency_key,serde_json::to_string(request)?,task.state.as_str(),task.target_worker,task.assigned_worker,task.priority,task.created_at,task.deadline_at,task.lease_expires_at,serde_json::to_string(task)?],
        )?;
        Ok(())
    }

    pub fn save_task(&self, task: &Task) -> Result<(), Error> {
        self.connection.execute(
            "UPDATE tasks SET state=?2,assigned_worker=?3,lease_expires_at=?4,body=?5 WHERE id=?1",
            params![
                task.id,
                task.state.as_str(),
                task.assigned_worker,
                task.lease_expires_at,
                serde_json::to_string(task)?
            ],
        )?;
        Ok(())
    }

    pub fn append_event(&self, task: &Task, at: i64, message: &str) -> Result<(), Error> {
        self.connection.execute(
            "INSERT INTO task_events (task_id,at,state,message) VALUES (?1,?2,?3,?4)",
            params![task.id, at, task.state.as_str(), message],
        )?;
        Ok(())
    }

    pub fn tasks(&self, limit: u32, offset: u32) -> Result<Vec<Task>, Error> {
        self.task_query(TASK_PAGE_SQL, params![limit, offset])
    }

    pub fn events(&self, id: &str, limit: u32, offset: u32) -> Result<Vec<TaskEvent>, Error> {
        let mut statement = self.connection.prepare("SELECT sequence,at,state,message FROM task_events WHERE task_id=?1 ORDER BY sequence LIMIT ?2 OFFSET ?3")?;
        let rows = statement.query_map(params![id, limit, offset], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;
        let mut events = Vec::new();
        for row in rows {
            let (sequence, at, state, message) = row?;
            events.push(TaskEvent {
                sequence,
                task_id: id.to_owned(),
                at,
                state: serde_json::from_str(&format!("\"{state}\""))?,
                message,
            });
        }
        Ok(events)
    }

    pub fn unfinished_count(&self) -> Result<u32, Error> {
        Ok(self
            .connection
            .query_row(UNFINISHED_COUNT_SQL, [], |row| row.get(0))?)
    }

    pub fn candidate(
        &self,
        worker_id: &str,
        capabilities: &[String],
        now: i64,
    ) -> Result<Option<Task>, Error> {
        // The service supplies authorized capabilities; SQL only applies the requested filter.
        Ok(self.task_query("SELECT body FROM tasks WHERE state='QUEUED' AND target_worker=?1 AND deadline_at>?2 AND NOT EXISTS (SELECT 1 FROM json_each(json_extract(tasks.body,'$.required_capabilities')) AS required WHERE required.value NOT IN (SELECT value FROM json_each(?3))) ORDER BY priority DESC,created_at,id LIMIT 1",params![worker_id,now,serde_json::to_string(capabilities)?])?.pop())
    }

    pub fn active_for_worker(&self, worker_id: &str) -> Result<Option<Task>, Error> {
        Ok(self.task_query("SELECT body FROM tasks WHERE assigned_worker=?1 AND state IN ('ASSIGNED','RUNNING') ORDER BY created_at,id LIMIT 1",[worker_id])?.pop())
    }

    pub fn expired(&self, now: i64, limit: u32) -> Result<Vec<Task>, Error> {
        self.task_query(EXPIRED_SQL, params![now, limit])
    }

    pub fn worker(&self, id: &str) -> Result<Option<Worker>, Error> {
        let body: Option<String> = self
            .connection
            .query_row("SELECT body FROM workers WHERE id=?1", [id], |row| {
                row.get(0)
            })
            .optional()?;
        body.map(|body| serde_json::from_str(&body))
            .transpose()
            .map_err(Error::from)
    }

    pub fn worker_count(&self) -> Result<u32, Error> {
        Ok(self
            .connection
            .query_row("SELECT count(*) FROM workers", [], |row| row.get(0))?)
    }

    pub fn save_worker(&self, worker: &Worker) -> Result<(), Error> {
        self.connection.execute("INSERT INTO workers(id,body) VALUES (?1,?2) ON CONFLICT(id) DO UPDATE SET body=excluded.body",params![worker.id,serde_json::to_string(worker)?])?;
        Ok(())
    }

    pub fn workers(&self, limit: u32, offset: u32) -> Result<Vec<Worker>, Error> {
        let mut statement = self
            .connection
            .prepare("SELECT body FROM workers ORDER BY id LIMIT ?1 OFFSET ?2")?;
        let rows = statement.query_map(params![limit, offset], |row| row.get::<_, String>(0))?;
        rows.map(|body| Ok(serde_json::from_str(&body?)?)).collect()
    }

    fn task_query(&self, sql: &str, parameters: impl rusqlite::Params) -> Result<Vec<Task>, Error> {
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map(parameters, |row| row.get::<_, String>(0))?;
        rows.map(|body| Ok(serde_json::from_str(&body?)?)).collect()
    }

    pub fn state_counts(&self) -> Result<Vec<(TaskState, u32)>, Error> {
        let mut statement = self
            .connection
            .prepare("SELECT state,count(*) FROM tasks GROUP BY state")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?;
        rows.map(|row| {
            let (state, count) = row?;
            Ok((serde_json::from_str(&format!("\"{state}\""))?, count))
        })
        .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(connection: &Connection, sql: &str, parameters: &[i64]) -> String {
        let mut statement = connection
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap();
        statement
            .query_map(rusqlite::params_from_iter(parameters), |row| {
                row.get::<_, String>(3)
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .join("; ")
    }

    #[test]
    fn queue_and_recovery_filter_live_state_indexes_and_history_pages_avoid_sorting() {
        let connection =
            connection(":memory:", 1, include_str!("../migrations/0001_hub.sql")).unwrap();
        let count = plan(&connection, UNFINISHED_COUNT_SQL, &[]);
        assert!(
            count.contains("SEARCH tasks USING COVERING INDEX"),
            "{count}"
        );
        assert!(count.contains("state=?"), "{count}");
        let expired = plan(&connection, EXPIRED_SQL, &[1000, 100]);
        assert!(!expired.contains("SCAN tasks"), "{expired}");
        assert!(expired.contains("state=?"), "{expired}");
        let page = plan(&connection, TASK_PAGE_SQL, &[100, 0]);
        assert!(page.contains("USING INDEX tasks_created"), "{page}");
        assert!(!page.contains("TEMP B-TREE"), "{page}");
    }
}
