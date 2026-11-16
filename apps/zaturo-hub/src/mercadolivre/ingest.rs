use super::{MAX_INGEST_BODY_BYTES, auth::CallerPrincipal};
use crate::{
    AppState,
    http::{ApiFailure, database},
    now_ms,
};
use axum::{
    Extension, Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use zaturo_core::{MercadoLivreIngestBatchRequest, MercadoLivreIngestReport};

fn extract_idempotency_key(headers: &HeaderMap) -> Result<String, ApiFailure> {
    let raw = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .ok_or(ApiFailure(
            StatusCode::BAD_REQUEST,
            "missing_idempotency_key",
        ))?;
    if raw.is_empty() || raw.len() > 128 || !raw.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(ApiFailure(
            StatusCode::BAD_REQUEST,
            "invalid_idempotency_key",
        ));
    }
    Ok(raw.to_owned())
}

fn parse_ingest_body(body: &Bytes) -> Result<(String, MercadoLivreIngestBatchRequest), ApiFailure> {
    if body.is_empty() || body.len() > MAX_INGEST_BODY_BYTES {
        return Err(ApiFailure(StatusCode::BAD_REQUEST, "invalid_request"));
    }
    let payload_sha256 = zaturo_transfer::minio::sha256_hex(body);
    let request: MercadoLivreIngestBatchRequest = serde_json::from_slice(body)
        .map_err(|_| ApiFailure(StatusCode::BAD_REQUEST, "invalid_request"))?;
    Ok((payload_sha256, request))
}

pub(super) async fn device_ingest(
    State(state): State<AppState>,
    Extension(principal): Extension<CallerPrincipal>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<MercadoLivreIngestReport>, ApiFailure> {
    let idempotency_key = extract_idempotency_key(&headers)?;
    let (payload_sha256, request) = parse_ingest_body(&body)?;
    if let CallerPrincipal::Device {
        id,
        enforces_worker_match: true,
    } = principal
        && request.worker_id != id
    {
        return Err(ApiFailure(StatusCode::FORBIDDEN, "worker_id_mismatch"));
    }
    let report = database(&state, move |e| {
        e.ingest_mercadolivre_batch(request, &idempotency_key, &payload_sha256, now_ms())
    })
    .await?;
    state.changes.notify_waiters();
    Ok(Json(report))
}

pub(super) async fn owner_ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<MercadoLivreIngestReport>, ApiFailure> {
    let idempotency_key = extract_idempotency_key(&headers)?;
    let (payload_sha256, request) = parse_ingest_body(&body)?;
    let report = database(&state, move |e| {
        e.ingest_mercadolivre_batch(request, &idempotency_key, &payload_sha256, now_ms())
    })
    .await?;
    state.changes.notify_waiters();
    Ok(Json(report))
}
