use crate::{
    AppState,
    http::{bearer, database},
    now_ms,
};
use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::time::Duration;
use zaturo_core::TaskState;
use zaturo_protocol::{ClientMessage, MAX_MESSAGE_BYTES, ServerMessage, VERSION};

pub async fn upgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if !bearer(&headers).is_some_and(|value| state.device.verifies(value)) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Ok(permit) = state.connections.clone().try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    ws.max_message_size(MAX_MESSAGE_BYTES)
        .max_frame_size(MAX_MESSAGE_BYTES)
        .on_upgrade(move |socket| async move {
            let _permit = permit;
            session(state, socket).await;
        })
        .into_response()
}

struct DeviceGuard {
    state: AppState,
    id: String,
}
impl Drop for DeviceGuard {
    fn drop(&mut self) {
        if let Ok(mut connected) = self.state.connected.lock() {
            connected.remove(&self.id);
        }
    }
}

async fn send(socket: &mut WebSocket, message: ServerMessage) -> bool {
    let Ok(encoded) = serde_json::to_string(&message) else {
        return false;
    };
    matches!(
        tokio::time::timeout(
            Duration::from_secs(5),
            socket.send(Message::Text(encoded.into()))
        )
        .await,
        Ok(Ok(()))
    )
}

async fn session(state: AppState, mut socket: WebSocket) {
    let hello = tokio::time::timeout(Duration::from_secs(5), socket.recv()).await;
    let Ok(Some(Ok(Message::Text(text)))) = hello else {
        return;
    };
    let Ok(ClientMessage::Hello {
        version: VERSION,
        worker_id,
        capabilities,
    }) = serde_json::from_str(&text)
    else {
        return;
    };
    if worker_id != state.config.worker_id || capabilities != ["diagnostic.v1"] {
        return;
    }
    {
        let Ok(mut connected) = state.connected.lock() else {
            return;
        };
        if !connected.insert(worker_id.clone()) {
            return;
        }
    }
    let _guard = DeviceGuard {
        state: state.clone(),
        id: worker_id.clone(),
    };
    let identity = worker_id.clone();
    if database(&state, move |e| {
        e.register(&identity, &capabilities, now_ms())
    })
    .await
    .is_err()
    {
        return;
    }
    if !send(&mut socket, ServerMessage::Welcome { version: VERSION }).await {
        return;
    }
    tracing::info!(worker_id = %worker_id, "worker_connected");
    let mut tick = tokio::time::interval(Duration::from_secs(5));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut outstanding: Option<String> = None;
    let mut last_seen = tokio::time::Instant::now();
    loop {
        tokio::select! {
            _ = async {
                tokio::select! {
                    _ = tick.tick() => {},
                    _ = state.changes.notified() => {},
                }
            } => {
                if last_seen.elapsed() > Duration::from_secs(state.config.lease_seconds) { break; }
                if let Some(task_id) = outstanding.clone() {
                    let id = task_id.clone();
                    if let Ok(task) = database(&state, move |e| e.get(&id)).await
                        && matches!(task.state, TaskState::Paused | TaskState::Cancelled | TaskState::Failed) {
                            if !send(&mut socket, ServerMessage::Revoked { task_id }).await { break; }
                            outstanding = None;
                            state.changes.notify_one();
                    }
                } else {
                    let id = worker_id.clone();
                    if let Ok(Some(assignment)) = database(&state, move |e| e.claim(&id, now_ms())).await {
                        outstanding = Some(assignment.task.id.clone());
                        if !send(&mut socket, ServerMessage::Assignment { assignment: Box::new(assignment) }).await { break; }
                    }
                }
            }
            received = socket.recv() => {
                let Some(Ok(message)) = received else { break; };
                let Message::Text(text) = message else { if matches!(message, Message::Close(_)) { break; } continue; };
                let Ok(message) = serde_json::from_str::<ClientMessage>(&text) else { break; };
                last_seen = tokio::time::Instant::now();
                let id = worker_id.clone();
                let mut started_id = None;
                let heartbeat = matches!(message, ClientMessage::Heartbeat { .. });
                let (result, completed_id) = match message {
                    ClientMessage::Hello { .. } => break,
                    ClientMessage::Heartbeat { lease_token } => (database(&state, move |e| e.heartbeat(&id, lease_token.as_deref(), now_ms())).await, None),
                    ClientMessage::Started { task_id, lease_token } => {
                        if outstanding.as_deref() != Some(task_id.as_str()) { break; }
                        started_id = Some(task_id.clone());
                        (database(&state, move |e| e.start(&task_id, &id, &lease_token, now_ms()).map(|_| ())).await, None)
                    }
                    ClientMessage::Completed { task_id, lease_token, result } => {
                        if outstanding.as_deref() != Some(task_id.as_str()) { break; }
                        let completed_id = task_id.clone();
                        (database(&state, move |e| e.complete(&task_id, &id, &lease_token, result, now_ms()).map(|_| ())).await, Some(completed_id))
                    }
                };
                if result.is_err() { if !send(&mut socket, ServerMessage::Error { code: "reconciliation_required".into() }).await { break; } break; }
                if heartbeat && !send(&mut socket, ServerMessage::Heartbeat).await { break; }
                if let Some(task_id) = started_id
                    && !send(&mut socket, ServerMessage::Ready { task_id }).await { break; }
                if let Some(task_id) = completed_id {
                    if !send(&mut socket, ServerMessage::Acknowledged { task_id }).await { break; }
                    outstanding = None;
                    state.changes.notify_one();
                }
            }
        }
    }
    tracing::info!(worker_id = %worker_id, "worker_disconnected");
}
