use crate::{config::WorkerConfig, now_ms};
use anyhow::{Result, bail, ensure};
use futures_util::{SinkExt, StreamExt};
use std::{sync::Arc, time::Duration};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{Message, client::IntoClientRequest, protocol::WebSocketConfig},
};
use zaturo_core::Assignment;
use zaturo_db::WorkerJournal;
use zaturo_protocol::{ClientMessage, MAX_MESSAGE_BYTES, ServerMessage, VERSION};
use zaturo_security::{ActionClass, Credential, permits};

pub async fn run(config: WorkerConfig, token: String) -> Result<()> {
    config.validate()?;
    Credential::new(&token)?;
    for path in [&config.workspace_root, &config.artifact_root] {
        std::fs::create_dir_all(path)?;
    }
    if let Some(path) = config
        .database
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(path)?;
    }
    let journal = Arc::new(WorkerJournal::open(&config.database)?);
    let mut backoff = 1;
    loop {
        let started = tokio::time::Instant::now();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => return Ok(()),
            outcome = connect_once(&config, &token, journal.clone()) => {
                match outcome { Ok(()) => tracing::info!("hub_disconnected"), Err(_) => tracing::warn!("hub_connection_failed") }
            }
        }
        if started.elapsed() > Duration::from_secs(30) {
            backoff = 1;
        }
        tokio::select! {
            _ = tokio::signal::ctrl_c() => return Ok(()),
            _ = tokio::time::sleep(Duration::from_secs(backoff)) => {}
        }
        backoff = (backoff * 2).min(config.reconnect_max_seconds);
    }
}

/// One bounded connection, exposed for synthetic local integration tests.
pub async fn connect_once(
    config: &WorkerConfig,
    token: &str,
    journal: Arc<WorkerJournal>,
) -> Result<()> {
    config.validate()?;
    Credential::new(token)?;
    let mut request = config.websocket_url()?.into_client_request()?;
    let mut auth =
        format!("Bearer {token}").parse::<tokio_tungstenite::tungstenite::http::HeaderValue>()?;
    auth.set_sensitive(true);
    request.headers_mut().insert("authorization", auth);
    let ws_config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE_BYTES))
        .max_frame_size(Some(MAX_MESSAGE_BYTES));
    let (mut socket, _) = tokio::time::timeout(
        Duration::from_secs(10),
        connect_async_with_config(request, Some(ws_config), false),
    )
    .await??;
    socket
        .send(Message::Text(
            serde_json::to_string(&ClientMessage::Hello {
                version: VERSION,
                worker_id: config.worker_id.clone(),
                capabilities: config.capabilities.clone(),
            })?
            .into(),
        ))
        .await?;
    let mut tick = tokio::time::interval(Duration::from_secs(config.heartbeat_seconds));
    let mut current: Option<Assignment> = None;
    let mut welcomed = false;
    let mut awaiting_ack = false;
    let mut last_seen = tokio::time::Instant::now();
    loop {
        let outbound = tokio::select! {
            _ = tick.tick() => {
                ensure!(last_seen.elapsed() < Duration::from_secs(45), "hub response timeout");
                ClientMessage::Heartbeat { lease_token: if awaiting_ack { None } else { current.as_ref().map(|a| a.lease_token.clone()) } }
            }
            received = socket.next() => {
                let Some(message) = received else { return Ok(()); };
                let message = message?;
                let Message::Text(text) = message else { if matches!(message, Message::Close(_)) { return Ok(()); } continue; };
                last_seen = tokio::time::Instant::now();
                match serde_json::from_str::<ServerMessage>(&text)? {
                    ServerMessage::Welcome { version: VERSION } if !welcomed => { welcomed = true; tracing::info!("worker_authenticated"); continue; }
                    ServerMessage::Heartbeat if welcomed => continue,
                    ServerMessage::Assignment { assignment } if welcomed && current.is_none() => {
                        ensure!(assignment.task.target_worker == config.worker_id && assignment.task.required_capabilities == ["diagnostic.v1"] && permits("diagnostic.v1", ActionClass::ReadOnly, &config.capabilities), "assignment denied by local policy");
                        let message = ClientMessage::Started { task_id: assignment.task.id.clone(), lease_token: assignment.lease_token.clone() };
                        current = Some(*assignment);
                        message
                    }
                    ServerMessage::Ready { task_id } => {
                        let assignment = current.as_ref().ok_or_else(|| anyhow::anyhow!("unexpected dispatch authorization"))?;
                        ensure!(assignment.task.id == task_id && now_ms() < assignment.task.deadline_at && now_ms() < assignment.lease_expires_at, "expired assignment");
                        let operation = assignment.task.operation_id.clone();
                        let task = task_id.clone();
                        let local = journal.clone();
                        let result = tokio::task::spawn_blocking(move || -> Result<_> {
                            let result = match local.begin(&operation, &task, now_ms())? {
                                Some(result) => result,
                                None => {
                                    let result = zaturo_tools::diagnostic(&operation);
                                    local.record_result(&operation, &result, now_ms())?;
                                    result
                                }
                            };
                            Ok(result)
                        }).await??;
                        awaiting_ack = true;
                        ClientMessage::Completed { task_id, lease_token: assignment.lease_token.clone(), result }
                    }
                    ServerMessage::Acknowledged { task_id } => {
                        ensure!(current.as_ref().is_some_and(|a| a.task.id == task_id), "unexpected acknowledgement");
                        tracing::info!(task_id = %task_id, "task_result_acknowledged");
                        current = None;
                        awaiting_ack = false;
                        continue;
                    }
                    ServerMessage::Revoked { task_id } => { if current.as_ref().is_some_and(|a| a.task.id == task_id) { current = None; awaiting_ack = false; } continue; }
                    ServerMessage::Error { .. } => bail!("hub requires reconciliation"),
                    _ => bail!("unexpected protocol message"),
                }
            }
        };
        tokio::time::timeout(
            Duration::from_secs(5),
            socket.send(Message::Text(serde_json::to_string(&outbound)?.into())),
        )
        .await??;
    }
}
