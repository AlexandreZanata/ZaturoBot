//! Authenticated file transport composition. Services own state and authorization rules.
use crate::{
    AppState,
    http::{ApiFailure, bearer, database},
    now_ms,
};
use axum::{
    Extension, Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use zaturo_core::{CreateTransfer, NewTask, TaskKind, TransferState};

#[derive(Clone)]
struct Device(String);
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/device/me", get(me))
        .route("/v1/device/peers", get(peers))
        .route("/v1/device/limits", get(limits))
        .route("/v1/device/tasks", post(task_create))
        .route("/v1/device/tasks/{id}", get(task_show))
        .route("/v1/device/transfers", get(list).post(create))
        .route("/v1/device/transfers/{id}", get(show))
        .route("/v1/device/transfers/{id}/complete", post(complete))
        .route("/v1/device/transfers/{id}/ack", post(ack))
        .layer(DefaultBodyLimit::max(zaturo_protocol::MAX_MESSAGE_BYTES))
        .route(
            "/v1/device/transfers/{id}/chunk",
            get(download)
                .put(upload)
                .layer(DefaultBodyLimit::max(zaturo_core::MAX_CHUNK_BYTES)),
        )
        .route_layer(middleware::from_fn_with_state(state, auth))
}
async fn auth(State(state): State<AppState>, mut request: Request<Body>, next: Next) -> Response {
    if state.devices.is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(id) = bearer(request.headers()).and_then(|v| state.authenticate_device(v)) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Ok(_permit) = state.file_requests.clone().try_acquire_owned() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    request.extensions_mut().insert(Device(id));
    next.run(request).await
}
fn invalid() -> ApiFailure {
    ApiFailure(StatusCode::BAD_REQUEST, "invalid_request")
}
fn conflict() -> ApiFailure {
    ApiFailure(StatusCode::CONFLICT, "file_operation_failed")
}
fn registered(state: &AppState, id: &str) -> bool {
    state
        .devices
        .as_ref()
        .is_some_and(|r| r.peers().iter().any(|p| p.id == id))
}
async fn me(
    State(state): State<AppState>,
    Extension(Device(id)): Extension<Device>,
) -> Result<Json<zaturo_transfer::Peer>, ApiFailure> {
    Ok(Json(
        state
            .devices
            .as_ref()
            .and_then(|r| r.peers().into_iter().find(|p| p.id == id))
            .ok_or_else(invalid)?,
    ))
}
async fn peers(State(state): State<AppState>) -> Json<Vec<zaturo_transfer::Peer>> {
    Json(
        state
            .devices
            .as_ref()
            .map(|r| r.peers())
            .unwrap_or_default(),
    )
}
async fn limits(State(state): State<AppState>) -> Json<zaturo_core::TransferLimits> {
    Json(state.config.transfer_limits)
}
async fn create(
    State(state): State<AppState>,
    Extension(Device(device)): Extension<Device>,
    Json(request): Json<CreateTransfer>,
) -> Result<Json<zaturo_core::FileTransfer>, ApiFailure> {
    if !registered(&state, &request.recipient) {
        return Err(invalid());
    }
    let limits = state.config.transfer_limits.clone();
    Ok(Json(
        database(&state, move |e| {
            e.create_transfer(&device, request, &limits, now_ms())
        })
        .await?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Page {
    #[serde(default = "yes")]
    incoming: bool,
    #[serde(default)]
    pending: bool,
    #[serde(default = "twenty")]
    limit: u32,
    #[serde(default)]
    offset: u32,
}
fn yes() -> bool {
    true
}
fn twenty() -> u32 {
    20
}
async fn list(
    State(state): State<AppState>,
    Extension(Device(device)): Extension<Device>,
    Query(page): Query<Page>,
) -> Result<Json<Vec<zaturo_core::FileTransfer>>, ApiFailure> {
    Ok(Json(
        database(&state, move |e| {
            e.list_transfers(
                &device,
                page.incoming,
                page.pending,
                page.limit,
                page.offset,
            )
        })
        .await?,
    ))
}
async fn show(
    State(state): State<AppState>,
    Extension(Device(device)): Extension<Device>,
    Path(id): Path<String>,
) -> Result<Json<zaturo_core::FileTransfer>, ApiFailure> {
    Ok(Json(
        database(&state, move |e| e.transfer(&id, &device)).await?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Range {
    offset: u64,
    #[serde(default = "chunk_size")]
    length: usize,
}
fn chunk_size() -> usize {
    zaturo_core::DEFAULT_CHUNK_BYTES
}
async fn upload(
    State(state): State<AppState>,
    Extension(Device(device)): Extension<Device>,
    Path(id): Path<String>,
    Query(range): Query<Range>,
    data: Bytes,
) -> Result<Json<zaturo_core::FileTransfer>, ApiFailure> {
    let blobs = state.blobs.clone().ok_or_else(invalid)?;
    let engine = state.engine.clone();
    let lock = state.file_lock.clone().lock_owned().await;
    tokio::task::spawn_blocking(move || {
        let _lock = lock;
        let t = engine.transfer(&id, &device).map_err(|_| conflict())?;
        if t.sender != device
            || t.state != TransferState::Uploading
            || t.expires_at <= now_ms()
            || range
                .offset
                .checked_add(data.len() as u64)
                .is_none_or(|n| n > t.size_bytes)
        {
            return Err(conflict());
        }
        let appended = blobs
            .append(&id, t.uploaded_bytes, range.offset, &data)
            .map_err(|_| conflict())?;
        if appended {
            Ok(Json(
                engine
                    .advance_transfer(&id, &device, range.offset, data.len() as u64, now_ms())
                    .map_err(|_| conflict())?,
            ))
        } else {
            Ok(Json(t))
        }
    })
    .await
    .map_err(|_| conflict())?
}
async fn complete(
    State(state): State<AppState>,
    Extension(Device(device)): Extension<Device>,
    Path(id): Path<String>,
) -> Result<Json<zaturo_core::FileTransfer>, ApiFailure> {
    let blobs = state.blobs.clone().ok_or_else(invalid)?;
    let engine = state.engine.clone();
    let lock = state.file_lock.clone().lock_owned().await;
    tokio::task::spawn_blocking(move || {
        let _lock = lock;
        let t = engine.transfer(&id, &device).map_err(|_| conflict())?;
        if t.sender != device || t.expires_at <= now_ms() || t.state == TransferState::Expired {
            return Err(conflict());
        }
        let hash = blobs.digest(&id, t.size_bytes).map_err(|_| conflict())?;
        Ok(Json(
            engine
                .finish_transfer(&id, &device, &hash, now_ms())
                .map_err(|_| conflict())?,
        ))
    })
    .await
    .map_err(|_| conflict())?
}
async fn download(
    State(state): State<AppState>,
    Extension(Device(device)): Extension<Device>,
    Path(id): Path<String>,
    Query(range): Query<Range>,
) -> Result<Vec<u8>, ApiFailure> {
    let blobs = state.blobs.clone().ok_or_else(invalid)?;
    let t = database(&state, move |e| e.transfer(&id, &device)).await?;
    if !matches!(t.state, TransferState::Ready | TransferState::Delivered)
        || t.expires_at <= now_ms()
    {
        return Err(conflict());
    }
    tokio::task::spawn_blocking(move || {
        blobs
            .read(&t.id, range.offset, range.length, t.size_bytes)
            .map_err(|_| conflict())
    })
    .await
    .map_err(|_| conflict())?
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Digest {
    sha256: String,
}
async fn ack(
    State(state): State<AppState>,
    Extension(Device(device)): Extension<Device>,
    Path(id): Path<String>,
    Json(digest): Json<Digest>,
) -> Result<Json<zaturo_core::FileTransfer>, ApiFailure> {
    Ok(Json(
        database(&state, move |e| {
            e.acknowledge_transfer(&id, &device, &digest.sha256, now_ms())
        })
        .await?,
    ))
}
pub async fn cleanup(state: &AppState) -> anyhow::Result<()> {
    let Some(blobs) = state.blobs.clone() else {
        return Ok(());
    };
    let engine = state.engine.clone();
    let limit = state.config.max_page_size.min(20);
    let lock = state.file_lock.clone().lock_owned().await;
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let _lock = lock;
        for t in engine.expired_transfers(now_ms(), limit)? {
            blobs.remove(&t.id)?;
            engine.expire_transfer(&t.id, now_ms())?;
        }
        Ok(())
    })
    .await??;
    Ok(())
}
async fn task_create(
    State(state): State<AppState>,
    Extension(Device(device)): Extension<Device>,
    Json(mut request): Json<NewTask>,
) -> Result<Json<zaturo_core::Task>, ApiFailure> {
    if request.kind != TaskKind::Diagnostic
        || request.target_worker != device
        || request.data_policy.is_some()
    {
        return Err(invalid());
    }
    request.idempotency_key = format!("{device}.{}", request.idempotency_key);
    let task = database(&state, move |e| e.create(request, now_ms())).await?;
    state.changes.notify_one();
    Ok(Json(task))
}
async fn task_show(
    State(state): State<AppState>,
    Extension(Device(device)): Extension<Device>,
    Path(id): Path<String>,
) -> Result<Json<zaturo_core::Task>, ApiFailure> {
    Ok(Json(
        database(&state, move |e| {
            let t = e.get(&id)?;
            if t.target_worker != device {
                return Err(zaturo_services::Error::NotFound);
            }
            Ok(t)
        })
        .await?,
    ))
}
