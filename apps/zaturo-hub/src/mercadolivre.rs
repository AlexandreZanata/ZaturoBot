//! Authenticated HTTP composition for Mercado Livre commerce ingestion, warm-cached Cloobe-style
//! taxonomy/search/PDP/price-intelligence endpoints, affiliate redirect tracking, alerts, telemetry,
//! and Telegram delivery queue orchestration.

use crate::{
    AppState,
    http::{ApiFailure, bearer, database},
    now_ms,
};
use axum::{
    Extension, Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{
        HeaderMap, HeaderName, Request, StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH, LOCATION},
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use std::time::Instant;
use zaturo_core::{
    CatalogSearchQuery, CatalogSearchResponse, ExposureBeaconRequest, ExposureBeaconResponse,
    MercadoLivreIngestBatchRequest, MercadoLivreIngestReport, PriceAlertActionRequest,
    PriceAlertResponse, ProductDetailResponse, TelegramDeliveryAck, TelegramDeliveryStatus,
    TelegramOfferQueueItem,
};

const MAX_INGEST_BODY_BYTES: usize = 4 * 1024 * 1024;
const CATALOG_CACHE_CONTROL: &str = "public, max-age=15, stale-while-revalidate=60";

#[derive(Clone)]
enum CallerPrincipal {
    Owner,
    Device {
        id: String,
        enforces_worker_match: bool,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueueQuery {
    channel_id: Option<String>,
    status: Option<TelegramDeliveryStatus>,
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AlertListQuery {
    subscriber_id: String,
}

pub fn router(state: AppState) -> Router<AppState> {
    let device_or_owner_routes = Router::new()
        .route(
            "/v1/device/commerce/mercadolivre/ingest",
            post(device_ingest).layer(DefaultBodyLimit::max(MAX_INGEST_BODY_BYTES)),
        )
        .route(
            "/v1/device/commerce/mercadolivre/reference-catalog",
            get(device_reference_catalog),
        )
        .route(
            "/v1/commerce/mercadolivre/telegram-queue",
            get(telegram_queue),
        )
        .route(
            "/v1/commerce/mercadolivre/telegram-queue/{id}/ack",
            post(telegram_ack),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            device_or_owner_auth,
        ));

    let owner_routes = Router::new()
        .route(
            "/v1/commerce/mercadolivre/ingest",
            post(owner_ingest).layer(DefaultBodyLimit::max(MAX_INGEST_BODY_BYTES)),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            owner_only_auth,
        ));

    let catalog_routes = Router::new()
        .route("/v1/commerce/mercadolivre/site-catalog", get(site_catalog))
        .route("/v1/commerce/mercadolivre/taxonomy", get(taxonomy))
        .route(
            "/v1/commerce/mercadolivre/catalog-search",
            get(catalog_search),
        )
        .route(
            "/v1/commerce/mercadolivre/products/{id_or_slug}",
            get(product_detail),
        )
        .route(
            "/v1/commerce/mercadolivre/redirect/{id}",
            get(offer_redirect),
        )
        .route(
            "/v1/commerce/mercadolivre/telemetry",
            post(exposure_telemetry),
        )
        .route(
            "/v1/commerce/mercadolivre/price-alerts",
            post(handle_price_alert).get(list_price_alerts),
        )
        .route_layer(middleware::from_fn_with_state(state, catalog_auth));

    Router::new()
        .merge(device_or_owner_routes)
        .merge(owner_routes)
        .merge(catalog_routes)
}

async fn device_or_owner_auth(
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

async fn owner_only_auth(
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

async fn catalog_auth(
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

async fn device_ingest(
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

async fn owner_ingest(
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

fn map_service_err(err: zaturo_services::Error) -> ApiFailure {
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

async fn site_catalog(
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

async fn device_reference_catalog(
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

async fn taxonomy(
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

async fn catalog_search(
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

async fn product_detail(
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

async fn offer_redirect(
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

async fn exposure_telemetry(
    State(state): State<AppState>,
    Json(req): Json<ExposureBeaconRequest>,
) -> Result<Json<ExposureBeaconResponse>, ApiFailure> {
    let resp = state
        .engine
        .mercadolivre_record_exposure(req)
        .map_err(map_service_err)?;
    Ok(Json(resp))
}

async fn handle_price_alert(
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

async fn list_price_alerts(
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

async fn telegram_queue(
    State(state): State<AppState>,
    Query(query): Query<QueueQuery>,
) -> Result<Json<Vec<TelegramOfferQueueItem>>, ApiFailure> {
    let limit = query.limit.unwrap_or(20.min(state.config.max_page_size));
    let items = database(&state, move |e| {
        e.mercadolivre_telegram_queue(query.channel_id.as_deref(), query.status, limit)
    })
    .await?;
    Ok(Json(items))
}

async fn telegram_ack(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(ack): Json<TelegramDeliveryAck>,
) -> Result<Json<TelegramOfferQueueItem>, ApiFailure> {
    let item = database(&state, move |e| {
        e.ack_mercadolivre_telegram_delivery(&id, ack, now_ms())
    })
    .await?;
    Ok(Json(item))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HubConfig;
    use axum::http::Method;
    use sha2::{Digest, Sha256};
    use std::{fs, os::unix::fs::PermissionsExt};
    use tokio::task::JoinHandle;
    use zaturo_core::{
        CatalogTaxonomyResponse, MercadoLivreProductState, MercadoLivreSpec, MercadoLivreVariant,
        MinioProductImage, SiteCatalogSnapshot, ValidatedMercadoLivreProduct,
    };

    const OWNER: &str = "synthetic-owner-ml-hub-test-token-value";
    const DEVICE: &str = "synthetic-device-ml-hub-test-token-value";
    const READ_APP: &str = "synthetic-read-app-ml-hub-test-token-value";

    struct TestDir(std::path::PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("zaturo-hub-ml-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    struct HubGuard {
        _dir: TestDir,
        base: String,
        server: JoinHandle<()>,
    }

    impl Drop for HubGuard {
        fn drop(&mut self) {
            self.server.abort();
        }
    }

    async fn spawn_test_hub() -> HubGuard {
        let dir = TestDir::new();
        let path = &dir.0;

        let recipient =
            zaturo_transfer::crypto::generate_identity(&path.join("worker-ml.identity")).unwrap();
        let devices_toml = format!(
            "[[devices]]\nid=\"worker-ml\"\ntoken_sha256=\"{:x}\"\nencryption_recipient={recipient:?}\n",
            Sha256::digest(DEVICE)
        );
        fs::write(path.join("devices.toml"), devices_toml).unwrap();
        fs::set_permissions(path.join("devices.toml"), fs::Permissions::from_mode(0o600)).unwrap();

        let apps_toml = format!(
            "[[clients]]\nid=\"procurali-builder\"\ntoken_sha256=\"{:x}\"\ncan_submit=false\n",
            Sha256::digest(READ_APP)
        );
        fs::write(path.join("apps.toml"), apps_toml).unwrap();
        fs::set_permissions(path.join("apps.toml"), fs::Permissions::from_mode(0o600)).unwrap();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let config = HubConfig {
            bind: addr,
            client_url: format!("http://{addr}"),
            database: path.join("hub.db"),
            worker_id: "worker-ml".into(),
            max_connections: 4,
            max_queued_tasks: 16,
            max_page_size: 100,
            lease_seconds: 30,
            max_workers: 4,
            devices_file: Some(path.join("devices.toml")),
            apps_file: Some(path.join("apps.toml")),
            transfer_root: Some(path.join("spool")),
            transfer_limits: Default::default(),
        };
        let state = AppState::new(config, OWNER, DEVICE).unwrap();
        let app = crate::router(state);
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        HubGuard {
            _dir: dir,
            base: format!("http://{addr}"),
            server,
        }
    }

    fn large_product(product_id: &str, slug: &str) -> ValidatedMercadoLivreProduct {
        // Construct enough structured metadata so the full batch exceeds MAX_MESSAGE_BYTES (16 KiB)
        // and verifies the route-level 4 MiB body limit override.
        let mut specs = Vec::new();
        for idx in 0..40 {
            specs.push(MercadoLivreSpec {
                name: format!("Especificacao Tecnica {idx:02}"),
                value: format!(
                    "Valor conferido na ficha do fabricante #{idx:02} com margem detalhada."
                ),
            });
        }
        let padding_desc =
            "Controle termico estavel em carga continua sem throttling aparente; ".repeat(60);
        ValidatedMercadoLivreProduct {
            product_id: product_id.into(),
            slug: slug.into(),
            title: format!("Produto Teste Hardware {product_id}"),
            brand: "Kingston".into(),
            model: Some("SNV3S/500G".into()),
            exact_variant: "500 GB PCIe 4.0 M.2 2280".into(),
            category_slug: "hardware-e-perifericos".into(),
            canonical_url: format!("https://www.mercadolivre.com.br/item/p/{product_id}"),
            seller_name: "Loja Oficial".into(),
            share_link: "https://meli.la/1iWQZbJ".into(),
            share_id: "1PBCGF-SMN5".into(),
            share_text: "https://meli.la/1iWQZbJ".into(),
            commission_label: Some("5%".into()),
            commission_bp: Some(500),
            sponsored: false,
            price_brl_cents: 69_990,
            pix_price_cents: 69_990,
            reference_price_cents: Some(89_900),
            currency: "BRL".into(),
            marketplace_rating: Some("4,9 / 5".into()),
            marketplace_sales_label: Some("+10 mil vendidos".into()),
            card_badge: Some("Mais vendido".into()),
            discount_label: Some("22% OFF".into()),
            installment_summary: Some("10x sem juros".into()),
            payment_conditions: "A vista no Pix ou 10x sem juros".into(),
            short_condition: "10x sem juros · devolucao 30d".into(),
            freight_status: "Frete Full".into(),
            availability: "InStock".into(),
            price_valid_until: Some("2026-11-18".into()),
            return_days: Some(30),
            short_description: "SSD NVMe PCIe 4.0 x4 para upgrade rapido.".into(),
            long_description: padding_desc[..1500].to_owned(),
            one_line_reason: "Boa relacao entre leitura sequencial e preco observado.".into(),
            for_whom: "Notebooks e placas com slot M.2 Key M.".into(),
            supported_benefits: vec!["PCIe 4.0".into(), "M.2 2280".into()],
            limitations: vec!["Nao inclui dissipador.".into()],
            unknowns: vec!["Historico de 7 dias em acumulacao.".into()],
            tags: vec!["ssd".into(), "nvme".into()],
            specs,
            variants: vec![MercadoLivreVariant {
                variant_product_id: product_id.into(),
                sku: "MLB7607021422".into(),
                name: "SSD Kingston NV3 500GB".into(),
                color: Some("Azul".into()),
                voltage: None,
                price_brl_cents: 69_990,
                currency: "BRL".into(),
                availability: "InStock".into(),
                return_days: Some(30),
                return_fees: Some("Gratis".into()),
                canonical_url: Some(format!(
                    "https://www.mercadolivre.com.br/item/p/{product_id}"
                )),
                highlights: vec!["M.2 2280".into()],
            }],
            images: vec![MinioProductImage {
                position: 0,
                bucket: "procurali-media".into(),
                object_key: format!("mercadolivre/{product_id}/00-sample.jpg"),
                public_url: format!(
                    "https://media.procurali.com.br/procurali-media/mercadolivre/{product_id}/00-sample.jpg"
                ),
                source_origin_url: Some("https://http2.mlstatic.com/D_NQ_NP_sample.jpg".into()),
                content_type: "image/jpeg".into(),
                sha256_hex: "a".repeat(64),
                size_bytes: 2048,
                alt_text: "SSD Kingston NV3 500GB".into(),
            }],
            observed_at_ms: 1_763_231_280_000,
            freshness_ttl_minutes: 30,
            evidence_sha256: "b".repeat(64),
            state: MercadoLivreProductState::Approved,
        }
    }

    #[tokio::test]
    async fn hub_commerce_routes_enforce_auth_large_body_idempotency_catalog_and_telegram_ack() {
        let hub = spawn_test_hub().await;
        let client = reqwest::Client::builder().no_proxy().build().unwrap();

        let batch = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "ssd nvme".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: 1_763_231_280_000,
            products: vec![
                large_product("MLB39410080", "ssd-kingston-nv3-500gb-mlb39410080"),
                large_product("MLB53222888", "teclado-machenike-k500-b61-mlb53222888"),
                large_product("MLB48468943", "monitor-lg-ultragear-24-mlb48468943"),
            ],
        };
        let body_bytes = serde_json::to_vec(&batch).unwrap();
        assert!(
            body_bytes.len() > zaturo_protocol::MAX_MESSAGE_BYTES,
            "test batch should exceed MAX_MESSAGE_BYTES to exercise 4 MiB limit override"
        );

        let unauth = client
            .request(
                Method::POST,
                format!("{}/v1/device/commerce/mercadolivre/ingest", hub.base),
            )
            .header("Idempotency-Key", "ml-hub-test-1")
            .body(body_bytes.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

        let missing_idem = client
            .post(format!(
                "{}/v1/device/commerce/mercadolivre/ingest",
                hub.base
            ))
            .bearer_auth(DEVICE)
            .body(body_bytes.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(missing_idem.status(), StatusCode::BAD_REQUEST);

        let ok = client
            .post(format!(
                "{}/v1/device/commerce/mercadolivre/ingest",
                hub.base
            ))
            .bearer_auth(DEVICE)
            .header("Idempotency-Key", "ml-hub-test-1")
            .header("Content-Type", "application/json")
            .body(body_bytes.clone())
            .send()
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);
        let report: MercadoLivreIngestReport = ok.json().await.unwrap();
        assert_eq!(report.product_count, 3);
        assert_eq!(report.telegram_queued_count, 3);

        // Idempotent replay with identical bytes returns same report; conflicting payload returns 409.
        let replay = client
            .post(format!(
                "{}/v1/device/commerce/mercadolivre/ingest",
                hub.base
            ))
            .bearer_auth(DEVICE)
            .header("Idempotency-Key", "ml-hub-test-1")
            .body(body_bytes)
            .send()
            .await
            .unwrap();
        assert_eq!(replay.status(), StatusCode::OK);
        assert_eq!(
            replay.json::<MercadoLivreIngestReport>().await.unwrap(),
            report
        );

        let mut conflict_batch = batch.clone();
        conflict_batch.search_query = "perifericos gamers".into();
        let conflict = client
            .post(format!("{}/v1/commerce/mercadolivre/ingest", hub.base))
            .bearer_auth(OWNER)
            .header("Idempotency-Key", "ml-hub-test-1")
            .json(&conflict_batch)
            .send()
            .await
            .unwrap();
        assert_eq!(conflict.status(), StatusCode::CONFLICT);

        // Read-only app can read site catalog; device token cannot.
        let device_catalog = client
            .get(format!(
                "{}/v1/commerce/mercadolivre/site-catalog",
                hub.base
            ))
            .bearer_auth(DEVICE)
            .send()
            .await
            .unwrap();
        assert_eq!(device_catalog.status(), StatusCode::UNAUTHORIZED);

        let catalog_resp = client
            .get(format!(
                "{}/v1/commerce/mercadolivre/site-catalog",
                hub.base
            ))
            .bearer_auth(READ_APP)
            .send()
            .await
            .unwrap();
        assert_eq!(catalog_resp.status(), StatusCode::OK);
        let catalog: SiteCatalogSnapshot = catalog_resp.json().await.unwrap();
        assert_eq!(catalog.offers.len(), 3);
        assert_eq!(catalog.offers[0].specs.len(), 40);

        // Read-only app cannot access telegram queue; device token can list and ack.
        let read_queue = client
            .get(format!(
                "{}/v1/commerce/mercadolivre/telegram-queue",
                hub.base
            ))
            .bearer_auth(READ_APP)
            .send()
            .await
            .unwrap();
        assert_eq!(read_queue.status(), StatusCode::UNAUTHORIZED);

        let queue_resp = client
            .get(format!(
                "{}/v1/commerce/mercadolivre/telegram-queue?channel_id=%40procurali&status=queued&limit=10",
                hub.base
            ))
            .bearer_auth(DEVICE)
            .send()
            .await
            .unwrap();
        assert_eq!(queue_resp.status(), StatusCode::OK);
        let items: Vec<TelegramOfferQueueItem> = queue_resp.json().await.unwrap();
        assert_eq!(items.len(), 3);

        let ack_resp = client
            .post(format!(
                "{}/v1/commerce/mercadolivre/telegram-queue/{}/ack",
                hub.base, items[0].delivery_id
            ))
            .bearer_auth(DEVICE)
            .json(&TelegramDeliveryAck {
                status: TelegramDeliveryStatus::Published,
                external_message_id: Some("tg-1001".into()),
                error_code: None,
            })
            .send()
            .await
            .unwrap();
        assert_eq!(ack_resp.status(), StatusCode::OK);
        let acked: TelegramOfferQueueItem = ack_resp.json().await.unwrap();
        assert_eq!(acked.status, TelegramDeliveryStatus::Published);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn cloobe_taxonomy_search_facets_sparkline_alerts_redirect_and_high_concurrency_benchmark()
     {
        let hub = spawn_test_hub().await;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();

        let mut products = Vec::with_capacity(24);
        for idx in 0..24u32 {
            let pid = format!("MLB39410{:03}", 100 + idx);
            let title = match idx % 4 {
                0 => format!("SSD Kingston NV3 1TB NVMe PCIe 4.0 Lote {idx}"),
                1 => format!("Monitor Gamer LG UltraGear 27 IPS 200Hz Lote {idx}"),
                2 => format!("Teclado Mecanico Redragon Kumara Switch Blue Lote {idx}"),
                _ => format!("Headset Gamer Redragon Zeus Lite 53mm Lote {idx}"),
            };
            let mut p = large_product(&pid, &zaturo_core::slugify_pt_br(&title, &pid));
            p.title = title;
            p.long_description =
                "Produto conferido no catálogo oficial com garantia nacional.".into();
            p.specs.truncate(6);
            p.price_brl_cents = 15_000 + i64::from(idx) * 2_500;
            p.pix_price_cents = p.price_brl_cents;
            p.reference_price_cents = Some(p.price_brl_cents * 2);
            p.discount_label = Some("50% OFF".into());
            products.push(p);
        }

        let batch = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "hardware e perifericos".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: 1_763_231_280_000,
            products,
        };
        let ingest_resp = client
            .post(format!(
                "{}/v1/device/commerce/mercadolivre/ingest",
                hub.base
            ))
            .bearer_auth(DEVICE)
            .header("Idempotency-Key", "cloobe-bench-seed-1")
            .json(&batch)
            .send()
            .await
            .unwrap();
        assert_eq!(ingest_resp.status(), StatusCode::OK);

        // 1) Taxonomy tree endpoint
        let tax_resp = client
            .get(format!("{}/v1/commerce/mercadolivre/taxonomy", hub.base))
            .bearer_auth(READ_APP)
            .send()
            .await
            .unwrap();
        assert_eq!(tax_resp.status(), StatusCode::OK);
        let tax_etag = tax_resp
            .headers()
            .get(ETAG)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let tax: CatalogTaxonomyResponse = tax_resp.json().await.unwrap();
        assert_eq!(tax.categories[0].offer_count, 24);
        assert!(tax.categories[0].subcategories.len() >= 4);

        // ETag conditional GET returns 304 Not Modified cleanly
        let not_mod = client
            .get(format!("{}/v1/commerce/mercadolivre/taxonomy", hub.base))
            .bearer_auth(READ_APP)
            .header(IF_NONE_MATCH, tax_etag)
            .send()
            .await
            .unwrap();
        assert_eq!(not_mod.status(), StatusCode::NOT_MODIFIED);

        // 2) Faceted search + HTML fragment output
        let search_resp = client
            .get(format!(
                "{}/v1/commerce/mercadolivre/catalog-search?q=monitor&subcategoria=monitores&ordem=preco-asc&desconto=20&formato=html-fragment&por_pagina=4",
                hub.base
            ))
            .bearer_auth(READ_APP)
            .send()
            .await
            .unwrap();
        assert_eq!(search_resp.status(), StatusCode::OK);
        let search: CatalogSearchResponse = search_resp.json().await.unwrap();
        assert_eq!(search.total_matches, 6);
        assert_eq!(search.items.len(), 4);
        assert!(search.items[0].price_brl_cents <= search.items[1].price_brl_cents);
        assert!(
            search
                .html_fragment
                .as_deref()
                .unwrap()
                .contains("<article class=\"offer-card\"")
        );

        // 3) Product detail (PDP) with SVG sparkline, 30d price intel, FAQ, and similar products
        let pdp_resp = client
            .get(format!(
                "{}/v1/commerce/mercadolivre/products/MLB39410101",
                hub.base
            ))
            .bearer_auth(READ_APP)
            .send()
            .await
            .unwrap();
        assert_eq!(pdp_resp.status(), StatusCode::OK);
        let pdp: ProductDetailResponse = pdp_resp.json().await.unwrap();
        let intel = pdp.offer.price_intelligence.as_ref().unwrap();
        assert_eq!(intel.window_days, 30);
        assert!(intel.svg_sparkline.contains("<svg viewBox=\"0 0 300 80\""));
        assert!(!pdp.similar_offers.is_empty());
        assert!(!pdp.offer.dynamic_faq.is_empty());

        // 4) Redirect tracking + Exposure telemetry + Price Alerts
        let redir_resp = client
            .get(format!(
                "{}/v1/commerce/mercadolivre/redirect/MLB39410101",
                hub.base
            ))
            .bearer_auth(READ_APP)
            .send()
            .await
            .unwrap();
        assert_eq!(redir_resp.status(), StatusCode::FOUND);
        assert_eq!(
            redir_resp
                .headers()
                .get(LOCATION)
                .unwrap()
                .to_str()
                .unwrap(),
            "https://meli.la/1iWQZbJ"
        );

        let tel_resp = client
            .post(format!("{}/v1/commerce/mercadolivre/telemetry", hub.base))
            .bearer_auth(READ_APP)
            .json(&ExposureBeaconRequest {
                route: "/categorias/hardware-e-perifericos/monitores/".into(),
                events: vec![zaturo_core::ExposureEventItem {
                    product_id: "MLB39410101".into(),
                    store: "mercadolivre".into(),
                    pos: 1,
                    discount_pct: 50,
                    price_cents: 17_500,
                }],
                timestamp_ms: 1_763_231_300_000,
            })
            .send()
            .await
            .unwrap();
        assert_eq!(tel_resp.status(), StatusCode::OK);

        let alert_resp = client
            .post(format!(
                "{}/v1/commerce/mercadolivre/price-alerts",
                hub.base
            ))
            .bearer_auth(READ_APP)
            .json(&PriceAlertActionRequest {
                action: "criar-alerta".into(),
                subscriber_id: "user_bench_01".into(),
                product_id: Some("MLB39410101".into()),
                target_price_cents: Some(20_000),
            })
            .send()
            .await
            .unwrap();
        assert_eq!(alert_resp.status(), StatusCode::OK);
        let alert: PriceAlertResponse = alert_resp.json().await.unwrap();
        assert_eq!(alert.triggered.len(), 1);

        let pdp_after: ProductDetailResponse = client
            .get(format!(
                "{}/v1/commerce/mercadolivre/products/MLB39410101",
                hub.base
            ))
            .bearer_auth(READ_APP)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(pdp_after.clicks, 1);
        assert_eq!(pdp_after.impressions, 1);
        assert_eq!(pdp_after.alert_count, 1);

        // 5) High-concurrency benchmark: 2,400 concurrent requests across 60 parallel workers.
        let concurrency: usize = 60;
        let per_worker: usize = 40;
        let total_requests: usize = concurrency * per_worker;
        let shared_client = std::sync::Arc::new(client);
        let base_url = std::sync::Arc::new(hub.base.clone());
        let bench_start = Instant::now();
        let mut handles = Vec::with_capacity(concurrency);
        for w in 0..concurrency {
            let c = shared_client.clone();
            let b = base_url.clone();
            handles.push(tokio::spawn(async move {
                let mut latencies_us = Vec::with_capacity(per_worker);
                for req_idx in 0..per_worker {
                    let t0 = Instant::now();
                    let url = match (w + req_idx) % 4 {
                        0 => format!("{b}/v1/commerce/mercadolivre/site-catalog"),
                        1 => format!(
                            "{b}/v1/commerce/mercadolivre/catalog-search?q=ssd&ordem=preco-asc"
                        ),
                        2 => format!("{b}/v1/commerce/mercadolivre/products/MLB39410100"),
                        _ => format!("{b}/v1/commerce/mercadolivre/taxonomy"),
                    };
                    let resp = c.get(url).bearer_auth(READ_APP).send().await.unwrap();
                    assert_eq!(resp.status(), StatusCode::OK);
                    let _ = resp.bytes().await.unwrap();
                    latencies_us.push(u64::try_from(t0.elapsed().as_micros()).unwrap_or(u64::MAX));
                }
                latencies_us
            }));
        }

        let mut all_latencies = Vec::with_capacity(total_requests);
        for h in handles {
            all_latencies.extend(h.await.unwrap());
        }
        let elapsed = bench_start.elapsed();
        all_latencies.sort_unstable();
        let rps = total_requests as f64 / elapsed.as_secs_f64();
        let avg_ms = (all_latencies.iter().sum::<u64>() as f64 / total_requests as f64) / 1000.0;
        let p99_ms = all_latencies[(total_requests * 99) / 100] as f64 / 1000.0;
        eprintln!(
            "BENCHMARK hub_commerce: {total_requests} requests in {:.3}s => {rps:.1} req/s | avg={avg_ms:.2}ms | p99={p99_ms:.2}ms",
            elapsed.as_secs_f64()
        );
        let min_rps = if cfg!(debug_assertions) {
            500.0
        } else {
            1_000.0
        };
        assert!(
            rps > min_rps,
            "expected > {min_rps:.0} req/s warm throughput, got {rps:.1}"
        );
    }

    #[tokio::test]
    async fn device_reference_catalog_supports_etag_304_and_tracks_price_drops() {
        let hub = spawn_test_hub().await;
        let client = reqwest::Client::new();
        let ref_url = format!(
            "{}/v1/device/commerce/mercadolivre/reference-catalog",
            hub.base
        );

        // Unauthenticated or READ_APP token must be rejected on device route.
        let unauth = client.get(&ref_url).send().await.unwrap();
        assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);
        let wrong_token = client
            .get(&ref_url)
            .bearer_auth(READ_APP)
            .send()
            .await
            .unwrap();
        assert_eq!(wrong_token.status(), StatusCode::UNAUTHORIZED);

        // Day 1 ingest with DEVICE token
        let day1_ms = 1_763_200_000_000i64;
        let mut p1 = large_product("MLB39410200", "ssd-nv3-ref-test-mlb39410200");
        p1.price_brl_cents = 69_990;
        p1.pix_price_cents = 69_990;
        p1.observed_at_ms = day1_ms;
        let ingest1 = client
            .post(format!(
                "{}/v1/device/commerce/mercadolivre/ingest",
                hub.base
            ))
            .bearer_auth(DEVICE)
            .header("Idempotency-Key", "ref-batch-1")
            .json(&zaturo_core::MercadoLivreIngestBatchRequest {
                worker_id: "worker-ml".into(),
                search_query: "ssd".into(),
                channel_id: "@procurali".into(),
                captured_at_ms: day1_ms,
                products: vec![p1],
            })
            .send()
            .await
            .unwrap();
        assert_eq!(ingest1.status(), StatusCode::OK);
        let report1: zaturo_core::MercadoLivreIngestReport = ingest1.json().await.unwrap();
        assert_eq!(report1.new_product_count, 1);

        // Device fetches reference catalog and gets ETag + item summary
        let resp1 = client
            .get(&ref_url)
            .bearer_auth(DEVICE)
            .send()
            .await
            .unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);
        let etag1 = resp1
            .headers()
            .get(ETAG)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let ref1: zaturo_core::CommerceReferenceCatalog = resp1.json().await.unwrap();
        assert_eq!(ref1.total_products, 1);
        assert_eq!(ref1.items[0].product_id, "MLB39410200");
        assert_eq!(ref1.items[0].price_trend, zaturo_core::PriceTrendKind::New);

        // Conditional If-None-Match returns 304 Not Modified without body
        let not_modified = client
            .get(&ref_url)
            .bearer_auth(DEVICE)
            .header(IF_NONE_MATCH, &etag1)
            .send()
            .await
            .unwrap();
        assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);

        // Day 2 ingest with lower price
        let day2_ms = day1_ms + 86_400_000;
        let mut p2 = large_product("MLB39410200", "ssd-nv3-ref-test-mlb39410200");
        p2.price_brl_cents = 54_990;
        p2.pix_price_cents = 54_990;
        p2.observed_at_ms = day2_ms;
        let ingest2 = client
            .post(format!(
                "{}/v1/device/commerce/mercadolivre/ingest",
                hub.base
            ))
            .bearer_auth(DEVICE)
            .header("Idempotency-Key", "ref-batch-2")
            .json(&zaturo_core::MercadoLivreIngestBatchRequest {
                worker_id: "worker-ml".into(),
                search_query: "ssd".into(),
                channel_id: "@procurali".into(),
                captured_at_ms: day2_ms,
                products: vec![p2],
            })
            .send()
            .await
            .unwrap();
        assert_eq!(ingest2.status(), StatusCode::OK);
        let report2: zaturo_core::MercadoLivreIngestReport = ingest2.json().await.unwrap();
        assert_eq!(report2.price_drop_count, 1);
        assert_eq!(report2.comparisons[0].previous_price_cents, Some(69_990));
        assert_eq!(report2.comparisons[0].price_delta_cents, -15_000);

        let resp2 = client
            .get(&ref_url)
            .bearer_auth(DEVICE)
            .header(IF_NONE_MATCH, &etag1)
            .send()
            .await
            .unwrap();
        assert_eq!(resp2.status(), StatusCode::OK);
        let ref2: zaturo_core::CommerceReferenceCatalog = resp2.json().await.unwrap();
        assert_eq!(ref2.items[0].price_trend, zaturo_core::PriceTrendKind::Down);
        assert_eq!(ref2.items[0].previous_price_cents, Some(69_990));
        assert!(ref2.items[0].short_summary.contains("Queda"));
    }
}
