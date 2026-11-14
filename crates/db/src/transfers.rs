//! Hub transfer metadata; payload bytes remain in the bounded filesystem spool.
use crate::{Error, Repository};
use rusqlite::{OptionalExtension, params};
use zaturo_core::{CreateTransfer, FileTransfer};

impl Repository<'_> {
    pub fn transfer(&self, id: &str) -> Result<Option<FileTransfer>, Error> {
        let body: Option<String> = self
            .connection
            .query_row("SELECT body FROM file_transfers WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        body.map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(Error::from)
    }
    pub fn transfer_by_key(
        &self,
        sender: &str,
        key: &str,
    ) -> Result<Option<(FileTransfer, CreateTransfer)>, Error> {
        let row: Option<(String, String)> = self.connection.query_row("SELECT body,request_json FROM file_transfers WHERE sender=?1 AND idempotency_key=?2", params![sender,key], |r| Ok((r.get(0)?,r.get(1)?))).optional()?;
        row.map(|(a, b)| Ok((serde_json::from_str(&a)?, serde_json::from_str(&b)?)))
            .transpose()
    }
    pub fn transfer_capacity(&self) -> Result<(u64, u32), Error> {
        Ok(self.connection.query_row("SELECT COALESCE(SUM(size_bytes),0),COUNT(*) FROM file_transfers WHERE state IN ('uploading','ready','delivered')", [], |r| Ok((r.get(0)?,r.get(1)?)))?)
    }
    pub fn insert_transfer(
        &self,
        transfer: &FileTransfer,
        request: &CreateTransfer,
    ) -> Result<(), Error> {
        self.connection.execute("INSERT INTO file_transfers(id,sender,recipient,idempotency_key,request_json,state,size_bytes,uploaded_bytes,created_at,expires_at,body) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![transfer.id,transfer.sender,transfer.recipient,request.idempotency_key,serde_json::to_string(request)?,transfer.state.as_str(),transfer.size_bytes,transfer.uploaded_bytes,transfer.created_at,transfer.expires_at,serde_json::to_string(transfer)?])?;
        Ok(())
    }
    pub fn save_transfer(&self, transfer: &FileTransfer) -> Result<(), Error> {
        self.connection.execute(
            "UPDATE file_transfers SET state=?2,uploaded_bytes=?3,body=?4 WHERE id=?1",
            params![
                transfer.id,
                transfer.state.as_str(),
                transfer.uploaded_bytes,
                serde_json::to_string(transfer)?
            ],
        )?;
        Ok(())
    }
    pub fn transfer_event(
        &self,
        transfer: &FileTransfer,
        now: i64,
        message: &str,
    ) -> Result<(), Error> {
        self.connection.execute(
            "INSERT INTO transfer_events(transfer_id,at,state,message) VALUES (?1,?2,?3,?4)",
            params![transfer.id, now, transfer.state.as_str(), message],
        )?;
        Ok(())
    }
    pub fn list_transfers(
        &self,
        device: &str,
        incoming: bool,
        pending: bool,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<FileTransfer>, Error> {
        let sql = if pending {
            "SELECT body FROM file_transfers WHERE recipient=?1 AND state='ready' ORDER BY created_at,id LIMIT ?2 OFFSET ?3"
        } else if incoming {
            "SELECT body FROM file_transfers WHERE recipient=?1 ORDER BY created_at DESC,id DESC LIMIT ?2 OFFSET ?3"
        } else {
            "SELECT body FROM file_transfers WHERE sender=?1 ORDER BY created_at DESC,id DESC LIMIT ?2 OFFSET ?3"
        };
        let mut statement = self.connection.prepare(sql)?;
        let rows =
            statement.query_map(params![device, limit, offset], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    pub fn expired_transfers(&self, now: i64, limit: u32) -> Result<Vec<FileTransfer>, Error> {
        let mut statement=self.connection.prepare("SELECT body FROM file_transfers WHERE state IN ('uploading','ready','delivered') AND expires_at<=?1 ORDER BY expires_at,id LIMIT ?2")?;
        let rows = statement.query_map(params![now, limit], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::HubStore;
    #[test]
    fn upgrades_hub_one_to_five_without_losing_existing_worker_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hub.db");
        let old = rusqlite::Connection::open(&path).unwrap();
        old.execute_batch(include_str!("../migrations/0001_hub.sql"))
            .unwrap();
        old.execute(
            "INSERT INTO workers(id,body) VALUES (?1,?2)",
            rusqlite::params![
                "home-pc",
                serde_json::to_string(&zaturo_core::Worker {
                    id: "home-pc".into(),
                    capabilities: vec![],
                    last_seen_at: 1234
                })
                .unwrap()
            ],
        )
        .unwrap();
        drop(old);
        let store = HubStore::open(&path).unwrap();
        store
            .read::<_, crate::Error>(|r| {
                assert_eq!(r.workers(10, 0)?.len(), 1);
                assert_eq!(r.transfer_capacity()?, (0, 0));
                Ok(())
            })
            .unwrap();
        drop(store);
        let db = rusqlite::Connection::open(path).unwrap();
        assert_eq!(
            db.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            5
        );
    }
}
