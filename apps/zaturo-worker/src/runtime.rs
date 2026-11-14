use crate::{config::WorkerConfig, execution, now_ms};
use anyhow::{Result, bail, ensure};
use futures_util::{SinkExt, StreamExt};
use std::{future::pending, sync::Arc, time::Duration};
use tokio::{net::TcpStream, task::JoinHandle};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{Message, client::IntoClientRequest, protocol::WebSocketConfig},
};
use zaturo_core::{Assignment, TaskKind, TaskState};
use zaturo_db::WorkerJournal;
use zaturo_llm::{Cancellation, Router};
use zaturo_protocol::{ClientMessage, MAX_MESSAGE_BYTES, ServerMessage, VERSION};
use zaturo_security::{ActionClass, Credential, permits};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct Job {
    handle: JoinHandle<ClientMessage>,
    cancellation: Cancellation,
    operation_id: String,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancellation.cancel();
        self.handle.abort();
    }
}

async fn stop_job(job: &mut Option<Job>, journal: &Arc<WorkerJournal>) -> Result<()> {
    if let Some(mut running) = job.take() {
        running.cancellation.cancel();
        running.handle.abort();
        let _ = (&mut running.handle).await;
        let local = journal.clone();
        let operation = running.operation_id.clone();
        tokio::task::spawn_blocking(move || local.interrupt_model_operation(&operation, now_ms()))
            .await??;
    }
    Ok(())
}

pub async fn run(config: WorkerConfig, token: String) -> Result<()> {
    config.validate()?;
    Credential::new(&token)?;
    let router = execution::load_router(&config)?;
    let _inbox = crate::files::start_inbox(&config, &token)?;
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
            outcome = connect_once_with_router(&config, &token, journal.clone(), router.clone()) => {
                match outcome { Ok(()) => tracing::info!("hub_disconnected"), Err(_) => tracing::warn!("hub_connection_failed") }
            }
        }
        if started.elapsed() > Duration::from_secs(30) {
            backoff = 1;
        }
        tokio::select! { _ = tokio::signal::ctrl_c() => return Ok(()), _ = tokio::time::sleep(Duration::from_secs(backoff)) => {} }
        backoff = (backoff * 2).min(config.reconnect_max_seconds);
    }
}

/// One bounded connection. Production constructs only the configured real provider.
pub async fn connect_once(
    config: &WorkerConfig,
    token: &str,
    journal: Arc<WorkerJournal>,
) -> Result<()> {
    let router = execution::load_router(config)?;
    connect_once_with_router(config, token, journal, router).await
}

/// Dependency injection for deterministic synthetic provider/transport tests.
pub async fn connect_once_with_router(
    config: &WorkerConfig,
    token: &str,
    journal: Arc<WorkerJournal>,
    router: Option<Arc<Router>>,
) -> Result<()> {
    config.validate()?;
    Credential::new(token)?;
    ensure!(
        !config
            .capabilities
            .iter()
            .any(|c| c == "agent.diagnostic.v1")
            || router.is_some(),
        "assisted capability has no active model router"
    );
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
    let hello = ClientMessage::Hello {
        version: VERSION,
        worker_id: config.worker_id.clone(),
        capabilities: config.capabilities.clone(),
    };
    transmit(&mut socket, hello).await?;
    let mut job = None;
    let result = session(config, &mut socket, &journal, router, &mut job).await;
    stop_job(&mut job, &journal).await?;
    result
}

async fn transmit(socket: &mut Socket, message: ClientMessage) -> Result<()> {
    tokio::time::timeout(
        Duration::from_secs(5),
        socket.send(Message::Text(serde_json::to_string(&message)?.into())),
    )
    .await??;
    Ok(())
}

fn validate_assignment(config: &WorkerConfig, assignment: &Assignment) -> Result<()> {
    let task = &assignment.task;
    ensure!(
        task.target_worker == config.worker_id
            && task.assigned_worker.as_deref() == Some(&config.worker_id)
            && task.lease_token.as_deref() == Some(&assignment.lease_token)
            && task.required_capabilities == [task.kind.capability()]
            && config
                .capabilities
                .iter()
                .any(|c| c == task.kind.capability())
            && matches!(task.state, TaskState::Assigned | TaskState::Running)
            && permits("diagnostic.v1", ActionClass::ReadOnly, &config.capabilities),
        "assignment denied by local policy"
    );
    ensure!(
        task.deadline_at > now_ms() && task.deadline_at.saturating_sub(now_ms()) <= 300_000,
        "assignment deadline is outside the worker limit"
    );
    Ok(())
}

async fn session(
    config: &WorkerConfig,
    socket: &mut Socket,
    journal: &Arc<WorkerJournal>,
    router: Option<Arc<Router>>,
    job: &mut Option<Job>,
) -> Result<()> {
    let mut tick = tokio::time::interval(Duration::from_secs(config.heartbeat_seconds));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
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
            outcome = async { match job.as_mut() { Some(job) => (&mut job.handle).await, None => pending().await } } => {
                let finished = job.take().ok_or_else(|| anyhow::anyhow!("missing execution job"))?;
                let message = outcome.map_err(|_| anyhow::anyhow!("execution job interrupted"))?;
                // Completed joins release the owner guard without leaving detached work.
                drop(finished);
                awaiting_ack = true;
                message
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
                        validate_assignment(config, &assignment)?;
                        let message = ClientMessage::Started { task_id: assignment.task.id.clone(), lease_token: assignment.lease_token.clone() };
                        current = Some(*assignment); message
                    }
                    ServerMessage::Ready { task_id } => {
                        ensure!(job.is_none() && !awaiting_ack, "duplicate dispatch authorization");
                        let assignment = current.as_ref().ok_or_else(|| anyhow::anyhow!("unexpected dispatch authorization"))?;
                        ensure!(assignment.task.id == task_id && now_ms() < assignment.task.deadline_at && now_ms() < assignment.lease_expires_at, "expired assignment");
                        let cancellation = Cancellation::new();
                        let operation_id = assignment.task.operation_id.clone();
                        let work = assignment.clone(); let local = journal.clone(); let models = router.clone(); let cancel = cancellation.clone();
                        let handle = tokio::spawn(async move {
                            match work.task.kind {
                                TaskKind::Diagnostic => execution::diagnostic(work, local, cancel).await,
                                TaskKind::AssistedDiagnostic => execution::execute(work, local, models, cancel).await,
                            }
                        });
                        *job = Some(Job { handle, cancellation, operation_id });
                        continue;
                    }
                    ServerMessage::Acknowledged { task_id } => {
                        ensure!(awaiting_ack && current.as_ref().is_some_and(|a| a.task.id == task_id), "unexpected acknowledgement");
                        tracing::info!(task_id = %task_id, "task_result_acknowledged");
                        current = None; awaiting_ack = false; continue;
                    }
                    ServerMessage::Revoked { task_id } => {
                        if current.as_ref().is_some_and(|a| a.task.id == task_id) { stop_job(job, journal).await?; current = None; awaiting_ack = false; }
                        continue;
                    }
                    ServerMessage::Error { .. } => bail!("hub requires reconciliation"),
                    _ => bail!("unexpected protocol message"),
                }
            }
        };
        transmit(socket, outbound).await?;
    }
}
