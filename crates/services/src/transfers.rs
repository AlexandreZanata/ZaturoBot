//! Device-scoped transfer admission and durable checkpoints, independent of HTTP/files.
use crate::{Engine, Error, Result};
use uuid::Uuid;
use zaturo_core::{CreateTransfer, FileTransfer, TransferLimits, TransferState};

impl Engine {
    pub fn create_transfer(
        &self,
        sender: &str,
        request: CreateTransfer,
        limits: &TransferLimits,
        now: i64,
    ) -> Result<FileTransfer> {
        if !limits.validates()
            || !request.validate(limits)
            || zaturo_core::validate_identifier(sender).is_err()
            || sender == request.recipient
        {
            return Err(Error::Invalid("invalid transfer request".into()));
        }
        let expires_at = now
            .checked_add((limits.ttl_seconds * 1000) as i64)
            .ok_or_else(|| Error::Invalid("time overflow".into()))?;
        self.store.transaction(|r| {
            if let Some((existing, previous)) =
                r.transfer_by_key(sender, &request.idempotency_key)?
            {
                return if request == previous {
                    Ok(existing)
                } else {
                    Err(Error::Conflict("transfer idempotency collision".into()))
                };
            }
            let (bytes, count) = r.transfer_capacity()?;
            if count >= limits.max_pending
                || bytes
                    .checked_add(request.size_bytes)
                    .is_none_or(|n| n > limits.max_spool_bytes)
            {
                return Err(Error::Capacity);
            }
            let transfer = FileTransfer {
                id: Uuid::new_v4().to_string(),
                sender: sender.into(),
                recipient: request.recipient.clone(),
                filename: request.filename.clone(),
                size_bytes: request.size_bytes,
                sha256: request.sha256.clone(),
                uploaded_bytes: 0,
                state: TransferState::Uploading,
                created_at: now,
                updated_at: now,
                expires_at,
            };
            r.insert_transfer(&transfer, &request)?;
            r.transfer_event(&transfer, now, "Transfer admitted; storage reserved")?;
            Ok(transfer)
        })
    }
    pub fn transfer(&self, id: &str, device: &str) -> Result<FileTransfer> {
        self.store.read(|r| {
            let t = r.transfer(id)?.ok_or(Error::NotFound)?;
            if !t.visible_to(device) {
                return Err(Error::NotFound);
            }
            Ok(t)
        })
    }
    pub fn list_transfers(
        &self,
        device: &str,
        incoming: bool,
        pending: bool,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<FileTransfer>> {
        self.page(limit, offset)?;
        self.store
            .read(|r| Ok(r.list_transfers(device, incoming, pending, limit, offset)?))
    }
    pub fn advance_transfer(
        &self,
        id: &str,
        sender: &str,
        previous: u64,
        length: u64,
        now: i64,
    ) -> Result<FileTransfer> {
        self.store.transaction(|r| {
            let mut t = r.transfer(id)?.ok_or(Error::NotFound)?;
            if t.sender != sender {
                return Err(Error::NotFound);
            }
            if t.state != TransferState::Uploading
                || t.expires_at <= now
                || t.uploaded_bytes != previous
                || length == 0
                || length > zaturo_core::MAX_CHUNK_BYTES as u64
                || previous
                    .checked_add(length)
                    .is_none_or(|n| n > t.size_bytes)
            {
                return Err(Error::Conflict("invalid upload checkpoint".into()));
            }
            t.uploaded_bytes += length;
            t.updated_at = now;
            r.save_transfer(&t)?;
            Ok(t)
        })
    }
    /// Called only after infrastructure verifies the complete on-disk SHA-256 digest.
    pub fn finish_transfer(
        &self,
        id: &str,
        sender: &str,
        verified_sha256: &str,
        now: i64,
    ) -> Result<FileTransfer> {
        self.store.transaction(|r| {
            let mut t = r.transfer(id)?.ok_or(Error::NotFound)?;
            if t.sender != sender {
                return Err(Error::NotFound);
            }
            if t.expires_at <= now
                || t.state == TransferState::Expired
                || t.uploaded_bytes != t.size_bytes
                || t.sha256 != verified_sha256
            {
                return Err(Error::Conflict(
                    "transfer is incomplete or corrupted".into(),
                ));
            }
            if t.state == TransferState::Uploading {
                t.state = TransferState::Ready;
                t.updated_at = now;
                r.save_transfer(&t)?;
                r.transfer_event(&t, now, "Complete payload SHA-256 verified")?;
            }
            Ok(t)
        })
    }
    pub fn acknowledge_transfer(
        &self,
        id: &str,
        recipient: &str,
        digest: &str,
        now: i64,
    ) -> Result<FileTransfer> {
        self.store.transaction(|r| {
            let mut t = r.transfer(id)?.ok_or(Error::NotFound)?;
            if t.recipient != recipient {
                return Err(Error::NotFound);
            }
            if t.expires_at <= now
                || !matches!(t.state, TransferState::Ready | TransferState::Delivered)
                || digest != t.sha256
            {
                return Err(Error::Conflict("delivery evidence is invalid".into()));
            }
            if t.state == TransferState::Ready {
                t.state = TransferState::Delivered;
                t.updated_at = now;
                r.save_transfer(&t)?;
                r.transfer_event(&t, now, "Recipient verified and published the file")?;
            }
            Ok(t)
        })
    }
    pub fn expired_transfers(&self, now: i64, limit: u32) -> Result<Vec<FileTransfer>> {
        self.page(limit, 0)?;
        self.store.read(|r| Ok(r.expired_transfers(now, limit)?))
    }
    /// Infrastructure removes payload bytes first; metadata/audit evidence remains.
    pub fn expire_transfer(&self, id: &str, now: i64) -> Result<()> {
        self.store.transaction(|r| {
            let mut t = r.transfer(id)?.ok_or(Error::NotFound)?;
            if t.expires_at > now {
                return Err(Error::Conflict("transfer retention has not elapsed".into()));
            }
            if t.state != TransferState::Expired {
                t.state = TransferState::Expired;
                t.updated_at = now;
                r.save_transfer(&t)?;
                r.transfer_event(&t, now, "Expired payload removed; audit retained")?;
            }
            Ok(())
        })
    }
}
