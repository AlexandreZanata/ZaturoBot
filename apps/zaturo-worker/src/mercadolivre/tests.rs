use super::*;
use axum::{
    Router,
    body::Bytes,
    extract::{Path as AxumPath, State as AxumState},
    http::{HeaderMap, StatusCode},
    routing::put,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    os::unix::fs::PermissionsExt,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;
use zaturo_core::{SiteCatalogSnapshot, TelegramOfferQueueItem};
use zaturo_hub::{AppState, config::HubConfig};
use zaturo_transfer::minio::MinioConfig;

const OWNER: &str = "synthetic-owner-e2e-ml-token-value";
const DEVICE: &str = "synthetic-device-e2e-ml-token-value";
const READ_APP: &str = "synthetic-read-app-e2e-ml-token-value";

fn valid_png_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    out.extend_from_slice(&13u32.to_be_bytes());
    out.extend_from_slice(b"IHDR");
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&[8, 2, 0, 0, 0]);
    out.extend_from_slice(&[0u8; 4]);
    out
}

#[derive(Clone, Default)]
struct MockMinioState {
    buckets: Arc<Mutex<BTreeSet<String>>>,
    objects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

async fn mock_head_bucket(
    AxumState(state): AxumState<MockMinioState>,
    AxumPath(bucket): AxumPath<String>,
    headers: HeaderMap,
) -> StatusCode {
    if headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_none()
    {
        return StatusCode::FORBIDDEN;
    }
    if state.buckets.lock().unwrap().contains(&bucket) {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn mock_put_bucket(
    AxumState(state): AxumState<MockMinioState>,
    AxumPath(bucket): AxumPath<String>,
    headers: HeaderMap,
) -> StatusCode {
    let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) else {
        return StatusCode::FORBIDDEN;
    };
    if !auth.starts_with("AWS4-HMAC-SHA256 ") {
        return StatusCode::FORBIDDEN;
    }
    state.buckets.lock().unwrap().insert(bucket);
    StatusCode::OK
}

async fn mock_put_object(
    AxumState(state): AxumState<MockMinioState>,
    AxumPath((bucket, key)): AxumPath<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) else {
        return StatusCode::FORBIDDEN;
    };
    if !auth.starts_with("AWS4-HMAC-SHA256 ") || body.is_empty() {
        return StatusCode::BAD_REQUEST;
    }
    state
        .objects
        .lock()
        .unwrap()
        .insert(format!("{bucket}/{key}"), body.to_vec());
    StatusCode::OK
}

struct AbortOnDrop(JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
async fn end_to_end_ingests_four_real_ml_products_to_minio_and_hub_catalog_and_telegram() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Prepare mock local photos matching the 4 real ML hardware products, including a 9:16
    // vertical reel thumbnail next to a square product shot for SSD Kingston NV3.
    let photos_dir = root.join("photos");
    fs::create_dir_all(&photos_dir).unwrap();
    let nv3_vertical = photos_dir.join("nv3_reel.png");
    let nv3_square = photos_dir.join("nv3_box.png");
    let k500_square = photos_dir.join("k500.png");
    let lg_square = photos_dir.join("lg24.png");
    let fuxi_square = photos_dir.join("fuxi.png");

    fs::write(&nv3_vertical, valid_png_bytes(720, 1280)).unwrap();
    fs::write(&nv3_square, valid_png_bytes(1080, 1080)).unwrap();
    fs::write(&k500_square, valid_png_bytes(1000, 1000)).unwrap();
    fs::write(&lg_square, valid_png_bytes(1200, 900)).unwrap();
    fs::write(&fuxi_square, valid_png_bytes(960, 960)).unwrap();

    let prod_nv3 = json!({
        "product_id": "MLB39410080",
        "title": "SSD Kingston NV3 500GB M.2 2280 NVMe PCIe 4.0",
        "brand": "Kingston",
        "model": "SNV3S/500G",
        "product_url": "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/MLB39410080?matt_tool=99999999",
        "canonical_url": "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/MLB39410080",
        "image_url": "https://http2.mlstatic.com/D_NQ_NP_nv3.jpg",
        "commission": "5%",
        "sponsored": false,
        "card_text": "MAIS VENDIDO\n4.9 | +10mil vendidos\nR$\n1.379\n,\n00\nR$\n699\n,\n90\n49% OFF\n18x\nR$\n46\n,\n26\n",
        "share_link": "https://meli.la/1iWQZbJ",
        "share_id": "1PBCGF-SMN5",
        "share_text": "Código 1PBCGF-SMN5\nhttps://meli.la/1iWQZbJ",
        "price": 699.90,
        "meta_price": "699.90",
        "currency": "BRL",
        "availability": "https://schema.org/InStock",
        "price_valid_until": "2026-11-18",
        "return_days": 30,
        "seller": "Loja Oficial Kingston",
        "short_description": "SSD M.2 2280 PCIe 4.0 x4 com leitura sequencial de até 5.000 MB/s.",
        "long_description": "Unidade NV3 M.2 2280 para desktops e notebooks modernos.",
        "specs": [
            {"name": "Marca", "value": "Kingston"},
            {"name": "Capacidade", "value": "500 GB"},
            {"name": "Interface", "value": "PCIe 4.0 NVMe"}
        ],
        "variants": [
            {
                "name": "SSD Kingston NV3 500GB Azul-aço",
                "sku": "MLB7607021422",
                "productID": "MLB39410080",
                "color": "Azul-aço",
                "offers": {
                    "price": 698.00,
                    "priceCurrency": "BRL",
                    "availability": "https://schema.org/InStock",
                    "url": "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/MLB39410080",
                    "hasMerchantReturnPolicy": {"merchantReturnDays": 30, "returnFees": "https://schema.org/FreeReturn"}
                }
            }
        ],
        "gallery": [
            "https://http2.mlstatic.com/D_NQ_NP_nv3_reel.jpg",
            "https://http2.mlstatic.com/D_NQ_NP_nv3.jpg"
        ],
        "photos": ["photos/nv3_reel.png", "photos/nv3_box.png"]
    });
    let prod_k500 = json!({
        "product_id": "MLB53222888",
        "title": "Teclado Mecânico Gamer Machenike K500-B61 Switch Red",
        "brand": "Machenike",
        "model": "K500-B61",
        "canonical_url": "https://www.mercadolivre.com.br/teclado-machenike-k500-b61/p/MLB53222888",
        "commission": "11%",
        "card_text": "OFERTA DO DIA\n4.8 | +5mil vendidos\nR$\n199\n,\n90\nR$\n138\n,\n79\n30% OFF no Pix",
        "share_link": "https://meli.la/2kX9LmP",
        "price": 149.18,
        "return_days": 30,
        "seller": "Machenike Brasil",
        "short_description": "Teclado compacto 60% hot-swappable com switch linear vermelho.",
        "long_description": "Construção robusta resistente a salpicos acidentais na mesa.",
        "specs": [
            {"name": "Marca", "value": "Machenike"},
            {"name": "Tipo de switch", "value": "Outemu Red Linear"},
            {"name": "É resistente à água", "value": "Não"}
        ],
        "photos": ["photos/k500.png"]
    });
    let prod_lg = json!({
        "product_id": "MLB48468943",
        "title": "Monitor Gamer LG UltraGear 24 Full HD IPS 180Hz 1ms 24GS60F",
        "brand": "LG",
        "model": "24GS60F-B",
        "canonical_url": "https://www.mercadolivre.com.br/monitor-lg-ultragear-24gs60f/p/MLB48468943",
        "commission": "8%",
        "card_text": "MAIS PROCURADO\n4.9 | +2mil vendidos\nR$\n1.099\n,\n00\nR$\n789\n,\n90\n",
        "share_link": "https://meli.la/1nQ4RsT",
        "price": 789.90,
        "return_days": 30,
        "seller": "LG Loja Oficial",
        "specs": [
            {"name": "Tamanho da tela", "value": "23.8 pol"},
            {"name": "Taxa de atualização", "value": "180 Hz"}
        ],
        "variants": [
            {
                "name": "Monitor LG 24GS60F 110V/220V",
                "sku": "SKU-LG-110",
                "productID": "MLB48468943",
                "offers": {"price": 789.90, "priceCurrency": "BRL"},
                "additionalProperty": [{"name": "Voltagem", "value": "110V"}]
            },
            {
                "name": "Monitor LG 24GS60F 220V",
                "sku": "SKU-LG-220",
                "productID": "MLB48468944",
                "offers": {"price": 789.90, "priceCurrency": "BRL"},
                "additionalProperty": [{"name": "Voltagem", "value": "220V"}]
            }
        ],
        "photos": ["photos/lg24.png"]
    });
    let prod_fuxi = json!({
        "product_id": "MLB79071532",
        "title": "Headset Gamer Sem Fio Havit Fuxi-H3 Quad-Mode Branco",
        "brand": "Havit",
        "model": "Fuxi-H3",
        "canonical_url": "https://www.mercadolivre.com.br/headset-havit-fuxi-h3/p/MLB79071532",
        "commission": "10%",
        "card_text": "4.7 | +1mil vendidos\nR$\n249\n,\n00\nR$\n179\n,\n90\n",
        "share_link": "https://meli.la/2mP8VxY",
        "price": 179.90,
        "return_days": 30,
        "seller": "Havit Oficial",
        "specs": [
            {"name": "Conectividade", "value": "2.4GHz / Bluetooth / USB-C / 3.5mm"},
            {"name": "Peso", "value": "186 g"}
        ],
        "photos": ["photos/fuxi.png"]
    });
    let bot_batch = json!({
        "network": "mercadolivre_afiliados",
        "query": "hardware perifericos pc",
        "captured_at": "2026-11-15T20:10:00-04:00",
        "count": 4,
        "products": [prod_nv3, prod_k500, prod_lg, prod_fuxi]
    });

    // Start mock MinIO server on loopback.
    let minio_state = MockMinioState::default();
    let minio_router = Router::new()
        .route("/{bucket}", put(mock_put_bucket).head(mock_head_bucket))
        .route("/{bucket}/{*key}", put(mock_put_object))
        .with_state(minio_state.clone());
    let minio_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let minio_addr = minio_listener.local_addr().unwrap();
    let _minio_server = AbortOnDrop(tokio::spawn(async move {
        axum::serve(minio_listener, minio_router).await.unwrap();
    }));

    let minio_secret_path = root.join("minio.secret");
    fs::write(&minio_secret_path, "synthetic-test-minio-secret-key\n").unwrap();
    fs::set_permissions(&minio_secret_path, fs::Permissions::from_mode(0o600)).unwrap();

    let minio_cfg = MinioConfig {
        endpoint: format!("http://{minio_addr}"),
        public_base_url: format!("http://{minio_addr}"),
        bucket: "procurali-media".into(),
        region: "us-east-1".into(),
        access_key_id: "procurali-worker".into(),
        secret_access_key_file: minio_secret_path,
        timeout_seconds: 5,
    };
    let minio_client = MinioClient::from_config(minio_cfg).unwrap();

    // Start real zaturo-hub listener on loopback.
    let hub_dir = root.join("hub");
    fs::create_dir_all(&hub_dir).unwrap();
    let recipient =
        zaturo_transfer::crypto::generate_identity(&hub_dir.join("worker-ml.identity")).unwrap();
    let devices_toml = format!(
        "[[devices]]\nid=\"worker-ml\"\ntoken_sha256=\"{:x}\"\nencryption_recipient={recipient:?}\n",
        Sha256::digest(DEVICE)
    );
    fs::write(hub_dir.join("devices.toml"), devices_toml).unwrap();
    fs::set_permissions(
        hub_dir.join("devices.toml"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let apps_toml = format!(
        "[[clients]]\nid=\"procurali-builder\"\ntoken_sha256=\"{:x}\"\ncan_submit=false\n",
        Sha256::digest(READ_APP)
    );
    fs::write(hub_dir.join("apps.toml"), apps_toml).unwrap();
    fs::set_permissions(hub_dir.join("apps.toml"), fs::Permissions::from_mode(0o600)).unwrap();

    let hub_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let hub_addr = hub_listener.local_addr().unwrap();
    let hub_url = format!("http://{hub_addr}");
    let hub_state = AppState::new(
        HubConfig {
            bind: hub_addr,
            client_url: hub_url.clone(),
            database: hub_dir.join("hub.db"),
            worker_id: "worker-ml".into(),
            max_connections: 4,
            max_queued_tasks: 16,
            max_page_size: 100,
            lease_seconds: 30,
            max_workers: 4,
            devices_file: Some(hub_dir.join("devices.toml")),
            apps_file: Some(hub_dir.join("apps.toml")),
            transfer_root: Some(hub_dir.join("spool")),
            transfer_limits: Default::default(),
        },
        OWNER,
        DEVICE,
    )
    .unwrap();
    let _hub_server = AbortOnDrop(tokio::spawn(async move {
        axum::serve(hub_listener, zaturo_hub::router(hub_state))
            .await
            .unwrap();
    }));

    let parsed_schema1: MercadoLivreBotBatch = serde_json::from_value(json!({
        "schema": 1,
        "query": "monitor",
        "captured_at": "2026-11-15T18:30:32",
        "products": [prod_nv3.clone()]
    }))
    .unwrap();
    assert_eq!(parsed_schema1.schema, Some(1));
    assert!(parsed_schema1.network.is_empty());

    let raw_bytes = serde_json::to_vec(&bot_batch).unwrap();
    let mut req = prepare_and_upload_batch(
        &raw_bytes,
        &minio_client,
        &BatchIngestOptions {
            worker_id: "worker-ml".into(),
            default_category_slug: "hardware-e-perifericos".into(),
            channel_id: "@procurali".into(),
            now_ms: 1_763_252_000_000,
        },
        Some(root),
    )
    .await
    .unwrap();
    let live_now = crate::now_ms();
    req.captured_at_ms = live_now;
    for p in &mut req.products {
        p.observed_at_ms = live_now;
    }

    assert_eq!(req.products.len(), 4);
    // NV3 had 1 vertical video frame + 1 square photo -> only the 1 square photo was uploaded.
    assert_eq!(req.products[0].images.len(), 1);
    assert_eq!(minio_state.objects.lock().unwrap().len(), 4);
    // K500 flagged waterproofing contradiction and dual Pix/anúncio price.
    assert_eq!(req.products[1].pix_price_cents, 13_879);
    assert_eq!(req.products[1].price_brl_cents, 14_918);
    assert!(
        req.products[1]
            .limitations
            .iter()
            .any(|l| l.contains("resistência à água"))
    );
    // NV3 flagged interest-bearing 18x installments (18 * 46.26 = 832.68 > 699.90).
    assert!(
        req.products[0]
            .limitations
            .iter()
            .any(|l| l.contains("acréscimo/juros"))
    );
    // Monitor flagged multi-voltage variants.
    assert!(
        req.products[2]
            .limitations
            .iter()
            .any(|l| l.contains("voltagens distintas"))
    );

    let report = ingest_to_hub(&hub_url, DEVICE, &req, "e2e-ml-4-products")
        .await
        .unwrap();
    assert_eq!(report.product_count, 4);
    assert_eq!(report.image_count, 4);
    assert_eq!(report.telegram_queued_count, 4);
    assert_eq!(report.telegram_awaiting_approval_count, 0);

    let http = reqwest::Client::builder().no_proxy().build().unwrap();
    let catalog: SiteCatalogSnapshot = http
        .get(format!("{hub_url}/v1/commerce/mercadolivre/site-catalog"))
        .bearer_auth(READ_APP)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(catalog.offers.len(), 4);
    let catalog_json = serde_json::to_string(&catalog).unwrap();
    assert!(no_local_path_leak(&catalog_json));
    assert!(catalog_json.contains("https://meli.la/1iWQZbJ"));
    assert!(catalog.offers.iter().all(|o| {
        o.image_path
            .starts_with(&format!("http://{minio_addr}/procurali-media/"))
    }));

    let queue = |status: &'static str| {
        let http = http.clone();
        let hub_url = hub_url.clone();
        async move {
            http.get(format!(
                    "{hub_url}/v1/commerce/mercadolivre/telegram-queue?channel_id=%40procurali&status={status}&limit=10"
                ))
                .bearer_auth(DEVICE)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json::<Vec<TelegramOfferQueueItem>>()
                .await
                .unwrap()
        }
    };
    assert_eq!(queue("queued").await.len(), 4);
    assert!(queue("awaiting_approval").await.is_empty());
}

async fn spawn_watch_test_backends(root: &Path) -> (WorkerConfig, AbortOnDrop, AbortOnDrop) {
    let minio_state = MockMinioState::default();
    let minio_router = Router::new()
        .route("/{bucket}", put(mock_put_bucket).head(mock_head_bucket))
        .route("/{bucket}/{*key}", put(mock_put_object))
        .with_state(minio_state);
    let minio_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let minio_addr = minio_listener.local_addr().unwrap();
    let minio_srv = AbortOnDrop(tokio::spawn(async move {
        axum::serve(minio_listener, minio_router).await.unwrap();
    }));

    let minio_secret_path = root.join("minio.watch.secret");
    fs::write(&minio_secret_path, "synthetic-test-minio-secret-key\n").unwrap();
    fs::set_permissions(&minio_secret_path, fs::Permissions::from_mode(0o600)).unwrap();

    let minio_cfg = MinioConfig {
        endpoint: format!("http://{minio_addr}"),
        public_base_url: format!("http://{minio_addr}"),
        bucket: "procurali-media".into(),
        region: "us-east-1".into(),
        access_key_id: "procurali-worker".into(),
        secret_access_key_file: minio_secret_path,
        timeout_seconds: 5,
    };

    let hub_dir = root.join("hub-watch");
    fs::create_dir_all(&hub_dir).unwrap();
    let recipient =
        zaturo_transfer::crypto::generate_identity(&hub_dir.join("worker-ml.identity")).unwrap();
    let devices_toml = format!(
        "[[devices]]\nid=\"worker-ml\"\ntoken_sha256=\"{:x}\"\nencryption_recipient={recipient:?}\n",
        Sha256::digest(DEVICE)
    );
    fs::write(hub_dir.join("devices.toml"), devices_toml).unwrap();
    fs::set_permissions(
        hub_dir.join("devices.toml"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();

    let hub_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let hub_addr = hub_listener.local_addr().unwrap();
    let hub_url = format!("http://{hub_addr}");
    let hub_state = AppState::new(
        HubConfig {
            bind: hub_addr,
            client_url: hub_url.clone(),
            database: hub_dir.join("hub.db"),
            worker_id: "worker-ml".into(),
            max_connections: 4,
            max_queued_tasks: 16,
            max_page_size: 100,
            lease_seconds: 30,
            max_workers: 4,
            devices_file: Some(hub_dir.join("devices.toml")),
            apps_file: None,
            transfer_root: Some(hub_dir.join("spool")),
            transfer_limits: Default::default(),
        },
        OWNER,
        DEVICE,
    )
    .unwrap();
    let hub_srv = AbortOnDrop(tokio::spawn(async move {
        axum::serve(hub_listener, zaturo_hub::router(hub_state))
            .await
            .unwrap();
    }));

    let worker_cfg = WorkerConfig {
        hub_url,
        worker_id: "worker-ml".into(),
        database: root.join("worker.db"),
        workspace_root: root.join("ws"),
        artifact_root: root.join("art"),
        capabilities: vec!["diagnostic.v1".into()],
        max_concurrent_tasks: 1,
        heartbeat_seconds: 5,
        reconnect_max_seconds: 30,
        models_config: None,
        assistant_models_config: None,
        computer_models_config: None,
        computer: None,
        opencode: None,
        files: None,
        youtube: None,
        android: None,
        minio: Some(minio_cfg),
        telegram: None,
    };
    (worker_cfg, minio_srv, hub_srv)
}

#[tokio::test]
async fn ingest_file_if_changed_skips_unchanged_and_reingests_on_change() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let photos_dir = root.join("photos");
    fs::create_dir_all(&photos_dir).unwrap();
    fs::write(photos_dir.join("item.png"), valid_png_bytes(800, 800)).unwrap();

    let (worker_cfg, _minio_srv, _hub_srv) = spawn_watch_test_backends(root).await;
    let input_path = root.join("products.json");
    let state_file = root.join("state/last.sha256");

    let make_batch = |price: f64| {
        json!({
            "network": "mercadolivre_afiliados",
            "query": "hardware watch test",
            "captured_at": "2026-11-16T12:00:00Z",
            "products": [{
                "product_id": "MLB39410080",
                "title": "SSD Kingston NV3 500GB NVMe",
                "canonical_url": "https://www.mercadolivre.com.br/ssd-kingston/p/MLB39410080",
                "share_link": "https://meli.la/1iWQZbJ",
                "price": price,
                "currency": "BRL",
                "seller": "Loja Oficial Kingston",
                "photos": ["photos/item.png"]
            }]
        })
    };

    fs::write(
        &input_path,
        serde_json::to_vec(&make_batch(499.90)).unwrap(),
    )
    .unwrap();
    let opts = BatchIngestOptions {
        worker_id: "worker-ml".into(),
        default_category_slug: "hardware-e-perifericos".into(),
        channel_id: "@procurali".into(),
        now_ms: 1_763_300_000_000,
    };

    let first = ingest_file_if_changed(
        &worker_cfg,
        DEVICE,
        &input_path,
        &state_file,
        &opts,
        Some(root),
    )
    .await
    .unwrap()
    .expect("initial batch must ingest");
    assert_eq!(first.product_count, 1);

    let second = ingest_file_if_changed(
        &worker_cfg,
        DEVICE,
        &input_path,
        &state_file,
        &opts,
        Some(root),
    )
    .await
    .unwrap();
    assert!(second.is_none(), "unchanged digest must skip ingest");

    fs::write(
        &input_path,
        serde_json::to_vec(&make_batch(449.90)).unwrap(),
    )
    .unwrap();
    let third = ingest_file_if_changed(
        &worker_cfg,
        DEVICE,
        &input_path,
        &state_file,
        &opts,
        Some(root),
    )
    .await
    .unwrap()
    .expect("modified batch must ingest again");
    assert_eq!(third.product_count, 1);
}

#[tokio::test]
async fn sync_reference_catalog_supports_304_cache_and_fast_local_queries() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let photos_dir = root.join("photos");
    fs::create_dir_all(&photos_dir).unwrap();
    fs::write(photos_dir.join("item.png"), valid_png_bytes(800, 800)).unwrap();

    let (worker_cfg, _minio_srv, _hub_srv) = spawn_watch_test_backends(root).await;
    let input_path = root.join("products.json");
    let ref_path = root.join("state/vps-reference.json");

    let batch = json!({
        "network": "mercadolivre_afiliados",
        "query": "mouse gamer",
        "captured_at": "2026-11-16T12:00:00Z",
        "products": [{
            "product_id": "MLB63816534",
            "title": "Delux M900pro Mouse Gamer Sem Fio 8000Hz",
            "canonical_url": "https://www.mercadolivre.com.br/delux-m900pro/p/MLB63816534",
            "share_link": "https://meli.la/31tuNW3",
            "price": 299.86,
            "currency": "BRL",
            "brand": "Delux",
            "seller": "Delux Official",
            "photos": ["photos/item.png"]
        }]
    });
    fs::write(&input_path, serde_json::to_vec(&batch).unwrap()).unwrap();
    let opts = BatchIngestOptions {
        worker_id: "worker-ml".into(),
        default_category_slug: "hardware-e-perifericos".into(),
        channel_id: "@procurali".into(),
        now_ms: 1_763_300_000_000,
    };
    ingest_file_to_hub(
        &worker_cfg,
        DEVICE,
        &input_path,
        &opts,
        Some("ref-sync-1"),
        Some(root),
    )
    .await
    .unwrap();

    let sync1 = sync_reference_catalog(&worker_cfg.hub_url, DEVICE, &ref_path)
        .await
        .unwrap();
    assert_eq!(sync1.status, "updated");
    assert_eq!(sync1.total_products, 1);
    assert!(sync1.etag.is_some());

    let sync2 = sync_reference_catalog(&worker_cfg.hub_url, DEVICE, &ref_path)
        .await
        .unwrap();
    assert_eq!(sync2.status, "not_modified");
    assert_eq!(sync2.total_products, 1);

    let hits = load_and_query_reference_catalog(
        &ref_path,
        Some("delux m900pro"),
        None,
        Some("hardware-e-perifericos"),
        10,
    )
    .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].product_id, "MLB63816534");
    assert_eq!(hits[0].current_price_cents, 29_986);

    let by_id =
        load_and_query_reference_catalog(&ref_path, None, Some("mlb63816534"), None, 10).unwrap();
    assert_eq!(by_id.len(), 1);
}
