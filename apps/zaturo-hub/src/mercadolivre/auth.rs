use crate::{AppState, http::bearer};
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

#[derive(Clone)]
pub(super) enum CallerPrincipal {
    Owner,
    Device {
        id: String,
        enforces_worker_match: bool,
    },
}

pub(super) async fn device_or_owner_auth(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let Some(token) = bearer(request.headers()) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let principal = if state.owner.verifies(token) {
        CallerPrincipal::Owner
    } else if let Some(device_id) = state.authenticate_device(token) {
        CallerPrincipal::Device {
            id: device_id,
            enforces_worker_match: state.devices.is_some(),
        }
    } else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    request.extensions_mut().insert(principal);
    next.run(request).await
}

pub(super) async fn owner_only_auth(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if bearer(request.headers()).is_some_and(|t| state.owner.verifies(t)) {
        next.run(request).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

pub(super) async fn catalog_auth(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if bearer(request.headers()).is_some_and(|t| state.authorize_app(t, false)) {
        next.run(request).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
