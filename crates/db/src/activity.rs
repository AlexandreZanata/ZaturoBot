//! Indexed owner activity and idempotent chat persistence. No policy decisions.
use crate::{Error, Repository};
use rusqlite::{OptionalExtension, params};
use zaturo_core::{ChatReply, ChatRequest, FeedItem};
impl Repository<'_> {
    pub fn insert_feed(&self, item: &FeedItem, dedupe: &str) -> Result<(), Error> {
        self.connection.execute("INSERT INTO feed_items(id,dedupe_key,created_at,kind,role,text,search_text,task_id,artifact_id,body) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(dedupe_key) DO NOTHING",params![item.id,dedupe,item.created_at,item.kind.as_str(),item.role,item.text,item.text.to_lowercase(),item.task_id,item.artifact_id,serde_json::to_string(item)?])?;
        Ok(())
    }
    pub fn feed_page(
        &self,
        start: i64,
        end: i64,
        kind: Option<&str>,
        query: &str,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<FeedItem>, Error> {
        let mut statement=self.connection.prepare("SELECT body FROM feed_items WHERE created_at>=?1 AND created_at<?2 AND (?3 IS NULL OR kind=?3) AND instr(search_text,?4)>0 ORDER BY created_at DESC,id DESC LIMIT ?5 OFFSET ?6")?;
        let rows = statement.query_map(params![start, end, kind, query, limit, offset], |r| {
            r.get::<_, String>(0)
        })?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    pub fn chat_request(&self, key: &str) -> Result<Option<(ChatRequest, ChatReply)>, Error> {
        let row: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT request_json,reply_json FROM chat_requests WHERE idempotency_key=?1",
                [key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.map(|(a, b)| Ok((serde_json::from_str(&a)?, serde_json::from_str(&b)?)))
            .transpose()
    }
    pub fn save_chat(
        &self,
        key: &str,
        request: &ChatRequest,
        reply: &ChatReply,
    ) -> Result<(), Error> {
        self.connection.execute(
            "INSERT INTO chat_requests(idempotency_key,request_json,reply_json) VALUES (?1,?2,?3)",
            params![
                key,
                serde_json::to_string(request)?,
                serde_json::to_string(reply)?
            ],
        )?;
        Ok(())
    }
}
impl Repository<'_> {
    pub fn task_by_operation(&self, operation: &str) -> Result<Option<zaturo_core::Task>, Error> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT body FROM tasks WHERE json_extract(body,'$.operation_id')=?1",
                [operation],
                |r| r.get(0),
            )
            .optional()?;
        body.map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(Error::from)
    }
    pub fn service_state(&self, operation: &str) -> Result<Option<String>, Error> {
        Ok(self
            .connection
            .query_row(
                "SELECT state FROM service_updates WHERE operation_id=?1",
                [operation],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn save_service_state(&self, operation: &str, state: &str, now: i64) -> Result<(), Error> {
        self.connection.execute("INSERT INTO service_updates(operation_id,state,updated_at) VALUES (?1,?2,?3) ON CONFLICT(operation_id) DO UPDATE SET state=excluded.state,updated_at=excluded.updated_at",params![operation,state,now])?;
        Ok(())
    }
    pub fn feed_by_key(&self, key: &str) -> Result<Option<FeedItem>, Error> {
        let body: Option<String> = self
            .connection
            .query_row(
                "SELECT body FROM feed_items WHERE dedupe_key=?1",
                [key],
                |r| r.get(0),
            )
            .optional()?;
        body.map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(Error::from)
    }
}
