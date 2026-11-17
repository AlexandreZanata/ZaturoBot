//! Independent black-box contract tests for `zaturo_db` Mercado Livre persistence.
//!
//! Verifies schema 11 migration/constraints, ADR-0025 automatic Telegram queuing for approved
//! batches, ADR-0024 candidate approval gate, 30-day deduplication & price drops, pre-send
//! compare-and-set status transitions, and freshness checks.

use rusqlite::Connection;
use zaturo_core::{
    MercadoLivreIngestBatchRequest, MercadoLivreProductState, MercadoLivreSpec,
    MercadoLivreVariant, MinioProductImage, PriceTrendKind, TelegramDeliveryStatus,
    ValidatedMercadoLivreProduct, slugify_pt_br,
};
use zaturo_db::HubStore;

fn product_fixture(
    product_id: &str,
    price_cents: i64,
    observed_at_ms: i64,
    state: MercadoLivreProductState,
) -> ValidatedMercadoLivreProduct {
    let title = format!("Mouse Gamer Delux {product_id}");
    ValidatedMercadoLivreProduct {
        product_id: product_id.into(),
        slug: slugify_pt_br(&title, product_id),
        title,
        brand: "Delux".into(),
        model: Some("M900Pro".into()),
        exact_variant: "Preto 8000Hz".into(),
        category_slug: "hardware-e-perifericos".into(),
        canonical_url: format!("https://www.mercadolivre.com.br/mouse/p/{product_id}"),
        seller_name: "Loja Oficial".into(),
        share_link: "https://meli.la/31tuNW3".into(),
        share_id: "1PBCGF-SMN5".into(),
        share_text: "https://meli.la/31tuNW3".into(),
        commission_label: Some("6%".into()),
        commission_bp: Some(600),
        sponsored: false,
        price_brl_cents: price_cents,
        pix_price_cents: price_cents,
        reference_price_cents: Some(39_900),
        currency: "BRL".into(),
        marketplace_rating: Some("4,9 / 5".into()),
        marketplace_sales_label: Some("+5 mil vendidos".into()),
        card_badge: Some("Mais vendido".into()),
        discount_label: Some("25% OFF".into()),
        installment_summary: Some("10x sem juros".into()),
        payment_conditions: "A vista no Pix ou 10x sem juros".into(),
        short_condition: "10x sem juros".into(),
        freight_status: "Frete Full".into(),
        availability: "InStock".into(),
        price_valid_until: Some("2026-12-01".into()),
        return_days: Some(30),
        short_description: "Sensor PAW3395 com dock magnetico.".into(),
        long_description: "Mouse ergonomico leve sem fio com dongle 8kHz.".into(),
        one_line_reason: "Excelente sensor competitivo com dock.".into(),
        for_whom: "Jogadores competitivos de FPS.".into(),
        supported_benefits: vec!["Sensor PAW3395".into()],
        limitations: vec!["Peso ~63g sem cabo.".into()],
        unknowns: vec![],
        tags: vec!["mouse".into(), "gamer".into()],
        specs: vec![MercadoLivreSpec {
            name: "Sensor".into(),
            value: "PAW3395".into(),
        }],
        variants: vec![MercadoLivreVariant {
            variant_product_id: product_id.into(),
            sku: "M900PRO-BK".into(),
            name: "Preto".into(),
            color: Some("Preto".into()),
            voltage: None,
            price_brl_cents: price_cents,
            currency: "BRL".into(),
            availability: "InStock".into(),
            return_days: Some(30),
            return_fees: Some("Grátis".into()),
            canonical_url: None,
            highlights: vec!["8000Hz".into()],
        }],
        images: vec![MinioProductImage {
            position: 0,
            bucket: "procurali-media".into(),
            object_key: format!("mercadolivre/{product_id}/00-abc.jpg"),
            public_url: format!(
                "https://media.procurali.com.br/procurali-media/mercadolivre/{product_id}/00-abc.jpg"
            ),
            source_origin_url: None,
            content_type: "image/jpeg".into(),
            sha256_hex: "c".repeat(64),
            size_bytes: 16_000,
            alt_text: "Mouse Delux M900Pro".into(),
        }],
        observed_at_ms,
        freshness_ttl_minutes: 30,
        evidence_sha256: "d".repeat(64),
        state,
    }
}

#[test]
fn hub_schema_is_v11_and_approved_ingest_auto_queues_telegram_with_drop_alerts() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("hub.db");
    let store = HubStore::open(&db_path).unwrap();

    let version: i64 = Connection::open(&db_path)
        .unwrap()
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    assert_eq!(version, 11);

    let t0 = 1_763_231_280_000i64;
    let req1 = MercadoLivreIngestBatchRequest {
        worker_id: "procurali-local-pc".into(),
        search_query: "mouse gamer".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: t0,
        products: vec![
            product_fixture(
                "MLB63816534",
                29_986,
                t0,
                MercadoLivreProductState::Approved,
            ),
            product_fixture(
                "MLB72674962",
                2_099,
                t0,
                MercadoLivreProductState::Candidate,
            ),
        ],
    };

    let rep1 = store
        .transaction(|r| r.save_ml_ingest_batch("batch-1", "key-1", &"1".repeat(64), &req1, t0))
        .unwrap();
    // ADR-0025: Approved auto-queues (`queued`); ADR-0024: Candidate awaits approval (`awaiting_approval`).
    assert_eq!(rep1.telegram_queued_count, 1);
    assert_eq!(rep1.telegram_awaiting_approval_count, 1);
    assert_eq!(rep1.new_product_count, 2);

    let queued = store
        .read(|r| {
            r.ml_telegram_queue(
                None,
                Some(TelegramDeliveryStatus::Queued),
                Some(t0 + 1_000),
                10,
            )
        })
        .unwrap();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].product_id, "MLB63816534");

    // Pre-send compare-and-set: queued -> sending -> published, blocking duplicate claim.
    let del_id = queued[0].delivery_id.clone();
    let claimed = store
        .transaction(|r| {
            r.set_ml_telegram_delivery_status(
                &del_id,
                TelegramDeliveryStatus::Queued,
                TelegramDeliveryStatus::Sending,
                None,
                None,
                t0 + 2_000,
            )
        })
        .unwrap();
    assert!(claimed);
    let duplicate_claim = store
        .transaction(|r| {
            r.set_ml_telegram_delivery_status(
                &del_id,
                TelegramDeliveryStatus::Queued,
                TelegramDeliveryStatus::Sending,
                None,
                None,
                t0 + 2_100,
            )
        })
        .unwrap();
    assert!(!duplicate_claim);
    let published = store
        .transaction(|r| {
            r.set_ml_telegram_delivery_status(
                &del_id,
                TelegramDeliveryStatus::Sending,
                TelegramDeliveryStatus::Published,
                Some("tg-msg-101"),
                None,
                t0 + 3_000,
            )
        })
        .unwrap();
    assert!(published);

    let stats = store.read(|r| r.ml_telegram_product_stats()).unwrap();
    assert_eq!(stats.get("MLB63816534").map(|s| s.0), Some(1));

    // Re-ingest same SKU with unchanged price inside 30d window -> deduplicated (not queued again).
    let t1 = t0 + 3_600_000;
    let req_same = MercadoLivreIngestBatchRequest {
        worker_id: "procurali-local-pc".into(),
        search_query: "mouse gamer".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: t1,
        products: vec![product_fixture(
            "MLB63816534",
            29_986,
            t1,
            MercadoLivreProductState::Approved,
        )],
    };
    let rep_same = store
        .transaction(|r| r.save_ml_ingest_batch("batch-2", "key-2", &"2".repeat(64), &req_same, t1))
        .unwrap();
    assert_eq!(rep_same.telegram_queued_count, 0);
    assert_eq!(rep_same.unchanged_price_count, 1);

    // Re-ingest with significant price drop -> comparison trend Down + auto-requeued drop alert.
    let t2 = t1 + 3_600_000;
    let req_drop = MercadoLivreIngestBatchRequest {
        worker_id: "procurali-local-pc".into(),
        search_query: "mouse gamer".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: t2,
        products: vec![product_fixture(
            "MLB63816534",
            22_900,
            t2,
            MercadoLivreProductState::Approved,
        )],
    };
    let rep_drop = store
        .transaction(|r| r.save_ml_ingest_batch("batch-3", "key-3", &"3".repeat(64), &req_drop, t2))
        .unwrap();
    assert_eq!(rep_drop.price_drop_count, 1);
    assert_eq!(rep_drop.telegram_queued_count, 1);
    assert_eq!(rep_drop.comparisons[0].trend, PriceTrendKind::Down);

    let queued_after_drop = store
        .read(|r| {
            r.ml_telegram_queue(
                None,
                Some(TelegramDeliveryStatus::Queued),
                Some(t2 + 1_000),
                10,
            )
        })
        .unwrap();
    assert_eq!(queued_after_drop.len(), 1);
    assert!(
        queued_after_drop[0]
            .message_text
            .contains("BAIXOU DE PREÇO")
    );

    // Site catalog excludes `withdrawn` and exposes both active offers with exact state metadata
    // (`approved` vs `candidate`), while only `approved` auto-queued into Telegram.
    let catalog = store.read(|r| r.ml_site_catalog(t2 + 1_000)).unwrap();
    assert_eq!(catalog.offers.len(), 2);
    let mlb638 = catalog
        .offers
        .iter()
        .find(|o| o.product_id == "MLB63816534")
        .unwrap();
    assert_eq!(mlb638.state, "approved");
    assert_eq!(mlb638.price_brl_cents, 22_900);
    assert!(mlb638.price_observations.len() >= 2);
    let mlb726 = catalog
        .offers
        .iter()
        .find(|o| o.product_id == "MLB72674962")
        .unwrap();
    assert_eq!(mlb726.state, "candidate");
}

#[test]
fn idempotency_lookup_and_stale_freshness_contract_hold() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path().join("hub.db")).unwrap();
    let t0 = 1_763_231_280_000i64;
    let req = MercadoLivreIngestBatchRequest {
        worker_id: "worker-ml".into(),
        search_query: "ssd".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: t0,
        products: vec![product_fixture(
            "MLB51383856",
            5_899,
            t0,
            MercadoLivreProductState::Candidate,
        )],
    };
    store
        .transaction(|r| r.save_ml_ingest_batch("b1", "idem-1", &"a".repeat(64), &req, t0))
        .unwrap();

    let found = store
        .read(|r| r.ml_batch_by_idempotency_key("idem-1"))
        .unwrap()
        .expect("idempotent batch lookup");
    assert_eq!(found.0, "a".repeat(64));

    let cur = store
        .read(|r| r.ml_current_telegram_delivery("MLB51383856", "@procurali"))
        .unwrap()
        .expect("delivery row");
    assert_eq!(cur.status, TelegramDeliveryStatus::AwaitingApproval);
    assert!(
        store
            .read(|r| r.ml_telegram_delivery_offer_fresh(&cur.delivery_id, t0 + 10_000))
            .unwrap()
    );
    // Beyond 30 min freshness TTL -> false.
    assert!(
        !store
            .read(|r| r.ml_telegram_delivery_offer_fresh(&cur.delivery_id, t0 + 31 * 60_000))
            .unwrap()
    );
}
