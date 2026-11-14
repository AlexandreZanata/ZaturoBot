//! Shared instructions are inert data, scoped to an authenticated app or worker identity.
use crate::{
    AppState,
    http::{ApiFailure, bearer, database},
    now_ms,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use serde::Deserialize;
use zaturo_core::{
    SharedCreate, SharedDetail, SharedEntry, SharedState, SharedSummary, SharedUpdate,
};
struct Actor {
    id: String,
    target: Option<String>,
}
fn actor(state: &AppState, headers: &HeaderMap, write: bool) -> Result<Actor, ApiFailure> {
    let token = bearer(headers).ok_or(ApiFailure(StatusCode::UNAUTHORIZED, "unauthorized"))?;
    if state.owner.verifies(token) {
        return Ok(Actor {
            id: "owner".into(),
            target: None,
        });
    }
    if let Some(id) = state.apps.as_ref().and_then(|a| a.identity(token, write)) {
        return Ok(Actor {
            id: format!("app:{id}"),
            target: None,
        });
    }
    if let Some(id) = state.authenticate_device(token) {
        return Ok(Actor {
            id: format!("worker:{id}"),
            target: Some(id),
        });
    }
    Err(ApiFailure(StatusCode::UNAUTHORIZED, "unauthorized"))
}
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/shared/tasks", get(list).post(create))
        .route("/v1/shared/tasks/{id}", get(read))
        .route("/v1/shared/tasks/{id}/updates", axum::routing::post(update))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct List {
    target_worker: Option<String>,
    state: Option<SharedState>,
    #[serde(default = "limit")]
    limit: u32,
    #[serde(default)]
    offset: u32,
}
fn limit() -> u32 {
    10
}
async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(mut query): Query<List>,
) -> Result<Json<Vec<SharedSummary>>, ApiFailure> {
    let actor = actor(&state, &headers, false)?;
    if let Some(target) = actor.target {
        if query.target_worker.as_ref().is_some_and(|v| v != &target) {
            return Err(ApiFailure(StatusCode::FORBIDDEN, "wrong_shared_target"));
        }
        query.target_worker = Some(target);
    }
    Ok(Json(
        database(&state, move |e| {
            e.shared_list(
                query.target_worker.as_deref(),
                query.state,
                query.limit,
                query.offset,
            )
        })
        .await?,
    ))
}
async fn read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<SharedDetail>, ApiFailure> {
    let actor = actor(&state, &headers, false)?;
    Ok(Json(
        database(&state, move |e| e.shared_read(&id, actor.target.as_deref())).await?,
    ))
}
async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SharedCreate>,
) -> Result<Json<SharedEntry>, ApiFailure> {
    let actor = actor(&state, &headers, true)?;
    if actor
        .target
        .as_ref()
        .is_some_and(|v| v != &request.target_worker)
    {
        return Err(ApiFailure(StatusCode::FORBIDDEN, "wrong_shared_target"));
    }
    Ok(Json(
        database(&state, move |e| {
            e.shared_create(request, &actor.id, now_ms())
        })
        .await?,
    ))
}
async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(request): Json<SharedUpdate>,
) -> Result<Json<SharedEntry>, ApiFailure> {
    let actor = actor(&state, &headers, true)?;
    Ok(Json(
        database(&state, move |e| {
            e.shared_update(&id, request, &actor.id, actor.target.as_deref(), now_ms())
        })
        .await?,
    ))
}
