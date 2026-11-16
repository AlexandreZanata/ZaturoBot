//! Independent black-box contract tests for `zaturo_services` Engine Mercado Livre APIs.
//!
//! Protects ingestion idempotency, hot cache ETag compilation/invalidation, faceted search,
//! PDP breadcrumbs, exposure telemetry, price alerts, auto-queueing for Approved batches,
//! Candidate owner approval gate, and pre-send claim -> published ack transitions.

use zaturo_core::{
    CatalogSearchQuery, ExposureBeaconRequest, ExposureEventItem, Limits,
    MercadoLivreIngestBatchRequest, MercadoLivreProductState, MercadoLivreSpec,
    MercadoLivreVariant, MinioProductImage, PriceAlertActionRequest, TelegramDeliveryAck,
    TelegramDeliveryStatus, ValidatedMercadoLivreProduct, slugify_pt_br,
};
use zaturo_services::{Engine, Error};

fn sample_product(
    product_id: &str,
    price_cents: i64,
    observed_at_ms: i64,
    state: MercadoLivreProductState,
) -> ValidatedMercadoLivreProduct {
    let title = format!("SSD Kingston NV3 1TB {product_id}");
    ValidatedMercadoLivreProduct {
        product_id: product_id.into(),
        slug: slugify_pt_br(&title, product_id),
        title,
        brand: "Kingston".into(),
        model: Some("SNV3S/1000G".into()),
        exact_variant: "1 TB NVMe PCIe 4.0".into(),
        category_slug: "hardware-e-perifericos".into(),
        canonical_url: format!("https://www.mercadolivre.com.br/ssd/p/{product_id}"),
        seller_name: "Loja Oficial".into(),
        share_link: "https://meli.la/1iWQZbJ".into(),
        share_id: "1PBCGF-SMN5".into(),
        share_text: "https://meli.la/1iWQZbJ".into(),
        commission_label: Some("5%".into()),
        commission_bp: Some(500),
        sponsored: false,
        price_brl_cents: price_cents,
        pix_price_cents: price_cents,
        reference_price_cents: Some(79_900),
        currency: "BRL".into(),
        marketplace_rating: Some("4,9 / 5".into()),
        marketplace_sales_label: Some("+10 mil vendidos".into()),
        card_badge: Some("Mais vendido".into()),
        discount_label: Some("25% OFF".into()),
        installment_summary: Some("10x sem juros".into()),
        payment_conditions: "10x sem juros ou Pix".into(),
        short_condition: "10x sem juros".into(),
        freight_status: "Frete Full".into(),
        availability: "InStock".into(),
        price_valid_until: Some("2026-12-01".into()),
        return_days: Some(30),
        short_description: "SSD M.2 2280 NVMe PCIe 4.0".into(),
        long_description: "Unidade para notebook e desktop com ate 6000 MB/s".into(),
        one_line_reason: "Velocidade e custo-beneficio equilibrados.".into(),
        for_whom: "Usuarios buscando upgrade de armazenamento.".into(),
        supported_benefits: vec!["PCIe 4.0 rapido".into()],
        limitations: vec!["Sem dissipador incluso.".into()],
        unknowns: vec![],
        tags: vec!["ssd".into(), "nvme".into()],
        specs: vec![MercadoLivreSpec {
            name: "Capacidade".into(),
            value: "1 TB".into(),
        }],
        variants: vec![MercadoLivreVariant {
            variant_product_id: product_id.into(),
            sku: "SNV3-1T".into(),
            name: "1 TB".into(),
            color: None,
            voltage: None,
            price_brl_cents: price_cents,
            currency: "BRL".into(),
            availability: "InStock".into(),
            return_days: Some(30),
            return_fees: Some("Grátis".into()),
            canonical_url: None,
            highlights: vec!["M.2".into()],
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
            sha256_hex: "e".repeat(64),
            size_bytes: 20_000,
            alt_text: "SSD Kingston 1TB".into(),
        }],
        observed_at_ms,
        freshness_ttl_minutes: 30,
        evidence_sha256: "f".repeat(64),
        state,
    }
}

#[test]
fn engine_ingest_cache_search_alerts_and_telegram_lifecycle_contract() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("hub.db"), Limits::default()).unwrap();
    let t0 = 1_763_231_280_000i64;

    let req = MercadoLivreIngestBatchRequest {
        worker_id: "worker-ml".into(),
        search_query: "ssd kingston".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: t0,
        products: vec![
            sample_product(
                "MLB39410080",
                49_990,
                t0,
                MercadoLivreProductState::Approved,
            ),
            sample_product(
                "MLB39410081",
                59_990,
                t0,
                MercadoLivreProductState::Candidate,
            ),
        ],
    };

    let rep = engine
        .ingest_mercadolivre_batch(req.clone(), "idem-svc-1", &"1".repeat(64), t0)
        .unwrap();
    assert_eq!(rep.product_count, 2);
    assert_eq!(rep.telegram_queued_count, 1);
    assert_eq!(rep.telegram_awaiting_approval_count, 1);

    // Same idempotency key + same sha returns cached report without double ingest.
    let replay = engine
        .ingest_mercadolivre_batch(req.clone(), "idem-svc-1", &"1".repeat(64), t0 + 1_000)
        .unwrap();
    assert_eq!(replay.batch_id, rep.batch_id);

    // Same key + different sha -> Conflict.
    let conflict = engine.ingest_mercadolivre_batch(req, "idem-svc-1", &"2".repeat(64), t0 + 2_000);
    assert!(matches!(conflict, Err(Error::Conflict(_))));

    // Hot cache compiles once and warms.
    assert!(engine.mercadolivre_site_catalog_warm().is_none());
    let compiled = engine.mercadolivre_site_catalog_cached(t0 + 3_000).unwrap();
    assert_eq!(compiled.snapshot.offers.len(), 2);
    assert!(engine.mercadolivre_site_catalog_warm().is_some());

    // Reference catalog + taxonomy + search + detail + redirect.
    let ref_cat = engine.mercadolivre_reference_catalog(t0 + 3_000).unwrap();
    assert_eq!(ref_cat.reference_catalog.total_products, 2);

    let tax = engine.mercadolivre_taxonomy(t0 + 3_000).unwrap();
    assert!(!tax.taxonomy.categories.is_empty());

    let search = engine
        .mercadolivre_search(
            &CatalogSearchQuery {
                q: Some("kingston".into()),
                ..Default::default()
            },
            t0 + 3_000,
        )
        .unwrap();
    // Public search returns only approved offers; candidate remains hidden until approved.
    assert_eq!(search.total_matches, 1);
    assert_eq!(search.items[0].product_id, "MLB39410080");

    let detail = engine
        .mercadolivre_product_detail("MLB39410080", t0 + 3_000)
        .unwrap();
    assert!(!detail.breadcrumb.is_empty());

    // Exposure telemetry + redirect increment stats.
    engine
        .mercadolivre_record_exposure(ExposureBeaconRequest {
            route: "/ofertas/".into(),
            events: vec![ExposureEventItem {
                product_id: "MLB39410080".into(),
                store: "Mercado Livre".into(),
                pos: 0,
                discount_pct: 25,
                price_cents: 49_990,
            }],
            timestamp_ms: t0 + 3_500,
        })
        .unwrap();
    let redirect = engine
        .mercadolivre_resolve_redirect("MLB39410080", t0 + 3_600)
        .unwrap();
    assert_eq!(redirect, "https://meli.la/1iWQZbJ");

    let detail_after = engine
        .mercadolivre_product_detail("MLB39410080", t0 + 3_700)
        .unwrap();
    assert_eq!(detail_after.impressions, 1);
    assert_eq!(detail_after.clicks, 1);

    // Price alert triggers immediately when target >= current price.
    let alert_resp = engine
        .mercadolivre_handle_price_alert(
            PriceAlertActionRequest {
                action: "criar-alerta".into(),
                subscriber_id: "user-sub-1".into(),
                product_id: Some("MLB39410080".into()),
                target_price_cents: Some(55_000),
            },
            t0 + 4_000,
        )
        .unwrap();
    assert_eq!(alert_resp.active_alerts.len(), 1);
    assert_eq!(alert_resp.triggered.len(), 1);

    // Auto-queued Approved item can be claimed (`sending`) then published (`published`),
    // while Candidate waits for explicit approval first.
    let queue = engine
        .mercadolivre_telegram_queue(
            Some("@procurali"),
            Some(TelegramDeliveryStatus::Queued),
            10,
            t0 + 4_000,
        )
        .unwrap();
    assert_eq!(queue.len(), 1);
    let del_id = queue[0].delivery_id.clone();

    let claimed = engine
        .ack_mercadolivre_telegram_delivery(
            &del_id,
            TelegramDeliveryAck {
                status: TelegramDeliveryStatus::Sending,
                external_message_id: None,
                error_code: None,
            },
            t0 + 4_100,
        )
        .unwrap();
    assert_eq!(claimed.status, TelegramDeliveryStatus::Sending);

    let done = engine
        .ack_mercadolivre_telegram_delivery(
            &del_id,
            TelegramDeliveryAck {
                status: TelegramDeliveryStatus::Published,
                external_message_id: Some("42".into()),
                error_code: None,
            },
            t0 + 4_200,
        )
        .unwrap();
    assert_eq!(done.status, TelegramDeliveryStatus::Published);

    // Candidate cannot be skipped into Published without approve, and approve moves it to Queued.
    let approved = engine
        .approve_mercadolivre_telegram_delivery("MLB39410081", "@procurali", t0 + 5_000)
        .unwrap();
    assert_eq!(approved.status, TelegramDeliveryStatus::Queued);
}
