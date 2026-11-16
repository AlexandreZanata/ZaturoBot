//! Authenticated HTTP composition for Mercado Livre commerce ingestion, static-site catalog
//! export, and Telegram delivery queue orchestration.

use crate::{
    AppState,
    http::{ApiFailure, bearer, database},
    now_ms,
};
use axum::{
    Extension, Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use zaturo_core::{
    MercadoLivreIngestBatchRequest, MercadoLivreIngestReport, SiteCatalogSnapshot,
    TelegramDeliveryAck, TelegramDeliveryStatus, TelegramOfferQueueItem,
};

const MAX_INGEST_BODY_BYTES: usize = 4 * 1024 * 1024;

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

pub fn router(state: AppState) -> Router<AppState> {
    let device_or_owner_routes = Router::new()
        .route(
            "/v1/device/commerce/mercadolivre/ingest",
            post(device_ingest).layer(DefaultBodyLimit::max(MAX_INGEST_BODY_BYTES)),
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

async fn site_catalog(
    State(state): State<AppState>,
) -> Result<Json<SiteCatalogSnapshot>, ApiFailure> {
    let snapshot = database(&state, move |e| e.mercadolivre_site_catalog(now_ms())).await?;
    Ok(Json(snapshot))
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
        MercadoLivreProductState, MercadoLivreSpec, MercadoLivreVariant, MinioProductImage,
        ValidatedMercadoLivreProduct,
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
}
