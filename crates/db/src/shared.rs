//! Shared task SQL. Services own authorization, state transitions and optimistic revisions.
use crate::{Error, Repository};
use rusqlite::{OptionalExtension, params};
use zaturo_core::{SharedEntry, SharedNote};
impl Repository<'_> {
    pub fn app_viewer(&self, id: &str) -> Result<Option<String>, Error> {
        Ok(self
            .connection
            .query_row(
                "SELECT recipient FROM app_viewers WHERE app_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn save_app_viewer(&self, id: &str, recipient: &str, now: i64) -> Result<(), Error> {
        self.connection.execute("INSERT INTO app_viewers(app_id,recipient,updated_at) VALUES(?1,?2,?3) ON CONFLICT(app_id) DO UPDATE SET recipient=excluded.recipient,updated_at=excluded.updated_at",params![id,recipient,now])?;
        Ok(())
    }
    pub fn shared_request(
        &self,
        actor: &str,
        key: &str,
    ) -> Result<Option<(String, String)>, Error> {
        Ok(self.connection.query_row("SELECT request_json,reply_json FROM shared_requests WHERE actor=?1 AND idempotency_key=?2",params![actor,key],|r|Ok((r.get(0)?,r.get(1)?))).optional()?)
    }
    pub fn save_shared_request(
        &self,
        actor: &str,
        key: &str,
        request: &str,
        reply: &str,
    ) -> Result<(), Error> {
        self.connection.execute("INSERT INTO shared_requests(actor,idempotency_key,request_json,reply_json) VALUES(?1,?2,?3,?4)",params![actor,key,request,reply])?;
        Ok(())
    }
    pub fn shared_task(&self, id: &str) -> Result<Option<SharedEntry>, Error> {
        let body: Option<String> = self
            .connection
            .query_row("SELECT body FROM shared_tasks WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        body.map(|v| serde_json::from_str(&v).map_err(Error::from))
            .transpose()
    }
    pub fn shared_tasks(
        &self,
        target: Option<&str>,
        state: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SharedEntry>, Error> {
        let mut statement=self.connection.prepare("SELECT body FROM shared_tasks WHERE (?1 IS NULL OR target_worker=?1) AND (?2 IS NULL OR state=?2) ORDER BY updated_at DESC,id DESC LIMIT ?3 OFFSET ?4")?;
        let rows = statement.query_map(params![target, state, limit, offset], |r| {
            r.get::<_, String>(0)
        })?;
        rows.map(|v| Ok(serde_json::from_str(&v?)?)).collect()
    }
    pub fn save_shared_task(&self, value: &SharedEntry) -> Result<(), Error> {
        self.connection.execute("INSERT INTO shared_tasks(id,target_worker,state,revision,updated_at,body) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET state=excluded.state,revision=excluded.revision,updated_at=excluded.updated_at,body=excluded.body",params![value.id,value.target_worker,value.state.as_str(),value.revision,value.updated_at,serde_json::to_string(value)?])?;
        Ok(())
    }
    pub fn save_shared_note(&self, id: &str, note: &SharedNote) -> Result<(), Error> {
        self.connection.execute(
            "INSERT INTO shared_notes(task_id,revision,body) VALUES(?1,?2,?3)",
            params![id, note.revision, serde_json::to_string(note)?],
        )?;
        Ok(())
    }
    pub fn shared_notes(&self, id: &str) -> Result<Vec<SharedNote>, Error> {
        let mut statement = self.connection.prepare(
            "SELECT body FROM shared_notes WHERE task_id=?1 ORDER BY revision DESC LIMIT 20",
        )?;
        let rows = statement.query_map([id], |r| r.get::<_, String>(0))?;
        rows.map(|v| Ok(serde_json::from_str(&v?)?)).collect()
    }
    pub fn shared_count(&self) -> Result<u64, Error> {
        Ok(self
            .connection
            .query_row("SELECT count(*) FROM shared_tasks", [], |r| r.get(0))?)
    }
}
