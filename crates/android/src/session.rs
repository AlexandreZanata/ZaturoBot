//! One revocable in-process lease per configured USB identity. Authentication is upstream.
use crate::{Error, Result};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};
use tokio::sync::watch;
use uuid::Uuid;
use zaturo_core::{
    AndroidCapability, AndroidControlActor, AndroidSessionStatus, MAX_ANDROID_DEVICES,
    MAX_ANDROID_LEASE_SECONDS, RegisteredAndroidDevice,
};

struct LeaseState {
    token: Uuid,
    device: RegisteredAndroidDevice,
    actor: AndroidControlActor,
    expires_at: i64,
    revoked: watch::Sender<bool>,
}
/// Opaque local capability. A claimed actor in a remote payload is never authentication.
#[derive(Clone)]
pub struct SessionLease(Arc<LeaseState>);
impl std::fmt::Debug for SessionLease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionLease([REDACTED])")
    }
}
impl SessionLease {
    pub fn status(&self) -> AndroidSessionStatus {
        AndroidSessionStatus {
            device_id: self.0.device.device_id.clone(),
            actor: self.0.actor,
            expires_at: self.0.expires_at,
        }
    }
    pub fn device(&self) -> &RegisteredAndroidDevice {
        &self.0.device
    }
    pub fn authorize_sensitive(&self, device: &RegisteredAndroidDevice, now: i64) -> Result<()> {
        if self.0.actor != AndroidControlActor::Owner
            || self.0.device != *device
            || now < 0
            || now >= self.0.expires_at
            || *self.0.revoked.borrow()
            || !device
                .capabilities
                .contains(&AndroidCapability::SensitiveScreenView)
        {
            return Err(Error::PermissionDenied);
        }
        Ok(())
    }
    pub async fn revoked(&self) {
        let mut signal = self.0.revoked.subscribe();
        let _ = signal.wait_for(|v| *v).await;
    }
}
#[derive(Clone)]
pub struct SessionManager {
    devices: Arc<BTreeMap<String, RegisteredAndroidDevice>>,
    leases: Arc<Mutex<BTreeMap<String, Arc<LeaseState>>>>,
}
impl SessionManager {
    pub fn new(devices: Vec<RegisteredAndroidDevice>) -> Result<Self> {
        if devices.is_empty() || devices.len() > MAX_ANDROID_DEVICES {
            return Err(Error::Invalid);
        }
        let mut registry = BTreeMap::new();
        let mut serials = std::collections::BTreeSet::new();
        for device in devices {
            device.validate().map_err(|_| Error::Invalid)?;
            if !serials.insert(device.adb_serial.clone())
                || registry.insert(device.device_id.clone(), device).is_some()
            {
                return Err(Error::Invalid);
            }
        }
        Ok(Self {
            devices: Arc::new(registry),
            leases: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }
    pub fn device(&self, id: &str) -> Result<&RegisteredAndroidDevice> {
        self.devices.get(id).ok_or(Error::Invalid)
    }
    /// Call only after the application authenticates and authorizes the requested actor.
    pub fn acquire(
        &self,
        id: &str,
        actor: AndroidControlActor,
        seconds: u64,
        now: i64,
    ) -> Result<SessionLease> {
        self.create(id, actor, seconds, now, false)
    }
    /// Owner takeover revokes the old lease before creating its replacement.
    pub fn owner_takeover(&self, id: &str, seconds: u64, now: i64) -> Result<SessionLease> {
        self.create(id, AndroidControlActor::Owner, seconds, now, true)
    }
    fn create(
        &self,
        id: &str,
        actor: AndroidControlActor,
        seconds: u64,
        now: i64,
        takeover: bool,
    ) -> Result<SessionLease> {
        let device = self.device(id)?.clone();
        if now < 0 || !(1..=MAX_ANDROID_LEASE_SECONDS).contains(&seconds) {
            return Err(Error::Invalid);
        }
        let expires_at = now
            .checked_add((seconds * 1000) as i64)
            .ok_or(Error::Invalid)?;
        let mut leases = self.leases.lock().map_err(|_| Error::SessionUnavailable)?;
        if let Some(active) = leases.get(id) {
            if active.expires_at > now && !*active.revoked.borrow() && !takeover {
                return Err(Error::Busy);
            }
            active.revoked.send_replace(true);
        }
        let (revoked, _) = watch::channel(false);
        let lease = Arc::new(LeaseState {
            token: Uuid::new_v4(),
            device,
            actor,
            expires_at,
            revoked,
        });
        leases.insert(id.into(), lease.clone());
        Ok(SessionLease(lease))
    }
    pub fn release(&self, lease: &SessionLease) -> Result<()> {
        let mut leases = self.leases.lock().map_err(|_| Error::SessionUnavailable)?;
        if leases
            .get(&lease.0.device.device_id)
            .is_some_and(|active| active.token == lease.0.token)
        {
            leases.remove(&lease.0.device.device_id);
            lease.0.revoked.send_replace(true);
            Ok(())
        } else {
            Err(Error::PermissionDenied)
        }
    }
    pub fn active(&self, id: &str, now: i64) -> Result<Option<AndroidSessionStatus>> {
        self.device(id)?;
        let leases = self.leases.lock().map_err(|_| Error::SessionUnavailable)?;
        Ok(leases
            .get(id)
            .filter(|l| l.expires_at > now && !*l.revoked.borrow())
            .map(|l| SessionLease(l.clone()).status()))
    }
}
