//! Hub composition and bounded transport. Domain decisions live in services.
pub mod config;
mod devices;
mod files;
mod http;
pub mod listener;
mod websocket;

use anyhow::{Result, ensure};
use axum::Router;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{Notify, Semaphore};
use zaturo_security::Credential;
use zaturo_services::Engine;

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<Engine>,
    pub owner: Credential,
    pub device: Credential,
    pub config: config::HubConfig,
    pub devices: Option<Arc<devices::DeviceRegistry>>,
    pub blobs: Option<zaturo_transfer::blob::BlobStore>,
    pub file_lock: Arc<tokio::sync::Mutex<()>>,
    pub file_requests: Arc<Semaphore>,
    pub connections: Arc<Semaphore>,
    pub requests: Arc<Semaphore>,
    pub connected: Arc<Mutex<HashSet<String>>>,
    pub changes: Arc<Notify>,
}
impl AppState {
    pub fn new(config: config::HubConfig, owner: &str, device: &str) -> Result<Self> {
        config.validate()?;
        ensure!(owner != device, "owner and device credentials must differ");
        let owner = Credential::new(owner)?;
        let device = Credential::new(device)?;
        if let Some(parent) = config
            .database
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        let devices = config
            .devices_file
            .as_ref()
            .map(|p| devices::DeviceRegistry::load(p, config.max_workers))
            .transpose()?
            .map(Arc::new);
        let blobs = config
            .transfer_root
            .as_ref()
            .map(|p| zaturo_transfer::blob::BlobStore::open(p))
            .transpose()?;
        let engine = Arc::new(Engine::open(&config.database, config.limits())?);
        Ok(Self {
            engine,
            devices,
            blobs,
            file_lock: Arc::new(tokio::sync::Mutex::new(())),
            file_requests: Arc::new(Semaphore::new(2)),
            owner,
            device,
            connections: Arc::new(Semaphore::new(config.max_connections)),
            requests: Arc::new(Semaphore::new(32)),
            connected: Arc::new(Mutex::new(HashSet::new())),
            changes: Arc::new(Notify::new()),
            config,
        })
    }
    pub fn authenticate_device(&self, token: &str) -> Option<String> {
        match &self.devices {
            Some(registry) => registry.authenticate(token),
            None if self.device.verifies(token) => Some(self.config.worker_id.clone()),
            None => None,
        }
    }
}

pub fn router(state: AppState) -> Router {
    http::router(state)
}

pub fn now_ms() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(i64::MAX)
}

pub async fn serve(state: AppState) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(state.config.bind).await?;
    tracing::info!(address = %listener.local_addr()?, "hub_started");
    let cleanup_state = state.clone();
    let cleanup = tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            tick.tick().await;
            if files::cleanup(&cleanup_state).await.is_err() {
                tracing::error!("transfer_cleanup_failed");
            }
        }
    });
    let engine = state.engine.clone();
    let changes = state.changes.clone();
    let recovery = tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let engine = engine.clone();
            match tokio::task::spawn_blocking(move || engine.recover(now_ms())).await {
                Ok(Ok(recovered)) if recovered > 0 => changes.notify_one(),
                Ok(Ok(_)) => {}
                _ => tracing::error!("task_recovery_failed"),
            }
        }
    });
    let listener = listener::BoundedListener::new(listener, 32)?;
    let result = axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await;
    recovery.abort();
    cleanup.abort();
    result?;
    Ok(())
}
