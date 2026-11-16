use crate::HubStore;
use zaturo_core::{
    MercadoLivreIngestBatchRequest, MercadoLivreProductState, MercadoLivreSpec,
    MercadoLivreVariant, MinioProductImage, PriceTrendKind, TelegramDeliveryStatus,
    ValidatedMercadoLivreProduct,
};

fn sample_product(product_id: &str, slug: &str, price_cents: i64) -> ValidatedMercadoLivreProduct {
    ValidatedMercadoLivreProduct {
        product_id: product_id.into(),
        slug: slug.into(),
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
        price_brl_cents: price_cents,
        pix_price_cents: price_cents,
        reference_price_cents: Some(137_900),
        currency: "BRL".into(),
        marketplace_rating: Some("4,9 / 5".into()),
        marketplace_sales_label: Some("+10 mil vendidos no ML".into()),
        card_badge: Some("Mais vendido".into()),
        discount_label: Some("49% OFF".into()),
        installment_summary: Some("18× de R$ 46,26".into()),
        payment_conditions: "R$ 699,90 à vista ou em até 18× de R$ 46,26".into(),
        short_condition: "18× R$ 46,26 · devolução grátis 30d".into(),
        freight_status: "Frete calculado no CEP; devolução grátis 30 dias.".into(),
        availability: "InStock".into(),
        price_valid_until: Some("2026-11-18".into()),
        return_days: Some(30),
        short_description: "M.2 2280 NVMe PCIe 4.0 x4 · leitura até 5.000 MB/s.".into(),
        long_description: "SSD interno Kingston NV3 500 GB com memória 3D NAND.".into(),
        one_line_reason: "SSD PCIe 4.0 para notebook e desktop com leitura de 5.000 MB/s.".into(),
        for_whom: "Quem precisa atualizar slot M.2 2280 com boa velocidade.".into(),
        supported_benefits: vec!["Leitura até 5.000 MB/s.".into()],
        limitations: vec!["Não acompanha dissipador de calor.".into()],
        unknowns: vec!["Série contínua de 7 dias ainda em acumulação.".into()],
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
            variant_product_id: product_id.into(),
            sku: "MLB7607021422".into(),
            name: "SSD Kingston NV3 500GB".into(),
            color: Some("Azul-aço".into()),
            voltage: None,
            price_brl_cents: 69_800,
            currency: "BRL".into(),
            availability: "InStock".into(),
            return_days: Some(30),
            return_fees: Some("Grátis".into()),
            canonical_url: Some(format!(
                "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/{product_id}"
            )),
            highlights: vec!["Formato M.2 2280".into()],
        }],
        images: vec![MinioProductImage {
            position: 0,
            bucket: "procurali-media".into(),
            object_key: format!("mercadolivre/{product_id}/00-aaaaaaaaaaaaaaaa.jpg"),
            public_url: format!(
                "https://media.procurali.com.br/procurali-media/mercadolivre/{product_id}/00-aaaaaaaaaaaaaaaa.jpg"
            ),
            source_origin_url: Some("https://http2.mlstatic.com/D_NQ_NP_sample.jpg".into()),
            content_type: "image/jpeg".into(),
            sha256_hex: "a".repeat(64),
            size_bytes: 12_345,
            alt_text: "Embalagem do SSD Kingston NV3 500GB".into(),
        }],
        observed_at_ms: 1_763_231_280_000,
        freshness_ttl_minutes: 30,
        evidence_sha256: "b".repeat(64),
        state: MercadoLivreProductState::Approved,
    }
}

#[test]
fn stores_products_specs_variants_minio_images_and_serves_site_and_telegram_queue() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path().join("hub.db")).unwrap();
    let mut candidate = sample_product("MLB39410080", "ssd-kingston-nv3-500gb-mlb39410080", 69_990);
    candidate.state = MercadoLivreProductState::Candidate;
    let req = MercadoLivreIngestBatchRequest {
        worker_id: "worker-ml".into(),
        search_query: "ssd nvme".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: 1_763_231_280_000,
        products: vec![candidate],
    };
    req.validates().unwrap();

    let report = store
        .transaction(|repo| {
            repo.save_ml_ingest_batch(
                "batch-1",
                "idem-1",
                &"c".repeat(64),
                &req,
                1_763_231_290_000,
            )
        })
        .unwrap();
    assert_eq!(report.product_count, 1);
    assert_eq!(report.image_count, 1);
    assert_eq!(report.telegram_queued_count, 0);
    assert_eq!(report.telegram_awaiting_approval_count, 1);

    let catalog = store
        .read(|repo| repo.ml_site_catalog(1_763_231_300_000))
        .unwrap();
    assert_eq!(catalog.offers.len(), 1);
    assert_eq!(catalog.offers[0].specs.len(), 2);
    assert_eq!(catalog.offers[0].variants.len(), 1);
    assert!(
        catalog.offers[0]
            .image_path
            .starts_with("https://media.procurali.com.br/")
    );

    let awaiting = store
        .read(|repo| {
            repo.ml_telegram_queue(
                Some("@procurali"),
                Some(TelegramDeliveryStatus::AwaitingApproval),
                None,
                20,
            )
        })
        .unwrap();
    assert_eq!(awaiting.len(), 1);
    assert!(awaiting[0].message_text.contains("https://meli.la/1iWQZbJ"));
    let id = awaiting[0].delivery_id.clone();

    // Compare-and-set refuses a transition from a status the row is not in.
    let wrong_from = store
        .transaction(|repo| {
            repo.set_ml_telegram_delivery_status(
                &id,
                TelegramDeliveryStatus::Queued,
                TelegramDeliveryStatus::Sending,
                None,
                None,
                1_763_231_400_000,
            )
        })
        .unwrap();
    assert!(!wrong_from);

    // Re-ingesting the same product supersedes the unsent delivery.
    store
        .transaction(|repo| {
            repo.save_ml_ingest_batch(
                "batch-2",
                "idem-2",
                &"d".repeat(64),
                &req,
                1_763_231_410_000,
            )
        })
        .unwrap();
    let old = store
        .read(|repo| repo.ml_telegram_delivery(&id))
        .unwrap()
        .unwrap();
    assert_eq!(old.status, TelegramDeliveryStatus::Skipped);
    let current = store
        .read(|repo| repo.ml_current_telegram_delivery("MLB39410080", "@procurali"))
        .unwrap()
        .unwrap();
    assert_ne!(current.delivery_id, id);
    assert_eq!(current.status, TelegramDeliveryStatus::AwaitingApproval);

    // Freshness is bounded by observed_at_ms + freshness_ttl_minutes.
    let observed = 1_763_231_280_000_i64;
    let fresh = |now| {
        store
            .read(|repo| repo.ml_telegram_delivery_offer_fresh(&current.delivery_id, now))
            .unwrap()
    };
    assert!(fresh(observed + 30 * 60_000));
    assert!(!fresh(observed + 30 * 60_000 + 1));
    assert!(!fresh_old(&store, &id, observed));

    let updated = store
        .transaction(|repo| {
            assert!(repo.set_ml_telegram_delivery_status(
                &current.delivery_id,
                TelegramDeliveryStatus::AwaitingApproval,
                TelegramDeliveryStatus::Published,
                Some("98765"),
                None,
                1_763_231_420_000,
            )?);
            repo.ml_telegram_delivery(&current.delivery_id)
        })
        .unwrap()
        .unwrap();
    assert_eq!(updated.status, TelegramDeliveryStatus::Published);
    assert_eq!(updated.published_at_ms, Some(1_763_231_420_000));
    assert_eq!(updated.external_message_id.as_deref(), Some("98765"));
}

fn fresh_old(store: &HubStore, id: &str, now: i64) -> bool {
    store
        .read(|repo| repo.ml_telegram_delivery_offer_fresh(id, now))
        .unwrap()
}

#[test]
fn tracks_price_history_comparisons_30d_dedup_and_smart_telegram_alerts() {
    let dir = tempfile::tempdir().unwrap();
    let store = HubStore::open(dir.path().join("hub.db")).unwrap();
    let day1 = 1_763_200_000_000i64;

    let make_req = |price_cents: i64, ts: i64| -> MercadoLivreIngestBatchRequest {
        let mut p = sample_product("MLB63816534", "delux-m900pro-mlb63816534", price_cents);
        p.observed_at_ms = ts;
        let req = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "mouse gamer".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: ts,
            products: vec![p],
        };
        req.validates().unwrap();
        req
    };

    // 1. Initial capture -> New + queued to Telegram
    let r1 = store
        .transaction(|repo| {
            repo.save_ml_ingest_batch("b1", "i1", &"1".repeat(64), &make_req(29_900, day1), day1)
        })
        .unwrap();
    assert_eq!(r1.new_product_count, 1);
    assert_eq!(r1.telegram_queued_count, 1);
    assert_eq!(r1.comparisons[0].trend, PriceTrendKind::New);
    assert!(r1.comparisons[0].telegram_enqueued);

    let q1 = store
        .read(|repo| {
            repo.ml_telegram_queue(
                Some("@procurali"),
                Some(TelegramDeliveryStatus::Queued),
                None,
                10,
            )
        })
        .unwrap();
    assert_eq!(q1.len(), 1);
    store
        .transaction(|repo| {
            assert!(repo.set_ml_telegram_delivery_status(
                &q1[0].delivery_id,
                TelegramDeliveryStatus::Queued,
                TelegramDeliveryStatus::Sending,
                None,
                None,
                day1 + 59_000,
            )?);
            repo.set_ml_telegram_delivery_status(
                &q1[0].delivery_id,
                TelegramDeliveryStatus::Sending,
                TelegramDeliveryStatus::Published,
                Some("101"),
                None,
                day1 + 60_000,
            )
        })
        .unwrap();

    // 2. Same-day identical price -> Stable, NOT re-queued, collapsed in 30d series
    let same_day = day1 + 3_600_000;
    let r2 = store
        .transaction(|repo| {
            repo.save_ml_ingest_batch(
                "b2",
                "i2",
                &"2".repeat(64),
                &make_req(29_900, same_day),
                same_day,
            )
        })
        .unwrap();
    assert_eq!(r2.unchanged_price_count, 1);
    assert_eq!(r2.telegram_queued_count, 0);
    assert_eq!(r2.comparisons[0].trend, PriceTrendKind::Stable);
    assert!(!r2.comparisons[0].telegram_enqueued);
    assert_eq!(r2.comparisons[0].observation_count, 1);

    let cat2 = store.read(|repo| repo.ml_site_catalog(same_day)).unwrap();
    assert_eq!(cat2.offers[0].price_observations.len(), 1);

    // 3. Next-day price drop -> Down, enqueues Telegram price-drop alert, updates 30d min/avg
    let day2 = day1 + 86_400_000;
    let r3 = store
        .transaction(|repo| {
            repo.save_ml_ingest_batch("b3", "i3", &"3".repeat(64), &make_req(24_900, day2), day2)
        })
        .unwrap();
    assert_eq!(r3.price_drop_count, 1);
    assert_eq!(r3.telegram_queued_count, 1);
    assert_eq!(r3.comparisons[0].trend, PriceTrendKind::Down);
    assert_eq!(r3.comparisons[0].previous_price_cents, Some(29_900));
    assert_eq!(r3.comparisons[0].price_delta_cents, -5_000);
    assert_eq!(r3.comparisons[0].lowest_price_30d_cents, 24_900);
    assert_eq!(r3.comparisons[0].average_price_30d_cents, 27_400);
    assert_eq!(r3.comparisons[0].observation_count, 2);

    // Repeating same dropped price while still queued must update in-place without stacking duplicates
    let day2b = day2 + 600_000;
    let r3b = store
        .transaction(|repo| {
            repo.save_ml_ingest_batch(
                "b3b",
                "i3b",
                &"4".repeat(64),
                &make_req(24_900, day2b),
                day2b,
            )
        })
        .unwrap();
    assert_eq!(r3b.telegram_queued_count, 0);
    let q_drop = store
        .read(|repo| {
            repo.ml_telegram_queue(
                Some("@procurali"),
                Some(TelegramDeliveryStatus::Queued),
                None,
                10,
            )
        })
        .unwrap();
    assert_eq!(q_drop.len(), 1);
    assert!(q_drop[0].message_text.contains("📉 BAIXOU DE PREÇO"));

    store
        .transaction(|repo| {
            assert!(repo.set_ml_telegram_delivery_status(
                &q_drop[0].delivery_id,
                TelegramDeliveryStatus::Queued,
                TelegramDeliveryStatus::Sending,
                None,
                None,
                day2b + 59_000,
            )?);
            repo.set_ml_telegram_delivery_status(
                &q_drop[0].delivery_id,
                TelegramDeliveryStatus::Sending,
                TelegramDeliveryStatus::Published,
                Some("102"),
                None,
                day2b + 60_000,
            )
        })
        .unwrap();

    // 4. Later price increase -> Up, recorded in history, NOT re-queued
    let day3 = day1 + 2 * 86_400_000;
    let r4 = store
        .transaction(|repo| {
            repo.save_ml_ingest_batch("b4", "i4", &"5".repeat(64), &make_req(27_900, day3), day3)
        })
        .unwrap();
    assert_eq!(r4.price_increase_count, 1);
    assert_eq!(r4.telegram_queued_count, 0);
    assert_eq!(r4.comparisons[0].trend, PriceTrendKind::Up);
    assert_eq!(r4.comparisons[0].price_delta_cents, 3_000);
    assert_eq!(r4.comparisons[0].observation_count, 3);

    let cat4 = store.read(|repo| repo.ml_site_catalog(day3)).unwrap();
    assert_eq!(cat4.offers[0].price_observations.len(), 3);
    let stats = store.read(|repo| repo.ml_telegram_product_stats()).unwrap();
    assert_eq!(
        stats.get("MLB63816534"),
        Some(&(2u32, Some("published".to_string())))
    );
}
