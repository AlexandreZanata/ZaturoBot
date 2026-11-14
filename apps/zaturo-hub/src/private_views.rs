//! Short-lived encrypted screen frames in RAM only. No SQL/image spool/OCR/model access.
use crate::{
    AppState,
    http::{ApiFailure, bearer, database},
    now_ms,
};
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use zaturo_core::{ControlAction, NewTask, PrivateViewGrant, ServiceRequest, TaskKind};
const MAX_FRAME: usize = 8 * 1024 * 1024 + 65536;
#[derive(Clone, Default)]
pub struct PrivateViews(Arc<Mutex<HashMap<String, Session>>>);
struct Session {
    grant: PrivateViewGrant,
    viewer: String,
    task_id: Option<String>,
    frame: Option<Vec<u8>>,
    digest: Option<[u8; 32]>,
    consumed: bool,
    unchanged: bool,
    claimed: bool,
}
#[derive(Serialize)]
struct Metadata {
    id: String,
    state: &'static str,
    expires_at: i64,
    task_id: Option<String>,
}
impl Session {
    fn claim_grant(&mut self, worker: &str) -> Result<PrivateViewGrant, ApiFailure> {
        if self.grant.worker_id != worker
            || self.task_id.is_none()
            || self.claimed
            || self.digest.is_some()
            || self.frame.is_some()
            || self.consumed
            || self.unchanged
        {
            return Err(denied());
        }
        self.claimed = true;
        Ok(self.grant.clone())
    }
    fn metadata(&self) -> Metadata {
        Metadata {
            id: self.grant.id.clone(),
            state: if self.unchanged {
                "unchanged"
            } else if self.consumed {
                "consumed"
            } else if self.frame.is_some() {
                "ready"
            } else {
                "waiting"
            },
            expires_at: self.grant.expires_at,
            task_id: self.task_id.clone(),
        }
    }
}
impl PrivateViews {
    pub fn purge(&self, now: i64) {
        if let Ok(mut sessions) = self.0.lock() {
            sessions.retain(|_, s| s.grant.expires_at > now);
        }
    }
}
fn denied() -> ApiFailure {
    ApiFailure(StatusCode::FORBIDDEN, "private_view_not_authorized")
}
fn missing() -> ApiFailure {
    ApiFailure(StatusCode::NOT_FOUND, "private_view_unavailable_or_expired")
}
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/private-views", post(create))
        .route("/v1/private-views/{id}", get(status).delete(revoke))
        .route("/v1/private-views/{id}/content", get(content))
        .route_layer(middleware::from_fn_with_state(state.clone(), owner_auth))
        .merge(
            Router::new()
                .route("/v1/device/private-views/{id}/grant", get(grant))
                .route("/v1/device/private-views/{id}/complete", post(unchanged))
                .route(
                    "/v1/device/private-views/{id}/content",
                    post(upload).layer(DefaultBodyLimit::max(MAX_FRAME)),
                )
                .route_layer(middleware::from_fn_with_state(state, worker_auth)),
        )
        .layer(middleware::from_fn(no_store))
}
async fn no_store(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    for (name, value) in [
        ("cache-control", "no-store, private, max-age=0"),
        ("pragma", "no-cache"),
        ("expires", "0"),
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
    ] {
        response.headers_mut().insert(
            axum::http::HeaderName::from_static(name),
            axum::http::HeaderValue::from_static(value),
        );
    }
    response.headers_mut().append(
        axum::http::header::VARY,
        axum::http::HeaderValue::from_static("Authorization"),
    );
    response
}
async fn owner_auth(State(state): State<AppState>, request: Request<Body>, next: Next) -> Response {
    let write = request.method() != Method::GET;
    if bearer(request.headers())
        .is_some_and(|t| state.authorize_app(t, write) && state.viewer_device(t).is_some())
    {
        next.run(request).await
    } else {
        denied().into_response()
    }
}
async fn worker_auth(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if bearer(request.headers())
        .and_then(|t| state.authenticate_device(t))
        .is_none()
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let Ok(_permit) = state.private_requests.clone().try_acquire_owned() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    next.run(request).await
}
fn viewer(state: &AppState, headers: &HeaderMap) -> Result<String, ApiFailure> {
    bearer(headers)
        .and_then(|t| state.viewer_device(t))
        .ok_or_else(denied)
}
fn worker(state: &AppState, headers: &HeaderMap) -> Result<String, ApiFailure> {
    bearer(headers)
        .and_then(|t| state.authenticate_device(t))
        .ok_or_else(denied)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestView {
    worker_id: String,
    device_id: String,
    watch_seconds: u64,
    explicit_owner_confirmation: bool,
}
async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RequestView>,
) -> Result<Json<Metadata>, ApiFailure> {
    let viewer = viewer(&state, &headers)?;
    if !request.explicit_owner_confirmation
        || request.watch_seconds > 120
        || (request.watch_seconds > 0 && request.watch_seconds < 5)
        || zaturo_core::validate_identifier(&request.device_id).is_err()
    {
        return Err(denied());
    }
    let peers = state
        .devices
        .as_ref()
        .map(|r| r.peers())
        .ok_or_else(denied)?;
    if !peers.iter().any(|p| p.id == request.worker_id) {
        return Err(denied());
    }
    let recipient = peers
        .iter()
        .find(|p| p.id == viewer)
        .map(|p| p.encryption_recipient.clone())
        .ok_or_else(denied)?;
    let now = now_ms();
    state.private_views.purge(now);
    let id = uuid::Uuid::new_v4().to_string();
    let grant = PrivateViewGrant {
        id: id.clone(),
        worker_id: request.worker_id.clone(),
        device_id: request.device_id,
        recipient,
        watch_seconds: request.watch_seconds,
        expires_at: now + (request.watch_seconds as i64 + 60) * 1000,
    };
    {
        let mut sessions = state.private_views.0.lock().map_err(|_| missing())?;
        if sessions.len() >= 2
            || sessions
                .values()
                .any(|s| s.grant.worker_id == grant.worker_id && !s.consumed)
        {
            return Err(ApiFailure(
                StatusCode::TOO_MANY_REQUESTS,
                "private_view_capacity",
            ));
        }
        sessions.insert(
            id.clone(),
            Session {
                grant: grant.clone(),
                viewer,
                task_id: None,
                frame: None,
                digest: None,
                consumed: false,
                unchanged: false,
                claimed: false,
            },
        );
    }
    let mut task = NewTask::diagnostic(request.worker_id, format!("private-{id}"));
    task.kind = TaskKind::Service;
    task.max_attempts = 1;
    task.timeout_seconds = request.watch_seconds + 45;
    task.request =
        "Visualização privada Android autorizada; nenhum conteúdo da tela será salvo no histórico."
            .into();
    task.service = Some(ServiceRequest::AndroidPrivateView {
        device_id: grant.device_id.clone(),
        session_id: id.clone(),
        recipient: grant.recipient.clone(),
        watch_seconds: request.watch_seconds,
        owner_confirmed: true,
    });
    let task = match database(&state, move |e| e.create(task, now)).await {
        Ok(task) => task,
        Err(error) => {
            if let Ok(mut sessions) = state.private_views.0.lock() {
                sessions.remove(&id);
            }
            return Err(error);
        }
    };
    let metadata = {
        let mut sessions = state.private_views.0.lock().map_err(|_| missing())?;
        let session = sessions.get_mut(&id).ok_or_else(missing)?;
        session.task_id = Some(task.id);
        session.metadata()
    };
    state.changes.notify_one();
    Ok(Json(metadata))
}
async fn status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Metadata>, ApiFailure> {
    let viewer = viewer(&state, &headers)?;
    state.private_views.purge(now_ms());
    let mut metadata = {
        let sessions = state.private_views.0.lock().map_err(|_| missing())?;
        let session = sessions
            .get(&id)
            .filter(|s| s.viewer == viewer)
            .ok_or_else(missing)?;
        session.metadata()
    };
    if metadata.state == "waiting"
        && let Some(task_id) = metadata.task_id.clone()
        && let Ok(task) = database(&state, move |e| e.get(&task_id)).await
    {
        metadata.state = match task.state {
            zaturo_core::TaskState::Failed => "failed",
            zaturo_core::TaskState::Cancelled => "cancelled",
            _ => "waiting",
        };
    }
    Ok(Json(metadata))
}
async fn content(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Vec<u8>, ApiFailure> {
    let viewer = viewer(&state, &headers)?;
    state.private_views.purge(now_ms());
    let mut sessions = state.private_views.0.lock().map_err(|_| missing())?;
    let s = sessions
        .get_mut(&id)
        .filter(|s| s.viewer == viewer)
        .ok_or_else(missing)?;
    let frame = s.frame.take().ok_or_else(missing)?;
    s.consumed = true;
    Ok(frame)
}
async fn revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiFailure> {
    let viewer = viewer(&state, &headers)?;
    let session = {
        let mut sessions = state.private_views.0.lock().map_err(|_| missing())?;
        if sessions.get(&id).is_none_or(|s| s.viewer != viewer) {
            return Err(missing());
        }
        sessions.remove(&id).ok_or_else(missing)?
    };
    if let Some(task) = session.task_id {
        let _ = database(&state, move |e| {
            e.control(&task, ControlAction::Cancel, now_ms())
        })
        .await;
        state.changes.notify_one();
    }
    Ok(StatusCode::NO_CONTENT)
}
async fn grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<PrivateViewGrant>, ApiFailure> {
    let worker = worker(&state, &headers)?;
    state.private_views.purge(now_ms());
    let mut sessions = state.private_views.0.lock().map_err(|_| missing())?;
    let session = sessions.get_mut(&id).ok_or_else(missing)?;
    Ok(Json(session.claim_grant(&worker)?))
}
async fn upload(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    data: Bytes,
) -> Result<StatusCode, ApiFailure> {
    let worker = worker(&state, &headers)?;
    if data.len() > MAX_FRAME || !data.starts_with(b"age-encryption.org/v1\n") {
        return Err(ApiFailure(
            StatusCode::BAD_REQUEST,
            "encrypted_private_frame_required",
        ));
    }
    let now = now_ms();
    state.private_views.purge(now);
    let mut sessions = state.private_views.0.lock().map_err(|_| missing())?;
    let s = sessions
        .get_mut(&id)
        .filter(|s| s.grant.worker_id == worker)
        .ok_or_else(missing)?;
    if !s.claimed {
        return Err(denied());
    }
    let digest: [u8; 32] = Sha256::digest(&data).into();
    if s.digest == Some(digest) {
        return Ok(StatusCode::NO_CONTENT);
    }
    if s.frame.is_some() || s.consumed || s.unchanged {
        return Err(ApiFailure(
            StatusCode::CONFLICT,
            "private_frame_already_delivered",
        ));
    }
    s.digest = Some(digest);
    s.frame = Some(data.to_vec());
    s.grant.expires_at = s.grant.expires_at.min(now + 60_000);
    Ok(StatusCode::NO_CONTENT)
}

async fn unchanged(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ApiFailure> {
    if body != serde_json::json!({"state":"unchanged"}) {
        return Err(denied());
    }
    let worker = worker(&state, &headers)?;
    let now = now_ms();
    state.private_views.purge(now);
    let mut sessions = state.private_views.0.lock().map_err(|_| missing())?;
    let s = sessions
        .get_mut(&id)
        .filter(|s| s.grant.worker_id == worker)
        .ok_or_else(missing)?;
    if !s.claimed || s.frame.is_some() || s.consumed {
        return Err(denied());
    }
    s.unchanged = true;
    s.grant.expires_at = s.grant.expires_at.min(now + 30_000);
    Ok(Json(serde_json::json!({"state":"unchanged"})))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        Session {
            grant: PrivateViewGrant {
                id: "00000000-0000-4000-8000-000000000001".into(),
                worker_id: "work".into(),
                device_id: "owner-phone".into(),
                recipient: "age1synthetic-public-recipient".into(),
                watch_seconds: 0,
                expires_at: 60_000,
            },
            viewer: "home".into(),
            task_id: Some("synthetic-task".into()),
            frame: None,
            digest: None,
            consumed: false,
            unchanged: false,
            claimed: false,
        }
    }
    #[test]
    fn a_private_grant_is_bound_to_one_worker_and_claimable_only_once() {
        let mut active = session();
        assert!(active.claim_grant("other-worker").is_err());
        assert!(!active.claimed);
        assert!(active.claim_grant("work").is_ok());
        assert!(active.claim_grant("work").is_err());
        let mut pending = session();
        pending.task_id = None;
        assert!(pending.claim_grant("work").is_err());
        assert!(!pending.claimed);
        for state in ["digest", "frame", "consumed", "unchanged"] {
            let mut completed = session();
            match state {
                "digest" => completed.digest = Some([0; 32]),
                "frame" => completed.frame = Some(vec![1]),
                "consumed" => completed.consumed = true,
                "unchanged" => completed.unchanged = true,
                _ => unreachable!(),
            }
            assert!(completed.claim_grant("work").is_err());
        }
    }
    #[test]
    fn expiry_removes_ready_ciphertext_and_claims() {
        let store = PrivateViews::default();
        let mut active = session();
        active.frame = Some(vec![1; 32]);
        active.claimed = true;
        store
            .0
            .lock()
            .unwrap()
            .insert(active.grant.id.clone(), active);
        store.purge(59_999);
        assert_eq!(store.0.lock().unwrap().len(), 1);
        store.purge(60_000);
        assert!(store.0.lock().unwrap().is_empty());
    }
}
