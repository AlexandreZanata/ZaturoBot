use super::map_service_err;
use crate::{
    AppState,
    http::{ApiFailure, database},
    now_ms,
};
use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{
        HeaderMap, HeaderName, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH, LOCATION},
    },
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use std::time::Instant;
use zaturo_core::{
    CatalogSearchQuery, CatalogSearchResponse, ExposureBeaconRequest, ExposureBeaconResponse,
    PriceAlertActionRequest, PriceAlertResponse, ProductDetailResponse,
};

const CATALOG_CACHE_CONTROL: &str = "public, max-age=15, stale-while-revalidate=60";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AlertListQuery {
    subscriber_id: String,
}

async fn warm_or_load_index(
    state: &AppState,
) -> Result<std::sync::Arc<zaturo_services::CompiledCommerceIndex>, ApiFailure> {
    if let Some(warm) = state.engine.mercadolivre_site_catalog_warm() {
        return Ok(warm);
    }
    database(state, move |e| e.mercadolivre_site_catalog_cached(now_ms())).await
}

struct SharedUtf8(std::sync::Arc<String>);

impl AsRef<[u8]> for SharedUtf8 {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

fn cached_json_response(
    headers: &HeaderMap,
    etag: &str,
    generation: u64,
    json_body: std::sync::Arc<String>,
    started: Instant,
) -> Response {
    let timing = format!("cache;dur={:.3}", started.elapsed().as_secs_f64() * 1000.0);
    if headers
        .get(IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.trim() == etag)
    {
        return (
            StatusCode::NOT_MODIFIED,
            [
                (ETAG, etag.to_owned()),
                (CACHE_CONTROL, CATALOG_CACHE_CONTROL.to_owned()),
                (
                    HeaderName::from_static("x-cache-generation"),
                    generation.to_string(),
                ),
                (HeaderName::from_static("server-timing"), timing),
            ],
        )
            .into_response();
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/json")
        .header(ETAG, etag)
        .header(CACHE_CONTROL, CATALOG_CACHE_CONTROL)
        .header("x-cache-generation", generation.to_string())
        .header("server-timing", timing)
        .body(Body::from(Bytes::from_owner(SharedUtf8(json_body))))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

pub(super) async fn site_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiFailure> {
    let started = Instant::now();
    let index = warm_or_load_index(&state).await?;
    Ok(cached_json_response(
        &headers,
        &index.etag,
        index.generation,
        index.snapshot_json.clone(),
        started,
    ))
}

pub(super) async fn device_reference_catalog(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiFailure> {
    let started = Instant::now();
    let index = warm_or_load_index(&state).await?;
    Ok(cached_json_response(
        &headers,
        &index.reference_etag,
        index.generation,
        index.reference_catalog_json.clone(),
        started,
    ))
}

pub(super) async fn taxonomy(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiFailure> {
    let started = Instant::now();
    let index = warm_or_load_index(&state).await?;
    let tax_etag = format!(
        "\"tax-gen-{}-{}\"",
        index.generation,
        index.taxonomy_json.len()
    );
    Ok(cached_json_response(
        &headers,
        &tax_etag,
        index.generation,
        index.taxonomy_json.clone(),
        started,
    ))
}

pub(super) async fn catalog_search(
    State(state): State<AppState>,
    Query(query): Query<CatalogSearchQuery>,
) -> Result<impl IntoResponse, ApiFailure> {
    let started = Instant::now();
    if state.engine.mercadolivre_site_catalog_warm().is_some() {
        let resp: CatalogSearchResponse = state
            .engine
            .mercadolivre_search(&query, now_ms())
            .map_err(map_service_err)?;
        let timing = format!("search;dur={:.3}", started.elapsed().as_secs_f64() * 1000.0);
        return Ok((
            [
                (CACHE_CONTROL, CATALOG_CACHE_CONTROL.to_owned()),
                (HeaderName::from_static("server-timing"), timing),
            ],
            Json(resp),
        )
            .into_response());
    }
    let resp = database(&state, move |e| e.mercadolivre_search(&query, now_ms())).await?;
    Ok(Json(resp).into_response())
}

pub(super) async fn product_detail(
    State(state): State<AppState>,
    Path(id_or_slug): Path<String>,
) -> Result<Json<ProductDetailResponse>, ApiFailure> {
    if state.engine.mercadolivre_site_catalog_warm().is_some() {
        return state
            .engine
            .mercadolivre_product_detail(&id_or_slug, now_ms())
            .map(Json)
            .map_err(map_service_err);
    }
    let resp = database(&state, move |e| {
        e.mercadolivre_product_detail(&id_or_slug, now_ms())
    })
    .await?;
    Ok(Json(resp))
}

pub(super) async fn offer_redirect(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, ApiFailure> {
    let share_link = if state.engine.mercadolivre_site_catalog_warm().is_some() {
        state
            .engine
            .mercadolivre_resolve_redirect(&id, now_ms())
            .map_err(map_service_err)?
    } else {
        database(&state, move |e| {
            e.mercadolivre_resolve_redirect(&id, now_ms())
        })
        .await?
    };
    Ok((
        StatusCode::FOUND,
        [
            (LOCATION, share_link),
            (CACHE_CONTROL, "no-store".to_owned()),
        ],
    )
        .into_response())
}

pub(super) async fn exposure_telemetry(
    State(state): State<AppState>,
    Json(req): Json<ExposureBeaconRequest>,
) -> Result<Json<ExposureBeaconResponse>, ApiFailure> {
    let resp = state
        .engine
        .mercadolivre_record_exposure(req)
        .map_err(map_service_err)?;
    Ok(Json(resp))
}

pub(super) async fn handle_price_alert(
    State(state): State<AppState>,
    Json(req): Json<PriceAlertActionRequest>,
) -> Result<Json<PriceAlertResponse>, ApiFailure> {
    if state.engine.mercadolivre_site_catalog_warm().is_some() {
        return state
            .engine
            .mercadolivre_handle_price_alert(req, now_ms())
            .map(Json)
            .map_err(map_service_err);
    }
    let resp = database(&state, move |e| {
        e.mercadolivre_handle_price_alert(req, now_ms())
    })
    .await?;
    Ok(Json(resp))
}

pub(super) async fn list_price_alerts(
    State(state): State<AppState>,
    Query(q): Query<AlertListQuery>,
) -> Result<Json<PriceAlertResponse>, ApiFailure> {
    let req = PriceAlertActionRequest {
        action: "listar-alertas".into(),
        subscriber_id: q.subscriber_id,
        product_id: None,
        target_price_cents: None,
    };
    handle_price_alert(State(state), Json(req)).await
}
