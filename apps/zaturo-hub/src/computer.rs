//! Device planners may submit only children of their current owner-authorized parent lease.
use crate::{
    AppState,
    http::{ApiFailure, bearer, database},
    now_ms,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::post,
};
use zaturo_core::{ComputerStep, ComputerStepRequest, ComputerStepStatus};
fn identity(state: &AppState, headers: &HeaderMap) -> Result<(String, String), ApiFailure> {
    let worker = bearer(headers)
        .and_then(|t| state.authenticate_device(t))
        .ok_or(ApiFailure(StatusCode::UNAUTHORIZED, "device_required"))?;
    let lease = headers
        .get("x-zaturo-lease")
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() <= 128)
        .ok_or(ApiFailure(StatusCode::UNAUTHORIZED, "lease_required"))?;
    Ok((worker, lease.into()))
}
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/device/computer-actions/{id}/grant",
            axum::routing::get(grant),
        )
        .route("/v1/device/computer/{parent}/steps", post(create))
        .route(
            "/v1/device/computer/{parent}/steps/{id}",
            axum::routing::get(status),
        )
}
async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(parent): Path<String>,
    Json(request): Json<ComputerStepRequest>,
) -> Result<Json<ComputerStep>, ApiFailure> {
    let (worker, lease) = identity(&state, &headers)?;
    let step = database(&state, move |e| {
        e.computer_step_create(&parent, &worker, &lease, request, now_ms())
    })
    .await?;
    state.changes.notify_waiters();
    Ok(Json(step))
}
async fn status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((parent, id)): Path<(String, String)>,
) -> Result<Json<ComputerStepStatus>, ApiFailure> {
    let (worker, lease) = identity(&state, &headers)?;
    Ok(Json(
        database(&state, move |e| {
            e.computer_step_status(&parent, &id, &worker, &lease, now_ms())
        })
        .await?,
    ))
}

async fn grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<zaturo_core::ComputerLease>, ApiFailure> {
    let (worker, lease) = identity(&state, &headers)?;
    Ok(Json(
        database(&state, move |e| {
            e.computer_action_grant(&id, &worker, &lease, now_ms())
        })
        .await?,
    ))
}
