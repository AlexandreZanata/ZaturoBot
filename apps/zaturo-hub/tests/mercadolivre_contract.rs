//! Independent black-box HTTP contract tests for `zaturo_hub` Mercado Livre routes.
//!
//! Spawns the real Axum router over TCP loopback and validates auth boundaries, ETag caching,
//! ADR-0025 automatic Telegram queuing for Approved batches, ADR-0024 owner approval gate,
//! pre-send claim -> published ack transitions, search/PDP/redirect/telemetry/alert routes,
//! and reference-catalog conditional GET (`304 Not Modified`).

use axum::http::StatusCode;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use tokio::task::JoinHandle;
use zaturo_core::{
    CatalogSearchResponse, CatalogTaxonomyResponse, CommerceReferenceCatalog,
    ExposureBeaconResponse, MercadoLivreIngestBatchRequest, MercadoLivreIngestReport,
    MercadoLivreProductState, MercadoLivreSpec, MercadoLivreVariant, MinioProductImage,
    PriceAlertResponse, ProductDetailResponse, SiteCatalogSnapshot, TelegramDeliveryAck,
    TelegramDeliveryStatus, TelegramOfferQueueItem, ValidatedMercadoLivreProduct, slugify_pt_br,
};
use zaturo_hub::{AppState, config::HubConfig, now_ms, router};

const OWNER: &str = "synthetic-owner-contract-token-value";
const DEVICE: &str = "synthetic-device-contract-token-value";
const READ_APP: &str = "synthetic-reader-contract-token-value";

struct TestDir(std::path::PathBuf);
impl TestDir {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!("zaturo-hub-contract-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        Self(p)
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

async fn spawn_contract_hub() -> HubGuard {
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
    let app = router(state);
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    HubGuard {
        _dir: dir,
        base: format!("http://{addr}"),
        server,
    }
}

fn product_payload(
    product_id: &str,
    price_cents: i64,
    state: MercadoLivreProductState,
    observed_at_ms: i64,
) -> ValidatedMercadoLivreProduct {
    let title = format!("Teclado Mecanico Redragon {product_id}");
    ValidatedMercadoLivreProduct {
        product_id: product_id.into(),
        slug: slugify_pt_br(&title, product_id),
        title,
        brand: "Redragon".into(),
        model: Some("Kumara K552".into()),
        exact_variant: "Switch Outemu Brown ABNT2".into(),
        category_slug: "hardware-e-perifericos".into(),
        canonical_url: format!("https://www.mercadolivre.com.br/teclado/p/{product_id}"),
        seller_name: "Loja Oficial Redragon".into(),
        share_link: "https://meli.la/22sq34s".into(),
        share_id: "1PBCGF-SMN5".into(),
        share_text: "https://meli.la/22sq34s".into(),
        commission_label: Some("5%".into()),
        commission_bp: Some(500),
        sponsored: false,
        price_brl_cents: price_cents,
        pix_price_cents: price_cents,
        reference_price_cents: Some(29_900),
        currency: "BRL".into(),
        marketplace_rating: Some("4,8 / 5".into()),
        marketplace_sales_label: Some("+10 mil vendidos".into()),
        card_badge: Some("Mais vendido".into()),
        discount_label: Some("30% OFF".into()),
        installment_summary: Some("6x sem juros".into()),
        payment_conditions: "6x sem juros ou Pix".into(),
        short_condition: "6x sem juros".into(),
        freight_status: "Frete Full".into(),
        availability: "InStock".into(),
        price_valid_until: Some("2026-12-01".into()),
        return_days: Some(30),
        short_description: "Teclado TKL mecanico ABNT2.".into(),
        long_description: "Layout compacto TKL para produtividade e jogos.".into(),
        one_line_reason: "Modelo consolidado em custo-beneficio ABNT2.".into(),
        for_whom: "Quem quer switch tatil compacto ABNT2.".into(),
        supported_benefits: vec!["Layout ABNT2".into()],
        limitations: vec!["Sem apoio de pulso incluso.".into()],
        unknowns: vec![],
        tags: vec!["teclado".into(), "mecanico".into()],
        specs: vec![MercadoLivreSpec {
            name: "Layout".into(),
            value: "ABNT2".into(),
        }],
        variants: vec![MercadoLivreVariant {
            variant_product_id: product_id.into(),
            sku: "K552-BR".into(),
            name: "Preto".into(),
            color: Some("Preto".into()),
            voltage: None,
            price_brl_cents: price_cents,
            currency: "BRL".into(),
            availability: "InStock".into(),
            return_days: Some(30),
            return_fees: Some("Grátis".into()),
            canonical_url: None,
            highlights: vec!["Switch Brown".into()],
        }],
        images: vec![MinioProductImage {
            position: 0,
            bucket: "procurali-media".into(),
            object_key: format!("mercadolivre/{product_id}/00-aaa.jpg"),
            public_url: format!(
                "https://media.procurali.com.br/procurali-media/mercadolivre/{product_id}/00-aaa.jpg"
            ),
            source_origin_url: None,
            content_type: "image/jpeg".into(),
            sha256_hex: "1".repeat(64),
            size_bytes: 18_000,
            alt_text: "Teclado Redragon Kumara".into(),
        }],
        observed_at_ms,
        freshness_ttl_minutes: 30,
        evidence_sha256: "2".repeat(64),
        state,
    }
}

#[tokio::test]
async fn hub_http_contract_enforces_auth_auto_queues_approved_and_serves_catalog_apis() {
    let hub = spawn_contract_hub().await;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let ts = now_ms().saturating_sub(2_000).max(1);

    let batch = MercadoLivreIngestBatchRequest {
        worker_id: "worker-ml".into(),
        search_query: "teclado mecanico".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: ts,
        products: vec![
            product_payload(
                "MLB51383856",
                19_990,
                MercadoLivreProductState::Approved,
                ts,
            ),
            product_payload(
                "MLB51383857",
                21_990,
                MercadoLivreProductState::Candidate,
                ts,
            ),
        ],
    };

    // Unauthenticated ingest rejected.
    let unauth = client
        .post(format!(
            "{}/v1/device/commerce/mercadolivre/ingest",
            hub.base
        ))
        .header("Idempotency-Key", "hub-contract-1")
        .json(&batch)
        .send()
        .await
        .unwrap();
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    // Authenticated device ingest succeeds and auto-queues the Approved offer.
    let ingest_res = client
        .post(format!(
            "{}/v1/device/commerce/mercadolivre/ingest",
            hub.base
        ))
        .bearer_auth(DEVICE)
        .header("Idempotency-Key", "hub-contract-1")
        .json(&batch)
        .send()
        .await
        .unwrap();
    assert_eq!(ingest_res.status(), StatusCode::OK);
    let report: MercadoLivreIngestReport = ingest_res.json().await.unwrap();
    assert_eq!(report.telegram_queued_count, 1);
    assert_eq!(report.telegram_awaiting_approval_count, 1);

    // Device token CANNOT approve Candidate offers (owner-only route).
    let device_approve = client
        .post(format!(
            "{}/v1/commerce/mercadolivre/telegram-queue/approve",
            hub.base
        ))
        .bearer_auth(DEVICE)
        .json(&json!({"product_id": "MLB51383857", "channel_id": "@procurali"}))
        .send()
        .await
        .unwrap();
    assert_eq!(device_approve.status(), StatusCode::UNAUTHORIZED);

    // Owner token CAN approve Candidate offer.
    let owner_approve = client
        .post(format!(
            "{}/v1/commerce/mercadolivre/telegram-queue/approve",
            hub.base
        ))
        .bearer_auth(OWNER)
        .json(&json!({"product_id": "MLB51383857", "channel_id": "@procurali"}))
        .send()
        .await
        .unwrap();
    assert_eq!(owner_approve.status(), StatusCode::OK);

    // Worker fetches queued deliveries, claims `sending`, then acks `published`.
    let queue_items: Vec<TelegramOfferQueueItem> = client
        .get(format!(
            "{}/v1/commerce/mercadolivre/telegram-queue?status=queued",
            hub.base
        ))
        .bearer_auth(DEVICE)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(queue_items.len(), 2);

    let first_id = queue_items
        .iter()
        .find(|i| i.product_id == "MLB51383856")
        .unwrap()
        .delivery_id
        .clone();

    let claim_resp: TelegramOfferQueueItem = client
        .post(format!(
            "{}/v1/commerce/mercadolivre/telegram-queue/{first_id}/ack",
            hub.base
        ))
        .bearer_auth(DEVICE)
        .json(&TelegramDeliveryAck {
            status: TelegramDeliveryStatus::Sending,
            external_message_id: None,
            error_code: None,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(claim_resp.status, TelegramDeliveryStatus::Sending);

    let pub_resp: TelegramOfferQueueItem = client
        .post(format!(
            "{}/v1/commerce/mercadolivre/telegram-queue/{first_id}/ack",
            hub.base
        ))
        .bearer_auth(DEVICE)
        .json(&TelegramDeliveryAck {
            status: TelegramDeliveryStatus::Published,
            external_message_id: Some("9001".into()),
            error_code: None,
        })
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(pub_resp.status, TelegramDeliveryStatus::Published);

    // Catalog app credential reads site-catalog, taxonomy, search, detail, telemetry, alerts, redirect.
    let catalog_res = client
        .get(format!(
            "{}/v1/commerce/mercadolivre/site-catalog",
            hub.base
        ))
        .bearer_auth(READ_APP)
        .send()
        .await
        .unwrap();
    assert_eq!(catalog_res.status(), StatusCode::OK);
    let catalog: SiteCatalogSnapshot = catalog_res.json().await.unwrap();
    assert_eq!(catalog.offers.len(), 2);

    let taxonomy: CatalogTaxonomyResponse = client
        .get(format!("{}/v1/commerce/mercadolivre/taxonomy", hub.base))
        .bearer_auth(READ_APP)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(!taxonomy.categories.is_empty());

    let search: CatalogSearchResponse = client
        .get(format!(
            "{}/v1/commerce/mercadolivre/catalog-search?q=redragon",
            hub.base
        ))
        .bearer_auth(READ_APP)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(search.total_matches, 2);

    let detail: ProductDetailResponse = client
        .get(format!(
            "{}/v1/commerce/mercadolivre/products/MLB51383856",
            hub.base
        ))
        .bearer_auth(READ_APP)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(detail.offer.product_id, "MLB51383856");

    let beacon: ExposureBeaconResponse = client
        .post(format!("{}/v1/commerce/mercadolivre/telemetry", hub.base))
        .bearer_auth(READ_APP)
        .json(&json!({
            "route": "/ofertas/",
            "events": [{
                "productId": "MLB51383856",
                "store": "Mercado Livre",
                "pos": 0,
                "discountPct": 30,
                "priceCents": 19990
            }],
            "timestampMs": ts
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(beacon.accepted, 1);

    let alert: PriceAlertResponse = client
        .post(format!(
            "{}/v1/commerce/mercadolivre/price-alerts",
            hub.base
        ))
        .bearer_auth(READ_APP)
        .json(&json!({
            "action": "criar-alerta",
            "subscriberId": "contract-sub",
            "productId": "MLB51383856",
            "targetPriceCents": 22000
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(alert.triggered.len(), 1);

    let redirect_res = client
        .get(format!(
            "{}/v1/commerce/mercadolivre/redirect/MLB51383856",
            hub.base
        ))
        .bearer_auth(READ_APP)
        .send()
        .await
        .unwrap();
    assert_eq!(redirect_res.status(), StatusCode::FOUND);
    assert_eq!(
        redirect_res.headers().get("cache-control").unwrap(),
        "no-store"
    );
    assert_eq!(
        redirect_res.headers().get("location").unwrap(),
        "https://meli.la/22sq34s"
    );

    // Device reference-catalog returns ETag and 304 Not Modified on conditional GET.
    let ref1 = client
        .get(format!(
            "{}/v1/device/commerce/mercadolivre/reference-catalog",
            hub.base
        ))
        .bearer_auth(DEVICE)
        .send()
        .await
        .unwrap();
    assert_eq!(ref1.status(), StatusCode::OK);
    let etag = ref1
        .headers()
        .get("etag")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let ref_body: CommerceReferenceCatalog = ref1.json().await.unwrap();
    assert_eq!(ref_body.total_products, 2);

    let ref2 = client
        .get(format!(
            "{}/v1/device/commerce/mercadolivre/reference-catalog",
            hub.base
        ))
        .bearer_auth(DEVICE)
        .header("If-None-Match", etag)
        .send()
        .await
        .unwrap();
    assert_eq!(ref2.status(), StatusCode::NOT_MODIFIED);
}
