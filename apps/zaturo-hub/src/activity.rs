//! Authenticated owner/application activity API. Sensitive screens have no persisted route.
use crate::{
    AppState,
    http::{ApiFailure, bearer, database},
    now_ms,
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use zaturo_core::{ChatReply, ChatRequest, FeedPage, FeedQuery};
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/v1/feed", get(feed))
        .route("/v1/chat", axum::routing::post(chat))
        .route("/v1/services", get(services))
        .route("/v1/artifacts/{id}", get(artifact))
        .route_layer(middleware::from_fn_with_state(state, auth))
}
async fn auth(State(state): State<AppState>, request: Request<Body>, next: Next) -> Response {
    let write = request.method() != Method::GET;
    if bearer(request.headers()).is_some_and(|t| state.authorize_app(t, write)) {
        next.run(request).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
async fn feed(
    State(state): State<AppState>,
    Query(query): Query<FeedQuery>,
) -> Result<Json<FeedPage>, ApiFailure> {
    Ok(Json(database(&state, move |e| e.feed(query)).await?))
}
async fn chat(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatReply>, ApiFailure> {
    let target = state.config.worker_id.clone();
    let reply = database(&state, move |e| e.chat(request, &target, now_ms())).await?;
    if reply.task.is_some() {
        state.changes.notify_one();
    }
    Ok(Json(reply))
}
async fn services(State(state): State<AppState>) -> Result<Json<serde_json::Value>, ApiFailure> {
    let limit = state.config.max_page_size;
    let workers = database(&state, move |e| e.workers(limit, 0)).await?;
    let capability = |cap: &str| {
        workers
            .iter()
            .filter(|w| w.capabilities.iter().any(|c| c == cap))
            .map(|w| w.id.clone())
            .collect::<Vec<_>>()
    };
    Ok(Json(
        serde_json::json!({"services":[{"id":"diagnostic","name":"Diagnóstico","action":"diagnostic","workers":capability("diagnostic.v1")},{"id":"youtube","name":"YouTube","action":"youtube_profiles","workers":capability("youtube.read.v1")},{"id":"android","name":"Android USB","action":"android_devices","workers":capability("android.devices.v1")}]}),
    ))
}
async fn artifact(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiFailure> {
    if uuid::Uuid::parse_str(&id).is_err() {
        return Err(ApiFailure(StatusCode::BAD_REQUEST, "invalid_id"));
    }
    let peers = state
        .devices
        .as_ref()
        .map(|r| r.peers())
        .unwrap_or_default();
    for peer in peers {
        let id = id.clone();
        let device = peer.id.clone();
        if let Ok(t) = database(&state, move |e| e.transfer(&id, &device)).await {
            let mime = if t.filename.to_ascii_lowercase().ends_with(".png") {
                "image/png"
            } else if t.filename.to_ascii_lowercase().ends_with(".jpg")
                || t.filename.to_ascii_lowercase().ends_with(".jpeg")
            {
                "image/jpeg"
            } else {
                "application/octet-stream"
            };
            return Ok(Json(
                serde_json::json!({"id":t.id,"filename":t.filename,"mime_type":mime,"size_bytes":t.size_bytes,"sha256":t.sha256,"delivery":"encrypted_inbox","sensitive":false}),
            ));
        }
    }
    Err(ApiFailure(StatusCode::NOT_FOUND, "not_found"))
}
