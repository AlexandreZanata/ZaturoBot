//! Independent black-box contract tests for `zaturo_core` Mercado Livre domain rules.
//!
//! These tests exercise only the public API and serialized wire shapes so internal module
//! splits cannot alter external behavior or silently weaken invariants.

use serde_json::json;
use zaturo_core::{
    AFFILIATE_DISCLOSURE_PT_BR, CatalogSearchQuery, DEFAULT_ML_FRESHNESS_TTL_MINUTES,
    ExposureBeaconRequest, ExposureEventItem, MAX_ML_BATCH_PRODUCTS, MAX_ML_IMAGES_PER_PRODUCT,
    MAX_ML_SPECS_PER_PRODUCT, MAX_ML_VARIANTS_PER_PRODUCT, MercadoLivreIngestBatchRequest,
    MercadoLivreProductState, MercadoLivreSpec, MercadoLivreVariant, MinioProductImage,
    PriceAlertActionRequest, PriceLevelKind, PriceTrendKind, SiteCatalogPriceObservation,
    TELEGRAM_CAPTION_MAX_CHARS, TELEGRAM_MESSAGE_MAX_CHARS, TelegramBotCommand,
    TelegramDeliveryAck, TelegramDeliveryStatus, TelegramOfferQueueItem,
    ValidatedMercadoLivreProduct, build_commerce_reference_item, build_site_offer_url,
    build_telegram_offer_keyboard, build_whatsapp_assisted_package, classify_product_subcategory,
    compute_discount_pct, compute_price_intelligence, enrich_site_catalog_offer,
    escape_telegram_html, extract_product_id_from_url, format_brl_cents,
    format_price_comparison_summary, format_telegram_bot_command_reply_pt_br,
    format_telegram_caption_with_price_history_pt_br, format_telegram_html_caption_pt_br,
    no_local_path_leak, parse_brl_amount_cents, parse_card_text, parse_commission_bp,
    parse_jsonld_variant, parse_price_value_cents, parse_rating_milli, parse_sales_volume_estimate,
    render_offer_cards_html_fragment, slugify_pt_br, valid_minio_bucket, valid_minio_object_key,
    valid_slug, valid_telegram_channel, validate_asset_public_url, validate_meli_share_link,
    validate_ml_canonical_url, validate_ml_product_id, validate_ml_share_id,
};

fn sample_image(product_id: &str) -> MinioProductImage {
    MinioProductImage {
        position: 0,
        bucket: "procurali-media".into(),
        object_key: format!("mercadolivre/{product_id}/00-aaaaaaaaaaaaaaaa.jpg"),
        public_url: format!(
            "https://media.procurali.com.br/procurali-media/mercadolivre/{product_id}/00-aaaaaaaaaaaaaaaa.jpg"
        ),
        source_origin_url: Some("https://http2.mlstatic.com/D_NQ_NP_sample.jpg".into()),
        content_type: "image/jpeg".into(),
        sha256_hex: "a".repeat(64),
        size_bytes: 24_576,
        alt_text: "Foto do produto em oferta".into(),
    }
}

fn sample_validated_product(product_id: &str) -> ValidatedMercadoLivreProduct {
    ValidatedMercadoLivreProduct {
        product_id: product_id.into(),
        slug: slugify_pt_br("SSD Kingston NV3 500GB NVMe", product_id),
        title: "SSD Kingston NV3 500GB NVMe".into(),
        brand: "Kingston".into(),
        model: Some("SNV3S/500G".into()),
        exact_variant: "500 GB M.2 2280 PCIe 4.0".into(),
        category_slug: "hardware-e-perifericos".into(),
        canonical_url: format!("https://www.mercadolivre.com.br/ssd-nv3/p/{product_id}"),
        seller_name: "Loja Oficial Kingston".into(),
        share_link: "https://meli.la/2ZpQqwX".into(),
        share_id: "1PBCGF-SMN5".into(),
        share_text: "Código 1PBCGF-SMN5\nhttps://meli.la/2ZpQqwX".into(),
        commission_label: Some("5%".into()),
        commission_bp: Some(500),
        sponsored: false,
        price_brl_cents: 29_990,
        pix_price_cents: 28_490,
        reference_price_cents: Some(44_990),
        currency: "BRL".into(),
        marketplace_rating: Some("4,9 / 5".into()),
        marketplace_sales_label: Some("+10 mil vendidos".into()),
        card_badge: Some("Mais vendido".into()),
        discount_label: Some("33% OFF".into()),
        installment_summary: Some("10× de R$ 29,99 sem juros".into()),
        payment_conditions: "R$ 284,90 no Pix ou 10x sem juros".into(),
        short_condition: "10x sem juros · devolução 30d".into(),
        freight_status: "Frete Full".into(),
        availability: "InStock".into(),
        price_valid_until: Some("2026-11-30".into()),
        return_days: Some(30),
        short_description: "SSD NVMe M.2 PCIe 4.0 x4 com leitura até 5.000 MB/s.".into(),
        long_description: "Unidade M.2 2280 para upgrade de notebooks e desktops.".into(),
        one_line_reason: "Ótimo custo-benefício em PCIe 4.0 nesta faixa.".into(),
        for_whom: "Quem quer acelerar sistema e jogos em slot M.2.".into(),
        supported_benefits: vec!["PCIe 4.0 até 5.000 MB/s".into()],
        limitations: vec!["Não acompanha dissipador de alumínio.".into()],
        unknowns: vec!["Série contínua de 7 dias em coleta.".into()],
        tags: vec!["ssd".into(), "nvme".into()],
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
            sku: "MLB500G".into(),
            name: "500 GB".into(),
            color: Some("Preto".into()),
            voltage: None,
            price_brl_cents: 29_990,
            currency: "BRL".into(),
            availability: "InStock".into(),
            return_days: Some(30),
            return_fees: Some("Grátis".into()),
            canonical_url: Some(format!(
                "https://www.mercadolivre.com.br/ssd-nv3/p/{product_id}"
            )),
            highlights: vec!["NVMe 2280".into()],
        }],
        images: vec![sample_image(product_id)],
        observed_at_ms: 1_763_231_280_000,
        freshness_ttl_minutes: DEFAULT_ML_FRESHNESS_TTL_MINUTES,
        evidence_sha256: "b".repeat(64),
        state: MercadoLivreProductState::Approved,
    }
}

fn sample_queue_item(
    delivery_id: &str,
    product_id: &str,
    status: TelegramDeliveryStatus,
) -> TelegramOfferQueueItem {
    TelegramOfferQueueItem {
        delivery_id: delivery_id.into(),
        product_id: product_id.into(),
        offer_id: format!("offer-{product_id}"),
        channel_id: "@procurali".into(),
        title: "SSD Kingston NV3 500GB NVMe PCIe 4.0".into(),
        brand: "Kingston".into(),
        model: Some("SNV3S/500G".into()),
        price_brl_cents: 28_490,
        reference_price_cents: Some(44_990),
        discount_label: Some("37% OFF".into()),
        share_link: "https://meli.la/2ZpQqwX".into(),
        share_id: "1PBCGF-SMN5".into(),
        image_public_url: Some(sample_image(product_id).public_url),
        image_object_key: Some(sample_image(product_id).object_key),
        message_text: "📉 BAIXOU DE PREÇO! Antes: R$ 350,00 → Agora: R$ 284,90\n💳 No Pix".into(),
        status,
        external_message_id: None,
        created_at_ms: 1_763_231_280_000,
        updated_at_ms: 1_763_231_280_000,
        published_at_ms: (status == TelegramDeliveryStatus::Published).then_some(1_763_231_290_000),
    }
}

#[test]
fn constants_and_state_transition_contracts_are_preserved() {
    assert_eq!(MAX_ML_BATCH_PRODUCTS, 50);
    assert_eq!(MAX_ML_SPECS_PER_PRODUCT, 64);
    assert_eq!(MAX_ML_VARIANTS_PER_PRODUCT, 16);
    assert_eq!(MAX_ML_IMAGES_PER_PRODUCT, 16);
    assert_eq!(DEFAULT_ML_FRESHNESS_TTL_MINUTES, 30);
    assert_eq!(TELEGRAM_CAPTION_MAX_CHARS, 1024);
    assert_eq!(TELEGRAM_MESSAGE_MAX_CHARS, 4096);
    assert!(AFFILIATE_DISCLOSURE_PT_BR.contains("afiliado"));

    for state in [
        MercadoLivreProductState::Candidate,
        MercadoLivreProductState::Approved,
        MercadoLivreProductState::Stale,
        MercadoLivreProductState::Withdrawn,
    ] {
        assert_eq!(MercadoLivreProductState::parse(state.as_str()), Some(state));
    }

    assert!(!TelegramDeliveryStatus::AwaitingApproval.is_terminal());
    assert!(!TelegramDeliveryStatus::Queued.is_terminal());
    assert!(!TelegramDeliveryStatus::Sending.is_terminal());
    assert!(TelegramDeliveryStatus::Published.is_terminal());
    assert!(TelegramDeliveryStatus::Skipped.is_terminal());
    assert!(TelegramDeliveryStatus::Failed.is_terminal());

    assert!(TelegramDeliveryStatus::ack_transition_allowed(
        TelegramDeliveryStatus::Queued,
        TelegramDeliveryStatus::Sending
    ));
    assert!(TelegramDeliveryStatus::ack_transition_allowed(
        TelegramDeliveryStatus::Sending,
        TelegramDeliveryStatus::Published
    ));
    assert!(TelegramDeliveryStatus::ack_transition_allowed(
        TelegramDeliveryStatus::Sending,
        TelegramDeliveryStatus::Failed
    ));
    assert!(!TelegramDeliveryStatus::ack_transition_allowed(
        TelegramDeliveryStatus::AwaitingApproval,
        TelegramDeliveryStatus::Queued
    ));
    assert!(!TelegramDeliveryStatus::ack_transition_allowed(
        TelegramDeliveryStatus::Queued,
        TelegramDeliveryStatus::Published
    ));
}

#[test]
fn validators_reject_local_leaks_and_enforce_official_links() {
    assert!(validate_ml_product_id("MLB72674962"));
    assert!(!validate_ml_product_id(""));
    assert!(!validate_ml_product_id("invalid id!"));

    assert!(validate_meli_share_link("https://meli.la/2ZpQqwX"));
    assert!(!validate_meli_share_link("https://evil.example/meli.la"));

    assert!(validate_ml_canonical_url(
        "https://www.mercadolivre.com.br/ssd/p/MLB123456"
    ));
    assert!(!validate_ml_canonical_url("https://localhost/p/MLB123456"));

    assert!(validate_ml_share_id("1PBCGF-SMN5"));
    assert!(valid_slug("hardware-e-perifericos"));
    assert!(!valid_slug("Hardware E Perifericos"));

    assert!(valid_minio_bucket("procurali-media"));
    assert!(valid_minio_object_key("mercadolivre/MLB123/00-abc.jpg"));
    assert!(validate_asset_public_url(
        "https://procurali.com.br/procurali-media/mercadolivre/MLB123/00.jpg"
    ));
    assert!(!validate_asset_public_url(
        "http://procurali.com.br/procurali-media/a.jpg"
    ));

    assert!(valid_telegram_channel("@procurali"));
    assert!(valid_telegram_channel("-1001234567890"));
    assert!(!valid_telegram_channel("procurali"));

    assert!(no_local_path_leak("SSD Kingston NV3 500GB"));
    assert!(!no_local_path_leak(".local/commercial/a.png"));
    assert!(!no_local_path_leak("/home/user/photo.jpg"));
}

#[test]
fn parsers_price_trends_and_classification_contracts_hold() {
    assert_eq!(format_brl_cents(29_986), "R$ 299,86");
    assert_eq!(parse_brl_amount_cents("R$ 1.299,90"), Some(129_990));
    assert_eq!(parse_price_value_cents(&json!("299.86")), Some(29_986));
    assert_eq!(parse_commission_bp("5,5%"), Some(550));
    assert_eq!(
        extract_product_id_from_url("https://www.mercadolivre.com.br/item/p/MLB63816534"),
        Some("MLB63816534".into())
    );

    let parsed = parse_card_text(
        "MAIS VENDIDO\n4.9 | +10mil vendidos\nR$ 449,90\nR$\n299\n,\n90\n33% OFF\n10x R$ 29,99 sem juros",
    );
    assert_eq!(parsed.card_badge.as_deref(), Some("Mais vendido"));
    assert_eq!(parsed.marketplace_rating.as_deref(), Some("4,9 / 5"));
    assert_eq!(parsed.discount_label.as_deref(), Some("33% OFF"));
    assert_eq!(parsed.reference_price_cents, Some(44_990));
    assert_eq!(parsed.card_price_cents, Some(29_990));

    let variant = parse_jsonld_variant(&json!({
        "productID": "MLB63816534",
        "sku": "M900PRO",
        "name": "Mouse Delux M900Pro",
        "color": "Preto",
        "offers": {
            "price": 299.86,
            "priceCurrency": "BRL",
            "availability": "https://schema.org/InStock"
        }
    }))
    .expect("jsonld variant should parse");
    assert_eq!(variant.price_brl_cents, 29_986);
    assert_eq!(variant.availability, "InStock");

    assert_eq!(
        classify_product_subcategory("SSD Kingston NV3 NVMe M.2", "hardware-e-perifericos", &[]).0,
        "ssd-e-armazenamento"
    );
    assert_eq!(parse_rating_milli(Some("4,9 / 5")), Some(4900));
    assert_eq!(
        parse_sales_volume_estimate(Some("+10 mil vendidos")),
        10_000
    );
    assert_eq!(
        compute_discount_pct(29_990, Some(44_990), Some("33% OFF")),
        33
    );

    let (trend, delta, pct) = PriceTrendKind::from_prices(25_000, Some(30_000));
    assert_eq!(trend, PriceTrendKind::Down);
    assert_eq!(delta, -5_000);
    assert_eq!(pct, -17);
}

#[test]
fn intelligence_telegram_and_whatsapp_contracts_are_preserved() {
    let product = sample_validated_product("MLB72674962");
    product.validates().expect("sample product must validate");

    let mut bad = product.clone();
    bad.share_text = "sem link oficial".into();
    assert!(bad.validates().is_err());

    let batch = MercadoLivreIngestBatchRequest {
        worker_id: "worker-ml".into(),
        search_query: "ssd nvme".into(),
        channel_id: "@procurali".into(),
        captured_at_ms: 1_763_231_280_000,
        products: vec![product.clone()],
    };
    batch.validates().expect("batch must validate");

    let obs = vec![
        SiteCatalogPriceObservation {
            id: "obs-2".into(),
            offer_id: "offer-ml-MLB72674962".into(),
            variant_id: "MLB72674962".into(),
            seller_name: "Loja Oficial Kingston".into(),
            payment_basis: "Pix".into(),
            freight_note: "Frete Full".into(),
            price_brl_cents: 28_490,
            observed_at: "2026-11-16T12:00:00Z".into(),
            evidence_id: "ev-1".into(),
        },
        SiteCatalogPriceObservation {
            id: "obs-1".into(),
            offer_id: "offer-ml-MLB72674962".into(),
            variant_id: "MLB72674962".into(),
            seller_name: "Loja Oficial Kingston".into(),
            payment_basis: "Pix".into(),
            freight_note: "Frete Full".into(),
            price_brl_cents: 35_000,
            observed_at: "2026-11-15T12:00:00Z".into(),
            evidence_id: "ev-1".into(),
        },
    ];
    let intel = compute_price_intelligence(
        28_490,
        Some(44_990),
        Some("37% OFF"),
        1_763_231_280_000,
        &obs,
    );
    assert_eq!(intel.window_days, 30);
    assert_eq!(intel.price_level, PriceLevelKind::LowestPrice);
    assert!(intel.is_price_drop);
    assert!(intel.svg_sparkline.starts_with("<svg"));

    let mut site_offer = product.to_site_catalog_offer("ev-1", "mr-1");
    site_offer.price_observations = obs;
    enrich_site_catalog_offer(&mut site_offer, 1_763_231_280_000);
    assert!(site_offer.price_intelligence.is_some());
    assert!(!site_offer.dynamic_faq.is_empty());
    let html_cards = render_offer_cards_html_fragment(std::slice::from_ref(&site_offer), 0);
    assert!(html_cards.contains("data-id=\"MLB72674962\""));
    assert!(html_cards.contains(&format!("/ofertas/{}/index.html", site_offer.slug)));

    let summary = format_price_comparison_summary(
        28_490,
        Some(35_000),
        28_490,
        31_745,
        2,
        PriceTrendKind::Down,
        -19,
    );
    assert!(summary.contains("Queda"));

    let ref_item = build_commerce_reference_item(&site_offer, 1, Some("published".into()));
    assert_eq!(ref_item.product_id, "MLB72674962");
    assert_eq!(ref_item.telegram_published_count, 1);

    let queue_pub = sample_queue_item("del-pub", "MLB72674962", TelegramDeliveryStatus::Published);
    let queue_hidden = sample_queue_item(
        "del-wait",
        "MLB99999999",
        TelegramDeliveryStatus::AwaitingApproval,
    );

    let caption = format_telegram_html_caption_pt_br(&queue_pub, Some("https://procurali.com.br"));
    assert!(caption.len() <= 4096);
    assert!(caption.contains("https://meli.la/2ZpQqwX"));
    assert_eq!(escape_telegram_html("<a&b>"), "&lt;a&amp;b&gt;");

    let kb = build_telegram_offer_keyboard(&queue_pub, Some("https://procurali.com.br"));
    assert_eq!(kb.inline_keyboard.len(), 2);
    assert!(
        build_site_offer_url(
            Some("https://procurali.com.br"),
            &product.title,
            &product.product_id
        )
        .is_some()
    );

    let drop_caption = format_telegram_caption_with_price_history_pt_br(&product, None);
    assert!(drop_caption.contains("https://meli.la/2ZpQqwX"));

    let (reply, _) = format_telegram_bot_command_reply_pt_br(
        &TelegramBotCommand::Offers,
        &[queue_pub.clone(), queue_hidden],
        Some("https://procurali.com.br"),
    );
    assert!(reply.contains("MLB72674962") || reply.contains("SSD Kingston"));
    assert!(!reply.contains("MLB99999999"));

    assert_eq!(
        TelegramBotCommand::parse("/buscar ssd nvme"),
        Some(TelegramBotCommand::Search("ssd nvme".into()))
    );

    let pkg = build_whatsapp_assisted_package(
        &[queue_pub],
        "@procurali",
        Some("https://procurali.com.br"),
        1_763_231_280_000,
    );
    assert_eq!(pkg.mode, "whatsapp_web_human_assisted_no_meta_api");
    assert_eq!(pkg.draft_count, 1);

    // Search, alert, exposure, ack validators.
    CatalogSearchQuery {
        q: Some("kingston".into()),
        pagina: Some(1),
        por_pagina: Some(24),
        ..Default::default()
    }
    .validate()
    .expect("search query validates");

    ExposureBeaconRequest {
        route: "/ofertas/".into(),
        events: vec![ExposureEventItem {
            product_id: "MLB72674962".into(),
            store: "Mercado Livre".into(),
            pos: 1,
            discount_pct: 33,
            price_cents: 28_490,
        }],
        timestamp_ms: 1_763_231_280_000,
    }
    .validates()
    .expect("exposure validates");

    PriceAlertActionRequest {
        action: "criar-alerta".into(),
        subscriber_id: "sub-01".into(),
        product_id: Some("MLB72674962".into()),
        target_price_cents: Some(25_000),
    }
    .validates()
    .expect("price alert validates");

    TelegramDeliveryAck {
        status: TelegramDeliveryStatus::Published,
        external_message_id: Some("12345".into()),
        error_code: None,
    }
    .validates()
    .expect("ack validates");
}
