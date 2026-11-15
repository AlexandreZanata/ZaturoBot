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
use zaturo_core::{
    ControlAction, NewTask, PrivateCapture, PrivateViewGrant, ServiceRequest, TaskKind,
};
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
    prepared: bool,
    displayed: bool,
}
#[derive(Serialize)]
struct Metadata {
    id: String,
    state: &'static str,
    expires_at: i64,
    remaining_millis: u64,
    task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_code: Option<String>,
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
    fn mark_displayed(&mut self, viewer: &str, now: i64) -> Result<String, ApiFailure> {
        if self.viewer != viewer
            || !self.claimed
            || !self.consumed
            || self.unchanged
            || self.grant.expires_at <= now
        {
            return Err(denied());
        }
        let task = self.task_id.clone().ok_or_else(denied)?;
        self.displayed = true;
        Ok(task)
    }
    fn metadata(&self) -> Metadata {
        Metadata {
            id: self.grant.id.clone(),
            state: if self.displayed {
                "displayed"
            } else if self.prepared {
                "prepared"
            } else if self.unchanged {
                "unchanged"
            } else if self.consumed {
                "consumed"
            } else if self.frame.is_some() {
                "ready"
            } else {
                "waiting"
            },
            expires_at: self.grant.expires_at,
            remaining_millis: self
                .grant
                .expires_at
                .saturating_sub(now_ms())
                .clamp(0, 180000) as u64,
            task_id: self.task_id.clone(),
            failure_code: None,
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
        .route("/v1/private-viewers", post(register_viewer))
        .route("/v1/private-views", post(create))
        .route("/v1/private-views/{id}", get(status).delete(revoke))
        .route("/v1/private-views/{id}/content", get(content))
        .route("/v1/private-views/{id}/displayed", post(displayed))
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
    if bearer(request.headers()).is_some_and(|t| {
        state.authorize_app(t, write)
            && (state.viewer_device(t).is_some()
                || state
                    .apps
                    .as_ref()
                    .and_then(|a| a.native_viewer(t))
                    .is_some())
    }) {
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
        .and_then(|t| {
            state
                .apps
                .as_ref()
                .and_then(|a| a.native_viewer(t))
                .or_else(|| state.viewer_device(t))
        })
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
    #[serde(default = "android_source")]
    source: String,
    worker_id: String,
    device_id: String,
    watch_seconds: u64,
    explicit_owner_confirmation: bool,
    #[serde(default)]
    prepared: bool,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    return_to_hub: bool,
}
impl RequestView {
    fn capture(&self) -> Result<PrivateCapture, ApiFailure> {
        match self.source.as_str() {
            "browser"
                if self.device_id == "browser"
                    && self.watch_seconds == 0
                    && !self.prepared
                    && !self.return_to_hub =>
            {
                let url = self
                    .url
                    .as_ref()
                    .filter(|u| zaturo_core::valid_private_page_url(u))
                    .ok_or_else(denied)?;
                Ok(PrivateCapture::Browser { url: url.clone() })
            }
            "android_messages"
                if self.watch_seconds == 0 && self.prepared && self.url.is_none() =>
            {
                Ok(PrivateCapture::Messages {
                    return_to_hub: self.return_to_hub,
                })
            }
            "desktop"
                if self.device_id == "desktop"
                    && self.watch_seconds == 0
                    && !self.prepared
                    && self.url.is_none()
                    && !self.return_to_hub =>
            {
                Ok(PrivateCapture::Current)
            }
            "android"
                if self.url.is_none()
                    && !self.return_to_hub
                    && (!self.prepared || self.watch_seconds == 0) =>
            {
                Ok(PrivateCapture::Current)
            }
            _ => Err(denied()),
        }
    }
}
fn android_source() -> String {
    "android".into()
}
async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RequestView>,
) -> Result<Json<Metadata>, ApiFailure> {
    let viewer = viewer(&state, &headers)?;
    let capture = request.capture()?;
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
    if matches!(
        request.source.as_str(),
        "desktop" | "browser" | "android_messages"
    ) {
        let capability = match request.source.as_str() {
            "browser" => "browser.private.v1",
            "android_messages" => "android.messages.private.v1",
            _ => "computer.private.v1",
        };
        let worker = request.worker_id.clone();
        let supported = database(&state, move |e| {
            Ok(e.workers(100, 0)?
                .into_iter()
                .any(|w| w.id == worker && w.capabilities.iter().any(|c| c == capability)))
        })
        .await?;
        if !supported {
            return Err(ApiFailure(
                StatusCode::FORBIDDEN,
                "private_source_not_configured",
            ));
        }
    }
    let recipient = if viewer.starts_with("app:") {
        let id = viewer.clone();
        database(&state, move |e| e.app_viewer(&id))
            .await?
            .ok_or_else(denied)?
    } else {
        peers
            .iter()
            .find(|p| p.id == viewer)
            .map(|p| p.encryption_recipient.clone())
            .ok_or_else(denied)?
    };
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
        capture,
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
                prepared: request.prepared,
                displayed: false,
            },
        );
    }
    if request.prepared {
        let delayed = state.clone();
        let prepared_id = id.clone();
        let source = request.source.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            let grant = delayed
                .private_views
                .0
                .lock()
                .ok()
                .and_then(|mut sessions| {
                    let session = sessions.get_mut(&prepared_id)?;
                    if !session.prepared || session.grant.expires_at <= now_ms() {
                        return None;
                    }
                    session.prepared = false;
                    Some(session.grant.clone())
                });
            if let Some(grant) = grant {
                let _ = dispatch_view(&delayed, grant, &source).await;
            }
        });
        let sessions = state.private_views.0.lock().map_err(|_| missing())?;
        return Ok(Json(sessions.get(&id).ok_or_else(missing)?.metadata()));
    }
    Ok(Json(dispatch_view(&state, grant, &request.source).await?))
}
async fn dispatch_view(
    state: &AppState,
    grant: PrivateViewGrant,
    source: &str,
) -> Result<Metadata, ApiFailure> {
    let id = grant.id.clone();
    let mut task = NewTask::diagnostic(grant.worker_id.clone(), format!("private-{id}"));
    task.kind = TaskKind::Service;
    task.max_attempts = 1;
    task.timeout_seconds = (grant.expires_at.saturating_sub(now_ms()) / 1000).clamp(1, 165) as u64;
    task.request =
        "Visualização privada autorizada; nenhum conteúdo da tela será salvo no histórico.".into();
    task.service = Some(if source == "browser" {
        ServiceRequest::BrowserPrivateView {
            session_id: id.clone(),
            recipient: grant.recipient.clone(),
            owner_confirmed: true,
        }
    } else if source == "android_messages" {
        let PrivateCapture::Messages { return_to_hub } = grant.capture else {
            return Err(denied());
        };
        ServiceRequest::AndroidMessagesPrivateView {
            device_id: grant.device_id.clone(),
            session_id: id.clone(),
            recipient: grant.recipient.clone(),
            return_to_hub,
            owner_confirmed: true,
        }
    } else if source == "desktop" {
        ServiceRequest::DesktopPrivateView {
            session_id: id.clone(),
            recipient: grant.recipient.clone(),
            owner_confirmed: true,
        }
    } else {
        ServiceRequest::AndroidPrivateView {
            device_id: grant.device_id.clone(),
            session_id: id.clone(),
            recipient: grant.recipient.clone(),
            watch_seconds: grant.watch_seconds,
            owner_confirmed: true,
        }
    });
    let task = match database(state, move |e| e.create(task, now_ms())).await {
        Ok(task) => task,
        Err(error) => {
            if let Ok(mut sessions) = state.private_views.0.lock() {
                sessions.remove(&id);
            }
            return Err(error);
        }
    };
    let metadata = state.private_views.0.lock().ok().and_then(|mut sessions| {
        sessions.get_mut(&id).map(|session| {
            session.task_id = Some(task.id.clone());
            session.metadata()
        })
    });
    if let Some(metadata) = metadata {
        state.changes.notify_one();
        Ok(metadata)
    } else {
        let _ = database(state, move |e| {
            e.control(&task.id, ControlAction::Cancel, now_ms())
        })
        .await;
        Err(missing())
    }
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
        metadata.failure_code = task.failure.clone();
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
async fn displayed(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<StatusCode, ApiFailure> {
    if body != serde_json::json!({"displayed":true}) {
        return Err(denied());
    }
    let viewer = viewer(&state, &headers)?;
    let now = now_ms();
    state.private_views.purge(now);
    let task = {
        let mut sessions = state.private_views.0.lock().map_err(|_| missing())?;
        sessions
            .get_mut(&id)
            .ok_or_else(missing)?
            .mark_displayed(&viewer, now)?
    };
    database(&state,move|engine|engine.publish_activity(zaturo_core::FeedItem{id:uuid::Uuid::new_v4().to_string(),created_at:now,kind:zaturo_core::FeedKind::Task,role:"assistant".into(),text:"Captura privada exibida pelo aplicativo nativo. Imagem temporária; nenhum conteúdo foi salvo no histórico.".into(),task_id:Some(task),artifact_id:None,sensitive:false,capture:None},&format!("private-view:{id}:displayed"))).await?;
    Ok(StatusCode::NO_CONTENT)
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
                capture: PrivateCapture::Current,
            },
            viewer: "home".into(),
            task_id: Some("synthetic-task".into()),
            frame: None,
            digest: None,
            consumed: false,
            unchanged: false,
            claimed: false,
            prepared: false,
            displayed: false,
        }
    }
    #[test]
    fn private_source_options_cannot_cross_capture_boundaries() {
        let base = serde_json::json!({"source":"browser","worker_id":"work","device_id":"browser","watch_seconds":0,"explicit_owner_confirmation":true,"url":"https://example.test/private?fixture=one"});
        let request: RequestView = serde_json::from_value(base.clone()).unwrap();
        let capture = request
            .capture()
            .unwrap_or_else(|_| panic!("valid native request"));
        assert!(!format!("{capture:?}").contains("fixture"));
        for (key, value) in [
            ("source", serde_json::json!("desktop")),
            ("device_id", serde_json::json!("owner-phone")),
            ("prepared", serde_json::json!(true)),
            ("return_to_hub", serde_json::json!(true)),
            ("watch_seconds", serde_json::json!(5)),
            ("url", serde_json::json!("file:///etc/passwd")),
        ] {
            let mut bad = base.clone();
            bad[key] = value;
            assert!(
                serde_json::from_value::<RequestView>(bad)
                    .unwrap()
                    .capture()
                    .is_err()
            );
        }
        let mut sms = base;
        sms.as_object_mut().unwrap().remove("url");
        sms["source"] = serde_json::json!("android_messages");
        sms["device_id"] = serde_json::json!("owner-phone");
        sms["prepared"] = serde_json::json!(true);
        sms["return_to_hub"] = serde_json::json!(true);
        assert!(matches!(
            serde_json::from_value::<RequestView>(sms.clone())
                .unwrap()
                .capture(),
            Ok(PrivateCapture::Messages {
                return_to_hub: true
            })
        ));
        sms["prepared"] = serde_json::json!(false);
        assert!(
            serde_json::from_value::<RequestView>(sms)
                .unwrap()
                .capture()
                .is_err()
        );
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
    fn displayed_receipt_requires_the_original_viewer_and_a_consumed_claimed_frame() {
        let mut active = session();
        assert!(active.mark_displayed("home", 1000).is_err());
        active.claimed = true;
        active.consumed = true;
        assert!(active.mark_displayed("other-viewer", 1000).is_err());
        assert!(active.mark_displayed("home", 60000).is_err());
        assert!(matches!(active.mark_displayed("home",1000),Ok(task) if task=="synthetic-task"));
        assert_eq!(active.metadata().state, "displayed");
    }
    #[test]
    fn prepared_consent_has_no_frame_or_claim_and_keeps_the_original_recipient() {
        let mut prepared = session();
        prepared.prepared = true;
        prepared.task_id = None;
        assert_eq!(prepared.metadata().state, "prepared");
        assert!(prepared.claim_grant("work").is_err());
        assert!(prepared.frame.is_none() && prepared.digest.is_none());
        assert_eq!(prepared.viewer, "home");
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegisterViewer {
    recipient: String,
    explicit_owner_confirmation: bool,
}
async fn register_viewer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterViewer>,
) -> Result<Json<serde_json::Value>, ApiFailure> {
    let identity = bearer(&headers)
        .and_then(|t| state.apps.as_ref().and_then(|a| a.native_viewer(t)))
        .ok_or_else(denied)?;
    if !request.explicit_owner_confirmation
        || !zaturo_transfer::crypto::valid_recipient(&request.recipient)
    {
        return Err(denied());
    }
    let id = identity.clone();
    let recipient = request.recipient;
    database(&state, move |e| {
        e.save_app_viewer(&id, &recipient, now_ms())
    })
    .await?;
    Ok(Json(
        serde_json::json!({"viewer":identity,"registered":true}),
    ))
}
