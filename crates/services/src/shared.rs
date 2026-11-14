//! Inert shared coordination with authenticated provenance and revision-fenced updates.
use crate::{Engine, Error, Result};
use zaturo_core::{
    SharedCreate, SharedDetail, SharedEntry, SharedNote, SharedState, SharedSummary, SharedUpdate,
};
impl Engine {
    pub fn app_viewer(&self, id: &str) -> Result<Option<String>> {
        self.store.read(|r| Ok(r.app_viewer(id)?))
    }
    pub fn save_app_viewer(&self, id: &str, recipient: &str, now: i64) -> Result<()> {
        if !zaturo_core::ordinary_text(id, 128)
            || !recipient.starts_with("age1")
            || recipient.len() > 128
            || now < 0
        {
            return Err(Error::Invalid("invalid public viewer".into()));
        }
        self.store.transaction(|r| {
            r.save_app_viewer(id, recipient, now)?;
            Ok(())
        })
    }
    pub fn shared_list(
        &self,
        target: Option<&str>,
        state: Option<SharedState>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<SharedSummary>> {
        self.page(limit, offset)?;
        if target.is_some_and(|v| zaturo_core::validate_identifier(v).is_err()) {
            return Err(Error::Invalid("invalid shared target".into()));
        }
        self.store.read(|r| {
            Ok(
                r.shared_tasks(target, state.map(SharedState::as_str), limit, offset)?
                    .into_iter()
                    .map(SharedSummary::from)
                    .collect(),
            )
        })
    }
    pub fn shared_read(&self, id: &str, target: Option<&str>) -> Result<SharedDetail> {
        self.store.read(|r| {
            let task = r
                .shared_task(id)?
                .filter(|t| target.is_none_or(|v| v == t.target_worker))
                .ok_or(Error::NotFound)?;
            Ok(SharedDetail {
                notes: r.shared_notes(id)?,
                task,
            })
        })
    }
    pub fn shared_create(
        &self,
        request: SharedCreate,
        actor: &str,
        now: i64,
    ) -> Result<SharedEntry> {
        self.store
            .transaction(|r| create_record(r, &request, actor, now))
    }
    pub fn shared_update(
        &self,
        id: &str,
        request: SharedUpdate,
        actor: &str,
        target: Option<&str>,
        now: i64,
    ) -> Result<SharedEntry> {
        if !request.validates() || now < 0 || !zaturo_core::ordinary_text(actor, 128) {
            return Err(Error::Invalid("invalid shared update".into()));
        }
        let serialized = serde_json::to_string(&(id, &request)).map_err(zaturo_db::Error::from)?;
        self.store.transaction(|r| {
            let mut entry = r
                .shared_task(id)?
                .filter(|t| target.is_none_or(|v| v == t.target_worker))
                .ok_or(Error::NotFound)?;
            if let Some((previous, reply)) = r.shared_request(actor, &request.idempotency_key)? {
                if previous != serialized {
                    return Err(Error::Conflict("shared idempotency collision".into()));
                }
                return Ok(serde_json::from_str(&reply).map_err(zaturo_db::Error::from)?);
            }
            if entry.revision != request.expected_revision
                || entry.revision >= 40
                || entry.state == SharedState::Done
            {
                return Err(Error::Conflict(
                    "shared task revision changed or task closed".into(),
                ));
            }
            entry.revision += 1;
            entry.state = request.state;
            entry.updated_at = now.max(entry.updated_at);
            r.save_shared_note(
                id,
                &SharedNote {
                    revision: entry.revision,
                    author: actor.into(),
                    state: request.state,
                    note: request.note.clone(),
                    created_at: entry.updated_at,
                },
            )?;
            r.save_shared_task(&entry)?;
            r.save_shared_request(
                actor,
                &request.idempotency_key,
                &serialized,
                &serde_json::to_string(&entry).map_err(zaturo_db::Error::from)?,
            )?;
            Ok(entry)
        })
    }
}

pub(crate) fn create_record(
    r: &zaturo_db::Repository<'_>,
    request: &SharedCreate,
    actor: &str,
    now: i64,
) -> Result<SharedEntry> {
    if !request.validates() || now < 0 || !zaturo_core::ordinary_text(actor, 128) {
        return Err(Error::Invalid(
            "invalid shared task; credentials are not accepted".into(),
        ));
    }
    let serialized = serde_json::to_string(&request).map_err(zaturo_db::Error::from)?;

    if let Some((previous, reply)) = r.shared_request(actor, &request.idempotency_key)? {
        if previous != serialized {
            return Err(Error::Conflict("shared idempotency collision".into()));
        }
        return Ok(serde_json::from_str(&reply).map_err(zaturo_db::Error::from)?);
    }
    if r.shared_count()? >= 1000 {
        return Err(Error::Capacity);
    }
    let entry = SharedEntry {
        id: uuid::Uuid::new_v4().to_string(),
        target_worker: request.target_worker.clone(),
        title: request.title.clone(),
        instructions: request.instructions.clone(),
        transfer_ids: request.transfer_ids.clone(),
        author: actor.into(),
        state: SharedState::Pending,
        revision: 1,
        created_at: now,
        updated_at: now,
    };
    r.save_shared_task(&entry)?;
    r.save_shared_request(
        actor,
        &request.idempotency_key,
        &serialized,
        &serde_json::to_string(&entry).map_err(zaturo_db::Error::from)?,
    )?;
    Ok(entry)
}
