use crate::{AppState, now_ms, websocket};
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use zaturo_core::{ControlAction, NewTask};
use zaturo_protocol::{ApiError, MAX_MESSAGE_BYTES};
use zaturo_services::{Engine, Error};

pub fn router(state: AppState) -> Router {
    let owner_routes = Router::new()
        .route("/v1/tasks", get(list).post(create))
        .route("/v1/tasks/{id}", get(show))
        .route("/v1/tasks/{id}/events", get(events))
        .route("/v1/tasks/{id}/pause", post(pause))
        .route("/v1/tasks/{id}/resume", post(resume))
        .route("/v1/tasks/{id}/cancel", post(cancel))
        .route("/v1/workers", get(workers))
        .route_layer(middleware::from_fn_with_state(state.clone(), owner_auth));
    Router::new()
        .merge(owner_routes)
        .route(
            "/healthz",
            get(|| async { Json(serde_json::json!({"status":"ok","protocol":1})) }),
        )
        .route("/v1/worker", get(websocket::upgrade))
        .layer(DefaultBodyLimit::max(MAX_MESSAGE_BYTES))
        .layer(middleware::from_fn_with_state(state.clone(), admission))
        .with_state(state)
}

async fn admission(State(state): State<AppState>, request: Request<Body>, next: Next) -> Response {
    let Ok(_permit) = state.requests.clone().try_acquire_owned() else {
        return (StatusCode::SERVICE_UNAVAILABLE, "request capacity reached").into_response();
    };
    match tokio::time::timeout(std::time::Duration::from_secs(10), next.run(request)).await {
        Ok(response) => response,
        Err(_) => (StatusCode::REQUEST_TIMEOUT, "request timed out").into_response(),
    }
}

pub fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}
async fn owner_auth(State(state): State<AppState>, request: Request<Body>, next: Next) -> Response {
    if bearer(request.headers()).is_some_and(|b| state.owner.verifies(b)) {
        next.run(request).await
    } else {
        failure(StatusCode::UNAUTHORIZED, "unauthorized")
    }
}
fn failure(status: StatusCode, error: &str) -> Response {
    (
        status,
        Json(ApiError {
            error: error.into(),
        }),
    )
        .into_response()
}

pub struct ApiFailure(StatusCode, &'static str);
impl IntoResponse for ApiFailure {
    fn into_response(self) -> Response {
        failure(self.0, self.1)
    }
}

pub async fn database<T, F>(state: &AppState, call: F) -> Result<T, ApiFailure>
where
    T: Send + 'static,
    F: FnOnce(&Engine) -> Result<T, Error> + Send + 'static,
{
    let engine = state.engine.clone();
    match tokio::task::spawn_blocking(move || call(&engine)).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(e)) => Err(match e {
            Error::Invalid(_) => ApiFailure(StatusCode::BAD_REQUEST, "invalid_request"),
            Error::NotFound => ApiFailure(StatusCode::NOT_FOUND, "not_found"),
            Error::Conflict(_) => ApiFailure(StatusCode::CONFLICT, "conflict"),
            Error::Capacity => ApiFailure(StatusCode::TOO_MANY_REQUESTS, "capacity"),
            Error::Storage(_) => ApiFailure(StatusCode::INTERNAL_SERVER_ERROR, "storage_failure"),
        }),
        Err(_) => Err(ApiFailure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_failure",
        )),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Page {
    limit: Option<u32>,
    #[serde(default)]
    offset: u32,
}
impl Page {
    fn limit(&self, state: &AppState) -> u32 {
        self.limit.unwrap_or(20.min(state.config.max_page_size))
    }
}

async fn create(State(state): State<AppState>, Json(request): Json<NewTask>) -> Response {
    match database(&state, move |e| e.create(request, now_ms())).await {
        Ok(v) => {
            state.changes.notify_one();
            (StatusCode::CREATED, Json(v)).into_response()
        }
        Err(e) => e.into_response(),
    }
}
async fn list(State(state): State<AppState>, Query(page): Query<Page>) -> Response {
    let limit = page.limit(&state);
    match database(&state, move |e| e.list(limit, page.offset)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => e.into_response(),
    }
}
async fn show(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match database(&state, move |e| e.get(&id)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => e.into_response(),
    }
}
async fn events(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(page): Query<Page>,
) -> Response {
    let limit = page.limit(&state);
    match database(&state, move |e| e.events(&id, limit, page.offset)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => e.into_response(),
    }
}
async fn workers(State(state): State<AppState>, Query(page): Query<Page>) -> Response {
    let limit = page.limit(&state);
    match database(&state, move |e| e.workers(limit, page.offset)).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => e.into_response(),
    }
}
async fn control(state: AppState, id: String, action: ControlAction) -> Response {
    match database(&state, move |e| e.control(&id, action, now_ms())).await {
        Ok(v) => {
            state.changes.notify_one();
            Json(v).into_response()
        }
        Err(e) => e.into_response(),
    }
}
async fn pause(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    control(state, id, ControlAction::Pause).await
}
async fn resume(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    control(state, id, ControlAction::Resume).await
}
async fn cancel(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    control(state, id, ControlAction::Cancel).await
}
