use crate::{Engine, Error};
use zaturo_core::*;

fn sample_product(product_id: &str, slug: &str) -> ValidatedMercadoLivreProduct {
    ValidatedMercadoLivreProduct {
        product_id: product_id.to_owned(),
        slug: slug.to_owned(),
        title: "SSD Kingston NV3 500GB M.2 2280 NVMe PCIe 4.0".into(),
        brand: "Kingston".into(),
        model: Some("SNV3S/500G".into()),
        exact_variant: "SNV3S/500G · 500 GB · M.2 2280".into(),
        category_slug: "hardware-e-perifericos".into(),
        canonical_url: format!(
            "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/{product_id}"
        ),
        seller_name: "Loja oficial FAMILAR".into(),
        share_link: "https://meli.la/1iWQZbJ".into(),
        share_id: "1PBCGF-SMN5".into(),
        share_text: "Código 1PBCGF-SMN5\nhttps://meli.la/1iWQZbJ".into(),
        commission_label: Some("5%".into()),
        commission_bp: Some(500),
        sponsored: false,
        price_brl_cents: 69_990,
        pix_price_cents: 69_990,
        reference_price_cents: Some(137_900),
        currency: "BRL".into(),
        marketplace_rating: Some("4,9 / 5".into()),
        marketplace_sales_label: Some("+10 mil vendidos no ML".into()),
        card_badge: Some("Mais vendido".into()),
        discount_label: Some("49% OFF".into()),
        installment_summary: Some("18x de R$ 46,26".into()),
        payment_conditions: "R$ 699,90 a vista ou em ate 18x de R$ 46,26".into(),
        short_condition: "18x R$ 46,26 · devolucao gratis 30d".into(),
        freight_status: "Frete calculado no CEP; devolucao gratis 30 dias.".into(),
        availability: "InStock".into(),
        price_valid_until: Some("2026-11-18".into()),
        return_days: Some(30),
        short_description: "M.2 2280 NVMe PCIe 4.0 x4 · leitura ate 5.000 MB/s.".into(),
        long_description: "SSD interno Kingston NV3 500 GB com memoria 3D NAND.".into(),
        one_line_reason: "SSD PCIe 4.0 para notebook e desktop com leitura de 5.000 MB/s.".into(),
        for_whom: "Quem precisa atualizar slot M.2 2280 com boa velocidade.".into(),
        supported_benefits: vec!["Leitura ate 5.000 MB/s.".into()],
        limitations: vec!["Nao acompanha dissipador de calor.".into()],
        unknowns: vec!["Serie continua de 7 dias ainda em acumulacao.".into()],
        tags: vec!["ssd".into(), "nvme".into(), "kingston".into()],
        specs: vec![
            MercadoLivreSpec {
                name: "Marca".into(),
                value: "Kingston".into(),
            },
            MercadoLivreSpec {
                name: "Capacidade".into(),
                value: "500 GB".into(),
            },
        ],
        variants: vec![MercadoLivreVariant {
            variant_product_id: product_id.to_owned(),
            sku: "MLB7607021422".into(),
            name: "SSD Kingston NV3 500GB".into(),
            color: Some("Azul-aco".into()),
            voltage: None,
            price_brl_cents: 69_800,
            currency: "BRL".into(),
            availability: "InStock".into(),
            return_days: Some(30),
            return_fees: Some("Gratis".into()),
            canonical_url: Some(format!(
                "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/{product_id}"
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
            size_bytes: 12_345,
            alt_text: "SSD Kingston NV3 500GB".into(),
        }],
        observed_at_ms: 1_763_231_280_000,
        freshness_ttl_minutes: 30,
        evidence_sha256: "b".repeat(64),
        state: MercadoLivreProductState::Approved,
    }
}

#[test]
fn ingests_with_idempotency_conflict_detection_feed_catalog_and_telegram_ack() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("hub.db"), Limits::default()).unwrap();

    let mut candidate_product = sample_product("MLB39410080", "ssd-kingston-nv3-500gb-mlb39410080");
    candidate_product.state = MercadoLivreProductState::Candidate;
    let req = MercadoLivreIngestBatchRequest {
        worker_id: "worker-ml".into(),
        search_query: "ssd nvme".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: 1_763_231_280_000,
        products: vec![candidate_product],
    };
    let sha_a = "c".repeat(64);
    let sha_b = "d".repeat(64);

    let first = engine
        .ingest_mercadolivre_batch(req.clone(), "batch-idem-1", &sha_a, 1_763_231_290_000)
        .unwrap();
    assert_eq!(first.product_count, 1);
    assert_eq!(first.telegram_queued_count, 0);
    assert_eq!(first.telegram_awaiting_approval_count, 1);

    let replay = engine
        .ingest_mercadolivre_batch(req.clone(), "batch-idem-1", &sha_a, 1_763_231_295_000)
        .unwrap();
    assert_eq!(replay, first);

    let conflict = engine.ingest_mercadolivre_batch(req, "batch-idem-1", &sha_b, 1_763_231_299_000);
    assert!(matches!(conflict, Err(Error::Conflict(_))));

    let feed = engine
        .feed(FeedQuery {
            day: "2025-11-15".into(),
            offset_minutes: -240,
            kind: Some(FeedKind::Service),
            q: Some("Mercado Livre batch".into()),
            limit: 20,
            offset: 0,
        })
        .unwrap();
    assert_eq!(feed.items.len(), 1);
    assert!(!feed.items[0].sensitive);

    let catalog = engine.mercadolivre_site_catalog(1_763_231_300_000).unwrap();
    assert_eq!(catalog.offers.len(), 1);
    assert_eq!(
        catalog.offers[0].official_affiliate_url,
        "https://meli.la/1iWQZbJ"
    );

    let now = 1_763_231_350_000;
    let queued = |engine: &Engine| {
        engine
            .mercadolivre_telegram_queue(
                Some("@procurali"),
                Some(TelegramDeliveryStatus::Queued),
                20,
                now,
            )
            .unwrap()
    };
    assert!(
        queued(&engine).is_empty(),
        "ingest must not approve publication"
    );

    let ack = |status| TelegramDeliveryAck {
        status,
        external_message_id: None,
        error_code: None,
    };
    let awaiting = engine
        .mercadolivre_telegram_queue(
            Some("@procurali"),
            Some(TelegramDeliveryStatus::AwaitingApproval),
            20,
            now,
        )
        .unwrap();
    assert_eq!(awaiting.len(), 1);
    let id = awaiting[0].delivery_id.clone();
    for status in [
        TelegramDeliveryStatus::Queued,
        TelegramDeliveryStatus::Sending,
    ] {
        assert!(matches!(
            engine.ack_mercadolivre_telegram_delivery(&id, ack(status), now),
            Err(Error::Conflict(_))
        ));
    }

    let approved = engine
        .approve_mercadolivre_telegram_delivery("MLB39410080", "@procurali", now)
        .unwrap();
    assert_eq!(approved.status, TelegramDeliveryStatus::Queued);
    assert_eq!(queued(&engine).len(), 1);
    assert_eq!(
        engine
            .approve_mercadolivre_telegram_delivery("MLB39410080", "@procurali", now)
            .unwrap(),
        approved
    );

    assert!(matches!(
        engine.ack_mercadolivre_telegram_delivery(&id, ack(TelegramDeliveryStatus::Published), now),
        Err(Error::Conflict(_))
    ));
    let claimed = engine
        .ack_mercadolivre_telegram_delivery(&id, ack(TelegramDeliveryStatus::Sending), now)
        .unwrap();
    assert_eq!(claimed.status, TelegramDeliveryStatus::Sending);
    assert!(queued(&engine).is_empty());
    assert!(matches!(
        engine.ack_mercadolivre_telegram_delivery(&id, ack(TelegramDeliveryStatus::Sending), now),
        Err(Error::Conflict(_))
    ));

    let acked = engine
        .ack_mercadolivre_telegram_delivery(
            &id,
            TelegramDeliveryAck {
                status: TelegramDeliveryStatus::Published,
                external_message_id: Some("msg-42".into()),
                error_code: None,
            },
            now,
        )
        .unwrap();
    assert_eq!(acked.status, TelegramDeliveryStatus::Published);
    assert_eq!(acked.external_message_id.as_deref(), Some("msg-42"));
    assert_eq!(
        engine
            .ack_mercadolivre_telegram_delivery(
                &id,
                ack(TelegramDeliveryStatus::Published),
                now + 1
            )
            .unwrap(),
        acked
    );

    // Cloobe-inspired search, taxonomy, PDP, redirect click tracking, exposure & price alerts:
    let cached = engine
        .mercadolivre_site_catalog_cached(1_763_231_360_000)
        .unwrap();
    assert_eq!(cached.generation, 2);

    let search = engine
        .mercadolivre_search(
            &CatalogSearchQuery {
                q: Some("kingston".into()),
                subcategoria: Some("ssd-e-armazenamento".into()),
                formato: Some("html-fragment".into()),
                ..Default::default()
            },
            1_763_231_360_000,
        )
        .unwrap();
    assert_eq!(search.total_matches, 1);
    assert!(
        search
            .html_fragment
            .as_deref()
            .unwrap()
            .contains("<article class=\"offer-card\"")
    );

    let alert = engine
        .mercadolivre_handle_price_alert(
            PriceAlertActionRequest {
                action: "criar-alerta".into(),
                subscriber_id: "sub_01".into(),
                product_id: Some("MLB39410080".into()),
                target_price_cents: Some(70_000),
            },
            1_763_231_365_000,
        )
        .unwrap();
    assert_eq!(alert.active_alerts.len(), 1);
    assert_eq!(alert.triggered.len(), 1);

    let link = engine
        .mercadolivre_resolve_redirect("MLB39410080", 1_763_231_370_000)
        .unwrap();
    assert_eq!(link, "https://meli.la/1iWQZbJ");

    let pdp = engine
        .mercadolivre_product_detail("MLB39410080", 1_763_231_375_000)
        .unwrap();
    assert_eq!(pdp.clicks, 1);
    assert_eq!(pdp.alert_count, 1);
    assert!(pdp.offer.price_intelligence.is_some());

    // Reference catalog warm cache hit reflects Telegram ack, and invalidates + updates on price drop:
    let ref_before = engine
        .mercadolivre_reference_catalog(1_763_231_376_000)
        .unwrap();
    assert_eq!(ref_before.reference_catalog.total_products, 1);
    assert_eq!(
        ref_before.reference_catalog.items[0].telegram_published_count,
        1
    );
    assert_eq!(
        ref_before.reference_catalog.items[0]
            .telegram_last_status
            .as_deref(),
        Some("published")
    );

    let day2_ms = 1_763_231_280_000 + 86_400_000;
    let mut p_drop = sample_product("MLB39410080", "ssd-kingston-nv3-500gb-mlb39410080");
    p_drop.price_brl_cents = 59_990;
    p_drop.pix_price_cents = 59_990;
    p_drop.observed_at_ms = day2_ms;
    let drop_req = MercadoLivreIngestBatchRequest {
        worker_id: "worker-ml".into(),
        search_query: "ssd nvme".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: day2_ms,
        products: vec![p_drop],
    };
    let drop_report = engine
        .ingest_mercadolivre_batch(drop_req, "batch-idem-drop", &"d".repeat(64), day2_ms)
        .unwrap();
    assert_eq!(drop_report.price_drop_count, 1);
    assert_eq!(drop_report.telegram_queued_count, 1);

    let ref_after = engine
        .mercadolivre_reference_catalog(day2_ms + 1000)
        .unwrap();
    assert_ne!(ref_before.reference_etag, ref_after.reference_etag);
    let item = &ref_after.reference_catalog.items[0];
    assert_eq!(item.price_trend, zaturo_core::PriceTrendKind::Down);
    assert_eq!(item.previous_price_cents, Some(69_990));
    assert_eq!(item.price_delta_cents, -10_000);
    assert!(item.short_summary.contains("Queda"));
    assert_eq!(item.telegram_last_status.as_deref(), Some("queued"));
}

#[test]
fn stale_or_superseded_offers_are_never_approved_or_claimed() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(dir.path().join("hub.db"), Limits::default()).unwrap();
    let request = |captured| {
        let mut p = sample_product("MLB39410080", "ssd-kingston-nv3-500gb-mlb39410080");
        p.state = MercadoLivreProductState::Candidate;
        MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "ssd nvme".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: captured,
            products: vec![p],
        }
    };
    let observed = 1_763_231_280_000_i64;
    let expired = observed + 30 * 60_000 + 1;
    engine
        .ingest_mercadolivre_batch(request(observed), "a", &"c".repeat(64), observed + 1)
        .unwrap();
    assert!(matches!(
        engine.approve_mercadolivre_telegram_delivery("MLB39410080", "@procurali", expired),
        Err(Error::Conflict(_))
    ));

    let approved = engine
        .approve_mercadolivre_telegram_delivery("MLB39410080", "@procurali", observed + 2)
        .unwrap();
    let claim = TelegramDeliveryAck {
        status: TelegramDeliveryStatus::Sending,
        external_message_id: None,
        error_code: None,
    };
    let stale = engine
        .ack_mercadolivre_telegram_delivery(&approved.delivery_id, claim.clone(), expired)
        .unwrap();
    assert_eq!(stale.status, TelegramDeliveryStatus::Skipped);

    // A newer observation supersedes an approved but unsent delivery.
    engine
        .ingest_mercadolivre_batch(request(observed), "b", &"d".repeat(64), observed + 3)
        .unwrap();
    let second = engine
        .approve_mercadolivre_telegram_delivery("MLB39410080", "@procurali", observed + 4)
        .unwrap();
    assert_ne!(second.delivery_id, approved.delivery_id);
    engine
        .ingest_mercadolivre_batch(request(observed), "c", &"e".repeat(64), observed + 5)
        .unwrap();
    let superseded =
        engine.ack_mercadolivre_telegram_delivery(&second.delivery_id, claim, observed + 6);
    assert!(matches!(superseded, Err(Error::Conflict(_))));
    assert!(
        engine
            .mercadolivre_telegram_queue(
                Some("@procurali"),
                Some(TelegramDeliveryStatus::Queued),
                20,
                observed + 6,
            )
            .unwrap()
            .is_empty()
    );
}
