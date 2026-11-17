//! Authenticated HTTP composition for Mercado Livre commerce ingestion, warm-cached Cloobe-style
//! taxonomy/search/PDP/price-intelligence endpoints, affiliate redirect tracking, alerts, telemetry,
//! and Telegram delivery queue orchestration.

mod auth;
mod catalog_handlers;
mod ingest;
mod telegram_handlers;

#[cfg(test)]
mod tests;

use crate::{AppState, http::ApiFailure};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::StatusCode,
    middleware,
    routing::{get, post},
};

pub(super) const MAX_INGEST_BODY_BYTES: usize = 4 * 1024 * 1024;

pub fn router(state: AppState) -> Router<AppState> {
    let device_or_owner_routes = Router::new()
        .route(
            "/v1/device/commerce/mercadolivre/ingest",
            post(ingest::device_ingest).layer(DefaultBodyLimit::max(MAX_INGEST_BODY_BYTES)),
        )
        .route(
            "/v1/device/commerce/mercadolivre/reference-catalog",
            get(catalog_handlers::device_reference_catalog),
        )
        .route(
            "/v1/commerce/mercadolivre/telegram-queue",
            get(telegram_handlers::telegram_queue),
        )
        .route(
            "/v1/commerce/mercadolivre/telegram-queue/{id}/ack",
            post(telegram_handlers::telegram_ack),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::device_or_owner_auth,
        ));

    let owner_routes = Router::new()
        .route(
            "/v1/commerce/mercadolivre/ingest",
            post(ingest::owner_ingest).layer(DefaultBodyLimit::max(MAX_INGEST_BODY_BYTES)),
        )
        .route(
            "/v1/commerce/mercadolivre/telegram-queue/approve",
            post(telegram_handlers::telegram_approve),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::owner_only_auth,
        ));

    let catalog_routes = Router::new()
        .route(
            "/v1/commerce/mercadolivre/site-catalog",
            get(catalog_handlers::site_catalog),
        )
        .route(
            "/v1/commerce/mercadolivre/taxonomy",
            get(catalog_handlers::taxonomy),
        )
        .route(
            "/v1/commerce/mercadolivre/catalog-search",
            get(catalog_handlers::catalog_search),
        )
        .route(
            "/v1/commerce/mercadolivre/products/{id_or_slug}",
            get(catalog_handlers::product_detail),
        )
        .route(
            "/v1/commerce/mercadolivre/redirect/{id}",
            get(catalog_handlers::offer_redirect),
        )
        .route(
            "/v1/commerce/mercadolivre/telemetry",
            post(catalog_handlers::exposure_telemetry),
        )
        .route(
            "/v1/commerce/mercadolivre/price-alerts",
            post(catalog_handlers::handle_price_alert).get(catalog_handlers::list_price_alerts),
        )
        .route_layer(middleware::from_fn_with_state(state, auth::catalog_auth));

    Router::new()
        .merge(device_or_owner_routes)
        .merge(owner_routes)
        .merge(catalog_routes)
}

pub(super) fn map_service_err(err: zaturo_services::Error) -> ApiFailure {
    match err {
        zaturo_services::Error::Invalid(_) => {
            ApiFailure(StatusCode::BAD_REQUEST, "invalid_request")
        }
        zaturo_services::Error::NotFound => ApiFailure(StatusCode::NOT_FOUND, "not_found"),
        zaturo_services::Error::Conflict(_) => ApiFailure(StatusCode::CONFLICT, "conflict"),
        zaturo_services::Error::Capacity => ApiFailure(StatusCode::TOO_MANY_REQUESTS, "capacity"),
        zaturo_services::Error::Storage(_) => {
            ApiFailure(StatusCode::INTERNAL_SERVER_ERROR, "storage_failure")
        }
    }
}
