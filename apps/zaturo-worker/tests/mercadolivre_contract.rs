//! Independent black-box end-to-end contract tests for `zaturo_worker` Mercado Livre ingestion,
//! MinIO media filtering/upload, reference catalog ETag sync, and automated Telegram Bot API publishing.

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    routing::put,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::task::JoinHandle;
use zaturo_core::{TelegramDeliveryStatus, slugify_pt_br};
use zaturo_hub::{AppState, config::HubConfig, now_ms, router};
use zaturo_transfer::minio::{MinioClient, MinioConfig};
use zaturo_worker::{
    mercadolivre::{
        BatchIngestOptions, ingest_to_hub, load_and_query_reference_catalog,
        prepare_and_upload_batch, sync_reference_catalog,
    },
    telegram::{TelegramBotClient, TelegramBotConfig},
};

const OWNER: &str = "synthetic-owner-worker-contract-token";
const DEVICE: &str = "synthetic-device-worker-contract-token";
const BOT_TOKEN: &str = "123456:synthetic_worker_contract_bot_token";

struct AbortOnDrop(JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn minimal_png(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    out.extend_from_slice(&13u32.to_be_bytes());
    out.extend_from_slice(b"IHDR");
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&[8, 2, 0, 0, 0, 0, 0, 0, 0]);
    out
}

#[derive(Clone, Default)]
struct MockMinio {
    buckets: Arc<Mutex<BTreeSet<String>>>,
    objects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

async fn minio_head_bucket(
    State(s): State<MockMinio>,
    AxumPath(bucket): AxumPath<String>,
) -> StatusCode {
    if s.buckets.lock().unwrap().contains(&bucket) {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn minio_put_bucket(
    State(s): State<MockMinio>,
    AxumPath(bucket): AxumPath<String>,
) -> StatusCode {
    s.buckets.lock().unwrap().insert(bucket);
    StatusCode::OK
}

async fn minio_put_object(
    State(s): State<MockMinio>,
    AxumPath((bucket, key)): AxumPath<(String, String)>,
    body: Bytes,
) -> StatusCode {
    s.objects
        .lock()
        .unwrap()
        .insert(format!("{bucket}/{key}"), body.to_vec());
    StatusCode::OK
}

async fn minio_get_object(
    State(s): State<MockMinio>,
    AxumPath((bucket, key)): AxumPath<(String, String)>,
) -> (StatusCode, Vec<u8>) {
    match s.objects.lock().unwrap().get(&format!("{bucket}/{key}")) {
        Some(bytes) => (StatusCode::OK, bytes.clone()),
        None => (StatusCode::NOT_FOUND, Vec::new()),
    }
}

#[derive(Clone, Default)]
struct MockTelegram {
    multipart_photos: Arc<Mutex<usize>>,
    messages: Arc<Mutex<Vec<Value>>>,
    updates: Arc<Mutex<Vec<Value>>>,
}

async fn tg_method(
    State(s): State<MockTelegram>,
    AxumPath((_bot, method)): AxumPath<(String, String)>,
    headers: HeaderMap,
    raw: Bytes,
) -> (StatusCode, Json<Value>) {
    let ctype = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if method == "sendPhoto" && ctype.starts_with("multipart/form-data") {
        *s.multipart_photos.lock().unwrap() += 1;
        return (
            StatusCode::OK,
            Json(json!({ "ok": true, "result": { "message_id": 777 } })),
        );
    }
    match method.as_str() {
        "sendMessage" => {
            if let Ok(v) = serde_json::from_slice::<Value>(&raw) {
                s.messages.lock().unwrap().push(v);
            }
            (
                StatusCode::OK,
                Json(json!({ "ok": true, "result": { "message_id": 778 } })),
            )
        }
        "getUpdates" => {
            let updates = std::mem::take(&mut *s.updates.lock().unwrap());
            (
                StatusCode::OK,
                Json(json!({ "ok": true, "result": updates })),
            )
        }
        _ => (StatusCode::NOT_FOUND, Json(json!({ "ok": false }))),
    }
}

#[tokio::test]
async fn worker_ingest_filters_reel_photos_syncs_reference_and_auto_publishes_telegram() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();

    // Write local vertical reel photo (dropped) + square product photo (kept).
    let reel_path = p.join("01-reel.png");
    let main_path = p.join("02-product.png");
    fs::write(&reel_path, minimal_png(540, 960)).unwrap();
    fs::write(&main_path, minimal_png(800, 800)).unwrap();

    // Start mock MinIO, mock Telegram, and real Hub.
    let minio_state = MockMinio::default();
    let minio_app = Router::new()
        .route("/{bucket}", put(minio_put_bucket).head(minio_head_bucket))
        .route(
            "/{bucket}/{*key}",
            put(minio_put_object).get(minio_get_object),
        )
        .with_state(minio_state.clone());
    let minio_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let minio_addr = minio_listener.local_addr().unwrap();
    let _minio_guard = AbortOnDrop(tokio::spawn(async move {
        axum::serve(minio_listener, minio_app).await.unwrap();
    }));

    let tg_state = MockTelegram::default();
    let tg_app = Router::new()
        .route("/{bot}/{method}", axum::routing::post(tg_method))
        .with_state(tg_state.clone());
    let tg_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tg_addr = tg_listener.local_addr().unwrap();
    let _tg_guard = AbortOnDrop(tokio::spawn(async move {
        axum::serve(tg_listener, tg_app).await.unwrap();
    }));

    let recipient = zaturo_transfer::crypto::generate_identity(&p.join("worker.id")).unwrap();
    fs::write(
        p.join("devices.toml"),
        format!(
            "[[devices]]\nid=\"worker-ml\"\ntoken_sha256=\"{:x}\"\nencryption_recipient={recipient:?}\n",
            Sha256::digest(DEVICE)
        ),
    )
    .unwrap();
    fs::set_permissions(p.join("devices.toml"), fs::Permissions::from_mode(0o600)).unwrap();

    let hub_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let hub_addr = hub_listener.local_addr().unwrap();
    let hub_state = AppState::new(
        HubConfig {
            bind: hub_addr,
            client_url: format!("http://{hub_addr}"),
            database: p.join("hub.db"),
            worker_id: "worker-ml".into(),
            max_connections: 4,
            max_queued_tasks: 16,
            max_page_size: 100,
            lease_seconds: 30,
            max_workers: 4,
            devices_file: Some(p.join("devices.toml")),
            apps_file: None,
            transfer_root: Some(p.join("spool")),
            transfer_limits: Default::default(),
        },
        OWNER,
        DEVICE,
    )
    .unwrap();
    let _hub_guard = AbortOnDrop(tokio::spawn(async move {
        axum::serve(hub_listener, router(hub_state)).await.unwrap();
    }));

    let secret_key_file = p.join("minio.secret");
    fs::write(&secret_key_file, "synthetic-minio-secret-key\n").unwrap();
    fs::set_permissions(&secret_key_file, fs::Permissions::from_mode(0o600)).unwrap();

    let minio_client = MinioClient::from_config(MinioConfig {
        endpoint: format!("http://{minio_addr}"),
        public_base_url: "https://media.procurali.com.br".into(),
        bucket: "procurali-media".into(),
        region: "us-east-1".into(),
        access_key_id: "procurali".into(),
        secret_access_key_file: secret_key_file,
        timeout_seconds: 10,
    })
    .unwrap();

    let ts = now_ms().saturating_sub(2_000).max(1);
    let captured_iso = zaturo_core::format_utc_iso_ms(ts);
    let bot_json = serde_json::to_vec(&json!({
        "schema": 1,
        "network": "mercadolivre",
        "query": "headset gamer",
        "captured_at": captured_iso,
        "count": 1,
        "products": [{
            "product_id": "MLB99887766",
            "title": "Headset Gamer Redragon Zeus X",
            "brand": "Redragon",
            "model": "H510-RGB",
            "product_url": "https://www.mercadolivre.com.br/headset/p/MLB99887766",
            "share_link": "https://meli.la/2ZpQqwX",
            "share_id": "1PBCGF-SMN5",
            "commission": "6%",
            "price": 249.90,
            "currency": "BRL",
            "availability": "InStock",
            "photos": [reel_path, main_path]
        }]
    }))
    .unwrap();

    let ingest_req = prepare_and_upload_batch(
        &bot_json,
        &minio_client,
        &BatchIngestOptions {
            worker_id: "worker-ml".into(),
            default_category_slug: "hardware-e-perifericos".into(),
            channel_id: "@procurali".into(),
            now_ms: ts,
        },
        Some(p),
    )
    .await
    .unwrap();

    // Vertical 9:16 photo filtered out; only square product photo uploaded to MinIO.
    assert_eq!(ingest_req.products[0].images.len(), 1);
    assert_eq!(minio_state.objects.lock().unwrap().len(), 1);
    assert_eq!(
        ingest_req.products[0].slug,
        slugify_pt_br("Headset Gamer Redragon Zeus X", "MLB99887766")
    );

    let hub_base = format!("http://{hub_addr}");
    let report = ingest_to_hub(&hub_base, DEVICE, &ingest_req, "worker-contract-1")
        .await
        .unwrap();
    assert_eq!(report.telegram_queued_count, 1);

    // Fast ETag-cached reference DB sync: first `updated`, second `not_modified`, offline query works.
    let ref_path = p.join("vps-reference.json");
    let sync1 = sync_reference_catalog(&hub_base, DEVICE, &ref_path)
        .await
        .unwrap();
    assert_eq!(sync1.status, "updated");
    assert_eq!(sync1.total_products, 1);
    let sync2 = sync_reference_catalog(&hub_base, DEVICE, &ref_path)
        .await
        .unwrap();
    assert_eq!(sync2.status, "not_modified");
    let matched =
        load_and_query_reference_catalog(&ref_path, Some("redragon"), None, None, 10).unwrap();
    assert_eq!(matched.len(), 1);

    // Automated Telegram publisher claims & posts queued offer to mock Telegram Bot API cleanly.
    let tg_token_file: PathBuf = p.join("tg.token");
    fs::write(&tg_token_file, BOT_TOKEN).unwrap();
    fs::set_permissions(&tg_token_file, fs::Permissions::from_mode(0o600)).unwrap();

    let tg_client = TelegramBotClient::from_config(TelegramBotConfig {
        api_base_url: format!("http://{tg_addr}"),
        bot_token_file: tg_token_file,
        default_channel_id: "@procurali".into(),
        site_base_url: Some("https://procurali.com.br".into()),
        min_send_interval_ms: 0,
        max_send_interval_ms: None,
        allow_live_publish: true,
        auto_publish_on_ingest: true,
        local_minio_endpoint: Some(format!("http://{minio_addr}")),
        minio_bucket: Some("procurali-media".into()),
        env_file: None,
    })
    .unwrap();

    let pub_rep = tg_client
        .publish_queued_from_hub(&hub_base, DEVICE, Some("@procurali"), 10, false, ts + 1_000)
        .await
        .unwrap();
    assert_eq!(pub_rep.published_count, 1);
    assert_eq!(pub_rep.failed_count, 0);
    assert_eq!(*tg_state.multipart_photos.lock().unwrap(), 1);
    assert_eq!(pub_rep.items[0].status, TelegramDeliveryStatus::Published);

    // Command polling answers `/buscar redragon` using the Published offer.
    tg_state.updates.lock().unwrap().push(json!({
        "update_id": 1001,
        "message": {
            "message_id": 88,
            "chat": { "id": -1001234567890i64, "type": "supergroup" },
            "text": "/buscar redragon"
        }
    }));
    let poll_rep = tg_client
        .poll_commands_once(&hub_base, DEVICE, None, 0)
        .await
        .unwrap();
    assert_eq!(poll_rep.commands_answered, 1);
    let sent_msgs = tg_state.messages.lock().unwrap();
    assert_eq!(sent_msgs.len(), 1);
    assert!(
        sent_msgs[0]["text"]
            .as_str()
            .unwrap()
            .contains("Redragon Zeus")
    );
}
