use super::*;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path as AxumPath, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    os::unix::fs::PermissionsExt,
    sync::{Arc, Mutex},
};
use std::{fs, path::PathBuf, time::Duration};
use tokio::task::JoinHandle;
use zaturo_core::{
    MercadoLivreIngestBatchRequest, MercadoLivreProductState, MercadoLivreSpec,
    MercadoLivreVariant, MinioProductImage, ValidatedMercadoLivreProduct,
};
use zaturo_hub::{AppState, config::HubConfig};

const OWNER: &str = "synthetic-owner-telegram-test-token-value";
const DEVICE: &str = "synthetic-device-telegram-test-token-value";
const BOT_TOKEN: &str = "987654:synthetic_telegram_bot_token_value";

struct AbortOnDrop(JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn test_tg_config(
    api_base_url: String,
    bot_token_file: PathBuf,
    min_send_interval_ms: u64,
    allow_live_publish: bool,
) -> TelegramBotConfig {
    TelegramBotConfig {
        api_base_url,
        bot_token_file,
        default_channel_id: "@procurali".into(),
        site_base_url: Some("https://procurali.com.br".into()),
        min_send_interval_ms,
        max_send_interval_ms: None,
        allow_live_publish,
        auto_publish_on_ingest: false,
        local_minio_endpoint: None,
        minio_bucket: None,
        env_file: None,
    }
}

fn minimal_png_bytes(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    out.extend_from_slice(&13u32.to_be_bytes());
    out.extend_from_slice(b"IHDR");
    out.extend_from_slice(&width.to_be_bytes());
    out.extend_from_slice(&height.to_be_bytes());
    out.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    out
}

struct MultipartPhotoRecord {
    content_type: String,
    body: Vec<u8>,
}

#[derive(Clone, Default)]
struct MockTelegramState {
    send_photo_plans: Arc<Mutex<VecDeque<(u16, Value)>>>,
    send_message_calls: Arc<Mutex<Vec<Value>>>,
    send_photo_calls: Arc<Mutex<Vec<Value>>>,
    send_photo_multipart_bodies: Arc<Mutex<Vec<MultipartPhotoRecord>>>,
    updates_queue: Arc<Mutex<Vec<Value>>>,
}

async fn mock_tg_method(
    State(state): State<MockTelegramState>,
    AxumPath((_bot_path, method)): AxumPath<(String, String)>,
    headers: HeaderMap,
    raw_body: Bytes,
) -> (StatusCode, Json<Value>) {
    let ctype = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    if method == "sendPhoto" && ctype.starts_with("multipart/form-data") {
        state
            .send_photo_multipart_bodies
            .lock()
            .unwrap()
            .push(MultipartPhotoRecord {
                content_type: ctype,
                body: raw_body.to_vec(),
            });
        return (
            StatusCode::OK,
            Json(json!({ "ok": true, "result": { "message_id": 5555 } })),
        );
    }
    let body: Value = serde_json::from_slice(&raw_body).unwrap_or(Value::Null);
    match method.as_str() {
        "sendPhoto" => {
            state.send_photo_calls.lock().unwrap().push(body);
            if let Some((status, payload)) = state.send_photo_plans.lock().unwrap().pop_front() {
                let code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_REQUEST);
                return (code, Json(payload));
            }
            (
                StatusCode::OK,
                Json(json!({ "ok": true, "result": { "message_id": 5001 } })),
            )
        }
        "sendMessage" => {
            state.send_message_calls.lock().unwrap().push(body);
            (
                StatusCode::OK,
                Json(json!({ "ok": true, "result": { "message_id": 6001 } })),
            )
        }
        "getUpdates" => {
            let updates = std::mem::take(&mut *state.updates_queue.lock().unwrap());
            (
                StatusCode::OK,
                Json(json!({ "ok": true, "result": updates })),
            )
        }
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({ "ok": false, "description": "unknown method" })),
        ),
    }
}

fn sample_validated_product(
    product_id: &str,
    title: &str,
    price_cents: i64,
) -> ValidatedMercadoLivreProduct {
    let mut p = ValidatedMercadoLivreProduct {
        product_id: product_id.to_owned(),
        slug: zaturo_core::slugify_pt_br(title, product_id),
        brand: "Kingston".into(),
        model: Some("SNV3S".into()),
        title: title.to_owned(),
        exact_variant: "PCIe 4.0 x4".into(),
        category_slug: "hardware-e-perifericos".into(),
        seller_name: "Loja oficial FAMILAR".into(),
        canonical_url: format!("https://www.mercadolivre.com.br/p/{product_id}"),
        share_link: "https://meli.la/1iWQZbJ".into(),
        share_id: format!("1PBCGF-{product_id}"),
        share_text: "https://meli.la/1iWQZbJ".into(),
        sponsored: false,
        commission_bp: Some(500),
        commission_label: Some("5%".into()),
        marketplace_rating: Some("4,9 / 5".into()),
        marketplace_sales_label: Some("+10 mil vendidos".into()),
        card_badge: Some("Mais vendido".into()),
        discount_label: Some("25% OFF".into()),
        installment_summary: Some("10x sem juros".into()),
        return_days: Some(30),
        availability: "InStock".into(),
        price_valid_until: None,
        price_brl_cents: price_cents,
        pix_price_cents: price_cents,
        reference_price_cents: Some(price_cents + 10_000),
        currency: "BRL".into(),
        payment_conditions: "À vista no Pix ou cartão".into(),
        short_condition: "10x sem juros".into(),
        freight_status: "Frete grátis".into(),
        observed_at_ms: crate::now_ms(),
        freshness_ttl_minutes: 30,
        state: MercadoLivreProductState::Approved,
        one_line_reason: "SSD NVMe PCIe 4.0.".into(),
        for_whom: "Upgrade de PC ou notebook.".into(),
        short_description: "Leitura até 6000 MB/s.".into(),
        long_description: "Descrição completa validada.".into(),
        supported_benefits: vec!["PCIe 4.0".into()],
        limitations: vec!["Sem dissipador incluído.".into()],
        unknowns: vec![],
        tags: vec!["ssd".into(), "hardware".into()],
        specs: vec![MercadoLivreSpec {
            name: "Capacidade".into(),
            value: "1 TB".into(),
        }],
        variants: vec![MercadoLivreVariant {
            variant_product_id: product_id.to_owned(),
            sku: "SNV3S".into(),
            name: title.to_owned(),
            color: None,
            voltage: None,
            price_brl_cents: price_cents,
            currency: "BRL".into(),
            availability: "https://schema.org/InStock".into(),
            canonical_url: None,
            return_days: Some(30),
            return_fees: Some("Grátis".into()),
            highlights: vec![],
        }],
        images: vec![MinioProductImage {
            position: 0,
            bucket: "procurali-media".into(),
            object_key: format!("mercadolivre/{product_id}/hero.png"),
            public_url: format!(
                "https://media.procurali.com.br/procurali-media/mercadolivre/{product_id}/hero.png"
            ),
            source_origin_url: None,
            content_type: "image/png".into(),
            size_bytes: 4096,
            sha256_hex: "a".repeat(64),
            alt_text: title.to_owned(),
        }],
        evidence_sha256: "b".repeat(64),
    };
    if product_id == "MLB53222888" {
        p.share_link = "https://meli.la/2xYvZ9W".into();
        p.share_text = "https://meli.la/2xYvZ9W".into();
    }
    p
}

#[test]
fn telegram_config_rejects_empty_token_and_invalid_endpoints() {
    let bad = test_tg_config(
        "ftp://api.telegram.org".into(),
        PathBuf::from("/tmp/tg.token"),
        0,
        false,
    );
    assert!(bad.validate().is_err());

    let mut bad_max = test_tg_config(
        "https://api.telegram.org".into(),
        PathBuf::from("/tmp/tg.token"),
        60_000,
        false,
    );
    bad_max.max_send_interval_ms = Some(30_000);
    assert!(bad_max.validate().is_err());

    let good = test_tg_config(
        "https://api.telegram.org".into(),
        PathBuf::from("/tmp/tg.token"),
        1100,
        false,
    );
    good.validate().unwrap();
}

#[test]
fn env_file_overrides_and_natural_one_to_three_minute_delay_work() {
    let dir = tempfile::tempdir().unwrap();
    let env_path = dir.path().join("telegram.env");
    fs::write(
        &env_path,
        [
            "# Procurali Telegram bot test env",
            "TELEGRAM_BOT_TOKEN=\"synthetic-123456:env_file_bot_token\"",
            "TELEGRAM_CHANNEL_ID=-1001987654321",
            "TELEGRAM_MIN_INTERVAL_SECS=60",
            "TELEGRAM_MAX_INTERVAL_SECS=180",
            "TELEGRAM_ALLOW_LIVE_PUBLISH=true",
            "TELEGRAM_AUTO_PUBLISH_ON_INGEST=true",
        ]
        .join("\n"),
    )
    .unwrap();

    let mut cfg = test_tg_config(
        "https://api.telegram.org".into(),
        PathBuf::new(),
        1100,
        false,
    );
    cfg.env_file = Some(env_path);

    let client = TelegramBotClient::from_config(cfg).unwrap();
    assert_eq!(client.config().default_channel_id, "-1001987654321");
    assert!(client.config().allow_live_publish);
    assert!(client.config().auto_publish_on_ingest);
    assert_eq!(client.effective_send_interval_range_ms(), (60_000, 180_000));

    for idx in 0..10 {
        let delay = client.compute_inter_post_delay(idx, &format!("tg-delivery-{idx}"));
        assert!(
            delay >= Duration::from_secs(60) && delay <= Duration::from_secs(180),
            "expected 60..=180s delay, got {:?}",
            delay
        );
    }
}

#[tokio::test]
async fn end_to_end_hub_telegram_publish_retry_fallback_polling_and_whatsapp_assisted() {
    let dir = tempfile::tempdir().unwrap();
    let hub_dir = dir.path().join("hub");
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
    let _hub_server = AbortOnDrop(tokio::spawn(async move {
        axum::serve(hub_listener, zaturo_hub::router(hub_state))
            .await
            .unwrap();
    }));

    // Ingest 2 validated products into Hub queue.
    let mut c1 = sample_validated_product("MLB39410080", "SSD Kingston NV3 1TB NVMe", 44_990);
    c1.state = MercadoLivreProductState::Candidate;
    let mut c2 = sample_validated_product("MLB53222888", "Teclado Mecanico Redragon K500", 13_879);
    c2.state = MercadoLivreProductState::Candidate;
    let batch = MercadoLivreIngestBatchRequest {
        worker_id: "worker-ml".into(),
        search_query: "hardware".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: 1_763_252_000_000,
        products: vec![c1, c2],
    };
    crate::mercadolivre::ingest_to_hub(&hub_url, DEVICE, &batch, "tg-e2e-batch-1")
        .await
        .unwrap();
    let empty = TelegramBotClient::from_config(test_tg_config(
        "http://127.0.0.1:9".into(),
        write_token(dir.path(), "unused.token"),
        0,
        true,
    ))
    .unwrap()
    .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
    .await
    .unwrap();
    assert_eq!(
        empty.fetched_count, 0,
        "ingest must not approve publication"
    );
    for product in ["MLB39410080", "MLB53222888"] {
        owner_approve(&hub_url, product).await;
    }

    // Start mock Telegram Bot API server.
    let tg_state = MockTelegramState::default();
    {
        let mut plans = tg_state.send_photo_plans.lock().unwrap();
        // Item 1: 429 retry_after=1 then 200 OK
        plans.push_back((
            429,
            json!({
                "ok": false,
                "error_code": 429,
                "description": "Too Many Requests: retry after 1",
                "parameters": { "retry_after": 1 }
            }),
        ));
        plans.push_back((200, json!({ "ok": true, "result": { "message_id": 7001 } })));
        // Item 2: 400 Bad Request -> falls back cleanly to sendMessage
        plans.push_back((
                400,
                json!({ "ok": false, "error_code": 400, "description": "Bad Request: failed to get HTTP URL content" }),
            ));
    }

    let tg_router = Router::new()
        .route("/{bot_token}/{method}", post(mock_tg_method))
        .with_state(tg_state.clone());
    let tg_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tg_addr = tg_listener.local_addr().unwrap();
    let _tg_server = AbortOnDrop(tokio::spawn(async move {
        axum::serve(tg_listener, tg_router).await.unwrap();
    }));

    let token_file = dir.path().join("telegram.token");
    fs::write(&token_file, format!("{BOT_TOKEN}\n")).unwrap();
    fs::set_permissions(&token_file, fs::Permissions::from_mode(0o600)).unwrap();

    let cfg = test_tg_config(format!("http://{tg_addr}"), token_file, 0, true);
    let tg = TelegramBotClient::from_config(cfg).unwrap();

    // 1) Dry run produces previews & WhatsApp Web drafts without sending or acking.
    let preview = tg
        .publish_queued_from_hub(&hub_url, DEVICE, None, 10, true, crate::now_ms())
        .await
        .unwrap();
    assert!(preview.dry_run);
    assert_eq!(preview.fetched_count, 2);
    assert_eq!(preview.whatsapp_package.draft_count, 2);
    assert!(
        preview.items[0]
            .caption_html_preview
            .contains("OFERTA RELÂMPAGO")
    );
    assert!(tg_state.send_photo_calls.lock().unwrap().is_empty());

    // 2) Live publish exercises 429 retry and 400 -> sendMessage fallback, acking Hub.
    let report = tg
        .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
        .await
        .unwrap();
    assert!(!report.dry_run);
    assert_eq!(report.fetched_count, 2);
    assert_eq!(report.published_count, 2);
    assert_eq!(report.failed_count, 0);
    assert_eq!(tg_state.send_photo_calls.lock().unwrap().len(), 3);
    assert_eq!(tg_state.send_message_calls.lock().unwrap().len(), 1);
    assert_eq!(
        report.whatsapp_package.mode,
        "whatsapp_web_human_assisted_no_meta_api"
    );
    assert!(
        report
            .whatsapp_package
            .combined_digest_text
            .contains("https://meli.la/1iWQZbJ")
    );

    let published_queue = tg
        .fetch_hub_queue(
            &hub_url,
            DEVICE,
            Some("@procurali"),
            Some(TelegramDeliveryStatus::Published),
            10,
        )
        .await
        .unwrap();
    assert_eq!(published_queue.len(), 2);

    // 3) Interactive command polling (/start, /ofertas, /buscar, /whatsapp)
    {
        let mut updates = tg_state.updates_queue.lock().unwrap();
        updates.push(json!({
            "update_id": 101,
            "message": { "message_id": 11, "chat": { "id": 55001 }, "text": "/start" }
        }));
        updates.push(json!({
            "update_id": 102,
            "message": { "message_id": 12, "chat": { "id": 55001 }, "text": "/buscar kingston" }
        }));
        updates.push(json!({
            "update_id": 103,
            "message": { "message_id": 13, "chat": { "id": 55001 }, "text": "/whatsapp" }
        }));
    }
    let poll = tg
        .poll_commands_once(&hub_url, DEVICE, None, 0)
        .await
        .unwrap();
    assert_eq!(poll.updates_processed, 3);
    assert_eq!(poll.commands_answered, 3);
    assert_eq!(poll.next_offset, Some(104));

    {
        let messages = tg_state.send_message_calls.lock().unwrap();
        // 1 fallback sendMessage + 3 interactive command replies = 4
        assert_eq!(messages.len(), 4);
        assert!(
            messages[2]["text"]
                .as_str()
                .unwrap()
                .contains("https://meli.la/1iWQZbJ")
        );
        assert!(
            messages[3]["text"]
                .as_str()
                .unwrap()
                .contains("WhatsApp Web")
        );
    }

    // 4) Uncertain transport failure (closed loopback port) marks Failed without retrying.
    let dead_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead_addr = dead_listener.local_addr().unwrap();
    drop(dead_listener);

    let mut c3 =
        sample_validated_product("MLB48468943", "Headset Gamer Redragon Zeus Lite", 19_990);
    c3.state = MercadoLivreProductState::Candidate;
    let batch2 = MercadoLivreIngestBatchRequest {
        worker_id: "worker-ml".into(),
        search_query: "headset".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: 1_763_252_300_000,
        products: vec![c3],
    };
    crate::mercadolivre::ingest_to_hub(&hub_url, DEVICE, &batch2, "tg-e2e-batch-2")
        .await
        .unwrap();
    owner_approve(&hub_url, "MLB48468943").await;
    let dead_token_file = dir.path().join("telegram-dead.token");
    fs::write(&dead_token_file, format!("{BOT_TOKEN}\n")).unwrap();
    fs::set_permissions(&dead_token_file, fs::Permissions::from_mode(0o600)).unwrap();
    let dead_tg = TelegramBotClient::from_config(test_tg_config(
        format!("http://{dead_addr}"),
        dead_token_file,
        0,
        true,
    ))
    .unwrap();
    let dead_report = dead_tg
        .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
        .await
        .unwrap();
    assert_eq!(dead_report.fetched_count, 1);
    assert_eq!(dead_report.published_count, 0);
    assert_eq!(dead_report.failed_count, 1);
    assert_eq!(
        dead_report.items[0].error_code.as_deref(),
        Some("telegram_uncertain_network")
    );

    let failed_queue = dead_tg
        .fetch_hub_queue(
            &hub_url,
            DEVICE,
            Some("@procurali"),
            Some(TelegramDeliveryStatus::Failed),
            10,
        )
        .await
        .unwrap();
    assert_eq!(failed_queue.len(), 1);
}

#[tokio::test]
async fn multipart_photo_upload_from_local_minio_and_daemon_cycle_work() {
    let dir = tempfile::tempdir().unwrap();
    let hub_dir = dir.path().join("hub");
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
    let _hub_server = AbortOnDrop(tokio::spawn(async move {
        axum::serve(hub_listener, zaturo_hub::router(hub_state))
            .await
            .unwrap();
    }));

    let png_bytes = minimal_png_bytes(600, 600);
    let served_png = png_bytes.clone();
    let minio_router = Router::new().route(
        "/{bucket}/{*key}",
        get(move || {
            let bytes = served_png.clone();
            async move { (StatusCode::OK, bytes) }
        }),
    );
    let minio_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let minio_addr = minio_listener.local_addr().unwrap();
    let _minio_server = AbortOnDrop(tokio::spawn(async move {
        axum::serve(minio_listener, minio_router).await.unwrap();
    }));

    // Ingest offer queued under @procurali, then have a custom group ID claim & publish it.
    let batch = MercadoLivreIngestBatchRequest {
        worker_id: "worker-ml".into(),
        search_query: "ssd".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: 1_763_252_400_000,
        products: vec![sample_validated_product(
            "MLB39410080",
            "SSD Kingston NV3 1TB NVMe",
            44_990,
        )],
    };
    crate::mercadolivre::ingest_to_hub(&hub_url, DEVICE, &batch, "tg-mp-batch-1")
        .await
        .unwrap();

    let tg_state = MockTelegramState::default();
    tg_state.updates_queue.lock().unwrap().push(json!({
        "update_id": 201,
        "message": { "message_id": 21, "chat": { "id": -1001555666777i64 }, "text": "/ofertas" }
    }));
    let tg_router = Router::new()
        .route("/{bot_token}/{method}", post(mock_tg_method))
        .with_state(tg_state.clone());
    let tg_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tg_addr = tg_listener.local_addr().unwrap();
    let _tg_server = AbortOnDrop(tokio::spawn(async move {
        axum::serve(tg_listener, tg_router).await.unwrap();
    }));

    let env_path = dir.path().join("telegram.env");
    fs::write(
        &env_path,
        [
            "TELEGRAM_BOT_TOKEN=synthetic-987654:telegram_bot_token_value",
            "TELEGRAM_CHANNEL_ID=-1001555666777",
            "TELEGRAM_MIN_INTERVAL_SECS=1",
            "TELEGRAM_MAX_INTERVAL_SECS=2",
            "TELEGRAM_ALLOW_LIVE_PUBLISH=true",
        ]
        .join("\n"),
    )
    .unwrap();
    let mut cfg = test_tg_config(format!("http://{tg_addr}"), PathBuf::new(), 0, false);
    cfg.env_file = Some(env_path);
    cfg.local_minio_endpoint = Some(format!("http://{minio_addr}"));
    cfg.minio_bucket = Some("procurali-media".into());

    let tg = TelegramBotClient::from_config(cfg).unwrap();
    let daemon_rep = tg
        .run_telegram_daemon_cycles(&hub_url, DEVICE, 0, Duration::ZERO, Some(1))
        .await
        .unwrap();
    assert_eq!(daemon_rep.cycles_completed, 1);
    assert_eq!(daemon_rep.published_count, 1);
    assert_eq!(daemon_rep.failed_count, 0);
    assert_eq!(daemon_rep.commands_answered, 1);

    let multipart_calls = tg_state.send_photo_multipart_bodies.lock().unwrap();
    assert_eq!(multipart_calls.len(), 1);
    assert!(
        multipart_calls[0]
            .content_type
            .starts_with("multipart/form-data; boundary=")
    );
    let body_lossy = String::from_utf8_lossy(&multipart_calls[0].body);
    assert!(body_lossy.contains("-1001555666777"));
    assert!(body_lossy.contains("OFERTA RELÂMPAGO"));
    assert!(body_lossy.contains("filename=\"offer.png\""));
    assert!(
        multipart_calls[0]
            .body
            .windows(png_bytes.len())
            .any(|w| w == png_bytes.as_slice())
    );
}

fn write_token(dir: &std::path::Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("{BOT_TOKEN}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    path
}

async fn owner_approve(hub_url: &str, product_id: &str) {
    let approved: TelegramOfferQueueItem = reqwest::Client::builder()
        .no_proxy()
        .build()
        .unwrap()
        .post(format!(
            "{hub_url}/v1/commerce/mercadolivre/telegram-queue/approve"
        ))
        .bearer_auth(OWNER)
        .json(&json!({"product_id": product_id, "channel_id": "@procurali"}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(approved.status, TelegramDeliveryStatus::Queued);
}

#[derive(Clone, Default)]
struct FakeHub {
    deliveries: Arc<Mutex<Vec<TelegramOfferQueueItem>>>,
    acks: Arc<Mutex<Vec<(String, TelegramDeliveryStatus)>>>,
    fail_claims: Arc<std::sync::atomic::AtomicBool>,
    fail_results: Arc<std::sync::atomic::AtomicBool>,
}

async fn fake_queue(State(hub): State<FakeHub>) -> Json<Vec<TelegramOfferQueueItem>> {
    Json(
        hub.deliveries
            .lock()
            .unwrap()
            .iter()
            .filter(|item| item.status == TelegramDeliveryStatus::Queued)
            .cloned()
            .collect(),
    )
}

async fn fake_ack(
    State(hub): State<FakeHub>,
    AxumPath(id): AxumPath<String>,
    Json(ack): Json<TelegramDeliveryAck>,
) -> Result<Json<TelegramOfferQueueItem>, StatusCode> {
    use std::sync::atomic::Ordering;
    hub.acks.lock().unwrap().push((id.clone(), ack.status));
    let failing = if ack.status == TelegramDeliveryStatus::Sending {
        &hub.fail_claims
    } else {
        &hub.fail_results
    };
    if failing.load(Ordering::SeqCst) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    let mut deliveries = hub.deliveries.lock().unwrap();
    let item = deliveries
        .iter_mut()
        .find(|item| item.delivery_id == id)
        .ok_or(StatusCode::NOT_FOUND)?;
    item.status = ack.status;
    Ok(Json(item.clone()))
}

fn queued_item(id: &str) -> TelegramOfferQueueItem {
    TelegramOfferQueueItem {
        delivery_id: id.into(),
        product_id: "MLB39410080".into(),
        offer_id: format!("obs-{id}"),
        channel_id: "@procurali".into(),
        title: "SSD Kingston NV3 1TB".into(),
        brand: "Kingston".into(),
        model: None,
        price_brl_cents: 44_990,
        reference_price_cents: None,
        discount_label: None,
        share_link: "https://meli.la/1iWQZbJ".into(),
        share_id: "1PBCGF-SMN5".into(),
        image_public_url: Some(
            "https://media.procurali.com.br/procurali-media/ml/MLB39410080/hero.jpg".into(),
        ),
        image_object_key: Some("ml/MLB39410080/hero.jpg".into()),
        message_text: "SSD".into(),
        status: TelegramDeliveryStatus::Queued,
        external_message_id: None,
        created_at_ms: 1,
        updated_at_ms: 1,
        published_at_ms: None,
    }
}

#[tokio::test]
async fn publisher_claims_before_sending_and_never_resends_after_a_lost_ack() {
    use std::sync::atomic::Ordering;
    let dir = tempfile::tempdir().unwrap();
    let hub = FakeHub::default();
    hub.deliveries
        .lock()
        .unwrap()
        .extend([queued_item("tg-one"), queued_item("tg-two")]);
    let hub_router = Router::new()
        .route(
            "/v1/commerce/mercadolivre/telegram-queue",
            axum::routing::get(fake_queue),
        )
        .route(
            "/v1/commerce/mercadolivre/telegram-queue/{id}/ack",
            post(fake_ack),
        )
        .with_state(hub.clone());
    let hub_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let hub_url = format!("http://{}", hub_listener.local_addr().unwrap());
    let _hub = AbortOnDrop(tokio::spawn(async move {
        axum::serve(hub_listener, hub_router).await.unwrap();
    }));

    let tg_state = MockTelegramState::default();
    let tg_router = Router::new()
        .route("/{bot_token}/{method}", post(mock_tg_method))
        .with_state(tg_state.clone());
    let tg_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let tg_addr = tg_listener.local_addr().unwrap();
    let _tg = AbortOnDrop(tokio::spawn(async move {
        axum::serve(tg_listener, tg_router).await.unwrap();
    }));
    let tg = TelegramBotClient::from_config(test_tg_config(
        format!("http://{tg_addr}"),
        write_token(dir.path(), "telegram.token"),
        0,
        true,
    ))
    .unwrap();
    let sends = || tg_state.send_photo_calls.lock().unwrap().len();
    let status_of = |id: &str| {
        hub.deliveries
            .lock()
            .unwrap()
            .iter()
            .find(|item| item.delivery_id == id)
            .unwrap()
            .status
    };

    // The send succeeds but the Hub loses the result: the run stops and the delivery
    // stays claimed (`sending`) instead of returning to the queue.
    hub.fail_results.store(true, Ordering::SeqCst);
    let lost = tg
        .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
        .await
        .unwrap();
    assert_eq!(sends(), 1);
    assert_eq!(lost.published_count, 1);
    assert_eq!(lost.items.len(), 1);
    assert_eq!(
        lost.hub_error.as_deref(),
        Some("hub_ack_failed_delivery_left_sending")
    );
    assert_eq!(status_of("tg-one"), TelegramDeliveryStatus::Sending);
    assert_eq!(status_of("tg-two"), TelegramDeliveryStatus::Queued);
    assert_eq!(
        hub.acks.lock().unwrap()[0],
        ("tg-one".into(), TelegramDeliveryStatus::Sending),
        "the claim must precede the external send"
    );

    // A later run never sends the claimed delivery again.
    hub.fail_results.store(false, Ordering::SeqCst);
    let next = tg
        .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
        .await
        .unwrap();
    assert_eq!(sends(), 2);
    assert_eq!(next.hub_error, None);
    assert_eq!(next.items[0].delivery_id, "tg-two");
    assert_eq!(status_of("tg-one"), TelegramDeliveryStatus::Sending);
    assert_eq!(status_of("tg-two"), TelegramDeliveryStatus::Published);

    // Without a recorded claim nothing is sent.
    hub.deliveries.lock().unwrap().push(queued_item("tg-three"));
    hub.fail_claims.store(true, Ordering::SeqCst);
    let unclaimed = tg
        .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
        .await
        .unwrap();
    assert_eq!(sends(), 2);
    assert_eq!(unclaimed.hub_error.as_deref(), Some("hub_claim_failed"));
    assert!(unclaimed.items.is_empty());
}
