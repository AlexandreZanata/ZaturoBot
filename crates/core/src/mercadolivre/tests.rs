use super::telegram::rendered_html_char_count;
use super::*;

#[test]
fn validates_ml_ids_urls_and_rejects_local_leaks() {
    assert!(validate_ml_product_id("MLB39410080"));
    assert!(!validate_ml_product_id("MLA123456"));
    assert!(!validate_ml_product_id("MLB12"));

    assert!(validate_meli_share_link("https://meli.la/1iWQZbJ"));
    assert!(!validate_meli_share_link("https://evil.test/1iWQZbJ"));
    assert!(!validate_meli_share_link("https://meli.la/abc?extra=1"));

    assert!(validate_ml_canonical_url(
        "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb-m2-2280-nvme-pcie-40/p/MLB39410080"
    ));
    assert!(!validate_ml_canonical_url(
        "https://www.mercadolivre.com.br/item?matt_tool=123"
    ));
    assert!(no_local_path_leak(
        "https://media.procurali.com.br/procurali-media/MLB1/01.jpg"
    ));
    assert!(!no_local_path_leak(
        "/home/iiii/.local/commercial/mercadolivre/images/1.jpg"
    ));
}

#[test]
fn parses_multiline_mercado_livre_card_text() {
    let raw = "MAIS VENDIDO\nSSD Kingston NV3 500GB\n4.9 | +10mil vendidos\nR$\n1.379\nR$\n699\n,\n90\n49% OFF\n18x \nR$\n46\n,\n26\nCompartilhar";
    let parsed = parse_card_text(raw);
    assert_eq!(parsed.card_badge.as_deref(), Some("Mais vendido"));
    assert_eq!(parsed.marketplace_rating.as_deref(), Some("4,9 / 5"));
    assert_eq!(
        parsed.marketplace_sales_label.as_deref(),
        Some("+10 mil vendidos no ML")
    );
    assert_eq!(parsed.reference_price_cents, Some(137_900));
    assert_eq!(parsed.card_price_cents, Some(69_990));
    assert_eq!(parsed.discount_label.as_deref(), Some("49% OFF"));
    assert_eq!(
        parsed.installment_summary.as_deref(),
        Some("18× de R$ 46,26")
    );
    assert_eq!(parse_commission_bp("5%"), Some(500));
}

#[test]
fn parses_jsonld_variant_and_formats_sigv4_timestamps() {
    let variant_json = serde_json::json!({
        "@type": "Product",
        "name": "LG UltraGear 27\"",
        "sku": "MLB5359373501",
        "productID": "MLB73964684",
        "color": "Preto",
        "additionalProperty": [{"name": "Voltagem", "value": "220V"}],
        "offers": {
            "price": 3970,
            "priceCurrency": "BRL",
            "availability": "https://schema.org/InStock",
            "url": "https://www.mercadolivre.com.br/lg-ultragear/p/MLB73964684",
            "hasMerchantReturnPolicy": {
                "merchantReturnDays": 30,
                "returnFees": "https://schema.org/FreeReturn"
            }
        }
    });
    let variant = parse_jsonld_variant(&variant_json).unwrap();
    assert_eq!(variant.variant_product_id, "MLB73964684");
    assert_eq!(variant.voltage.as_deref(), Some("220V"));
    assert_eq!(variant.price_brl_cents, 397_000);
    assert_eq!(variant.return_days, Some(30));
    assert_eq!(variant.return_fees.as_deref(), Some("Grátis"));

    let (amz, stamp) = format_utc_sigv4(1_763_231_280).unwrap();
    assert!(amz.ends_with('Z'));
    assert_eq!(amz.len(), 16);
    assert_eq!(stamp.len(), 8);
}

#[test]
fn formats_telegram_html_caption_commands_and_whatsapp_assisted_draft() {
    let item = TelegramOfferQueueItem {
        delivery_id: "tg-del-1".into(),
        product_id: "MLB39410080".into(),
        offer_id: "offer-ml-MLB39410080".into(),
        channel_id: "@procurali".into(),
        title: "SSD Kingston <NV3> 1TB & PCIe 4.0 \"Gen4\" ".repeat(8),
        brand: "Kingston".into(),
        model: Some("SNV3S/1000G".into()),
        price_brl_cents: 44_990,
        reference_price_cents: Some(59_900),
        discount_label: Some("25% OFF".into()),
        share_link: "https://meli.la/1iWQZbJ".into(),
        share_id: "1PBCGF-SMN5".into(),
        image_public_url: Some(
            "https://media.procurali.com.br/procurali-media/ml/MLB39410080/hero.jpg".into(),
        ),
        image_object_key: Some("ml/MLB39410080/hero.jpg".into()),
        message_text: "💳 10x sem juros\n🏪 Loja oficial\n⚠️ Confira slot M.2".into(),
        status: TelegramDeliveryStatus::Queued,
        external_message_id: None,
        created_at_ms: 1_763_231_280_000,
        updated_at_ms: 1_763_231_280_000,
        published_at_ms: None,
    };

    let caption = format_telegram_html_caption_pt_br(&item, Some("https://procurali.com.br"));
    assert!(caption.contains("&lt;NV3&gt;"));
    assert!(caption.contains("&amp;"));
    assert!(caption.contains("OFERTA RELÂMPAGO"));
    assert!(caption.contains("❌ De: <s>R$ 599,00</s>"));
    assert!(caption.contains("✅ <b>POR APENAS: R$ 449,90</b>"));
    assert!(caption.contains("Economia de R$ 149,10"));
    assert!(caption.contains("https://meli.la/1iWQZbJ"));
    assert!(rendered_html_char_count(&caption) <= TELEGRAM_CAPTION_MAX_CHARS);

    let kb = build_telegram_offer_keyboard(&item, Some("https://procurali.com.br"));
    assert_eq!(kb.inline_keyboard.len(), 2);
    assert_eq!(
        kb.inline_keyboard[0][0].text,
        "🔥 PEGAR OFERTA NO MERCADO LIVRE 🛒"
    );
    assert_eq!(kb.inline_keyboard[0][0].url, "https://meli.la/1iWQZbJ");
    assert!(
        kb.inline_keyboard[1][0]
            .url
            .starts_with("https://procurali.com.br/ofertas/")
    );

    assert_eq!(
        TelegramBotCommand::parse("/start@ProcuraliBot"),
        Some(TelegramBotCommand::Start)
    );
    assert_eq!(
        TelegramBotCommand::parse("/buscar@ProcuraliBot ssd kingston"),
        Some(TelegramBotCommand::Search("ssd kingston".into()))
    );
    assert_eq!(TelegramBotCommand::parse("/buscar   "), None);
    assert_eq!(
        TelegramBotCommand::parse("/whatsapp"),
        Some(TelegramBotCommand::WhatsappDigest)
    );

    let (hidden, _) = format_telegram_bot_command_reply_pt_br(
        &TelegramBotCommand::Search("kingston".into()),
        std::slice::from_ref(&item),
        Some("https://procurali.com.br"),
    );
    assert!(
        !hidden.contains("https://meli.la/1iWQZbJ"),
        "unpublished deliveries must not reach public bot replies"
    );

    let published = TelegramOfferQueueItem {
        status: TelegramDeliveryStatus::Published,
        ..item.clone()
    };
    let (reply, markup) = format_telegram_bot_command_reply_pt_br(
        &TelegramBotCommand::Search("kingston".into()),
        std::slice::from_ref(&published),
        Some("https://procurali.com.br"),
    );
    assert!(reply.contains("https://meli.la/1iWQZbJ"));
    assert!(markup.is_some());

    let pkg = build_whatsapp_assisted_package(
        &[item],
        "@procurali",
        Some("https://procurali.com.br"),
        1_763_231_280_000,
    );
    assert_eq!(pkg.mode, "whatsapp_web_human_assisted_no_meta_api");
    assert_eq!(pkg.draft_count, 1);
    assert!(pkg.drafts[0].whatsapp_text.contains("~De R$ 599,00~"));
    assert!(pkg.drafts[0].whatsapp_text.contains("```1PBCGF-SMN5```"));
    assert!(
        pkg.drafts[0]
            .whatsapp_text
            .contains("https://meli.la/1iWQZbJ")
    );
    assert!(pkg.drafts[0].whatsapp_text.contains("*R$ 449,90*"));
    assert!(pkg.drafts[0].whatsapp_text.contains("*Kingston*"));
    assert!(!pkg.drafts[0].whatsapp_text.contains("🔥 * "));
}

#[test]
fn computes_cloobe_taxonomy_price_intelligence_sparkline_and_alerts() {
    let (sub_slug, sub_name) = classify_product_subcategory(
        "Monitor Gamer LG UltraGear 27 IPS 200Hz",
        "hardware-e-perifericos",
        &[],
    );
    assert_eq!(sub_slug, "monitores");
    assert_eq!(sub_name, "Monitores");
    assert_eq!(parse_rating_milli(Some("4,9 / 5")), Some(4900));
    assert_eq!(
        parse_sales_volume_estimate(Some("+10 mil vendidos no ML")),
        10_000
    );
    assert_eq!(compute_discount_pct(69_990, Some(137_900), None), 49);

    let obs = vec![
        SiteCatalogPriceObservation {
            id: "o3".into(),
            offer_id: "of1".into(),
            variant_id: "v1".into(),
            seller_name: "Oficial".into(),
            payment_basis: "Pix".into(),
            freight_note: "Grátis".into(),
            price_brl_cents: 69_990,
            observed_at: "2026-11-16T12:00:00Z".into(),
            evidence_id: "ev1".into(),
        },
        SiteCatalogPriceObservation {
            id: "o2".into(),
            offer_id: "of1".into(),
            variant_id: "v1".into(),
            seller_name: "Oficial".into(),
            payment_basis: "Pix".into(),
            freight_note: "Grátis".into(),
            price_brl_cents: 89_900,
            observed_at: "2026-11-10T12:00:00Z".into(),
            evidence_id: "ev1".into(),
        },
        SiteCatalogPriceObservation {
            id: "o1".into(),
            offer_id: "of1".into(),
            variant_id: "v1".into(),
            seller_name: "Oficial".into(),
            payment_basis: "Pix".into(),
            freight_note: "Grátis".into(),
            price_brl_cents: 94_900,
            observed_at: "2026-11-01T12:00:00Z".into(),
            evidence_id: "ev1".into(),
        },
    ];

    let intel = compute_price_intelligence(
        69_990,
        Some(137_900),
        Some("49% OFF"),
        1_763_252_000_000,
        &obs,
    );
    assert_eq!(intel.window_days, 30);
    assert_eq!(intel.lowest_price_30d_cents, 69_990);
    assert_eq!(intel.highest_price_30d_cents, 137_900);
    assert_eq!(intel.previous_price_cents, Some(89_900));
    assert_eq!(intel.price_delta_cents, -19_910);
    assert_eq!(intel.price_delta_pct, -22);
    assert_eq!(intel.observation_count, 3);
    assert_eq!(intel.price_trend, PriceTrendKind::Down);
    assert_eq!(intel.first_observed_at_display, "2026-11-01T12:00:00Z");
    assert_eq!(intel.price_level, PriceLevelKind::LowestPrice);
    assert_eq!(intel.price_level_label, "Menor preço");
    assert!(intel.is_price_drop);
    assert!(
        intel
            .verified_at_display
            .starts_with("Preços verificados em ")
    );
    assert!(intel.svg_sparkline.contains("<svg viewBox=\"0 0 300 80\""));
    assert!(intel.svg_sparkline.contains("<polyline"));

    let summary = format_price_comparison_summary(
        intel.current_price_cents,
        intel.previous_price_cents,
        intel.lowest_price_30d_cents,
        intel.average_price_30d_cents,
        intel.observation_count,
        intel.price_trend,
        intel.price_delta_pct,
    );
    assert!(summary.contains("Queda para"));
    assert!(summary.contains("-22%"));

    // Verify WhatsApp package deduplicates duplicate deliveries by product_id.
    let dup_a = TelegramOfferQueueItem {
        delivery_id: "tg-new".into(),
        product_id: "MLB39410080".into(),
        offer_id: "of2".into(),
        channel_id: "@procurali".into(),
        title: "SSD Kingston NV3".into(),
        brand: "Kingston".into(),
        model: None,
        price_brl_cents: 69_990,
        reference_price_cents: None,
        discount_label: None,
        share_link: "https://meli.la/1iWQZbJ".into(),
        share_id: "1PBCGF-SMN5".into(),
        image_public_url: None,
        image_object_key: None,
        message_text: String::new(),
        status: TelegramDeliveryStatus::Queued,
        external_message_id: None,
        created_at_ms: 2_000,
        updated_at_ms: 2_000,
        published_at_ms: None,
    };
    let mut dup_b = dup_a.clone();
    dup_b.delivery_id = "tg-old".into();
    let pkg = build_whatsapp_assisted_package(&[dup_a, dup_b], "@procurali", None, 2_000);
    assert_eq!(pkg.draft_count, 1);
    assert_eq!(pkg.drafts[0].delivery_id, "tg-new");

    let ok_alert = PriceAlertActionRequest {
        action: "criar-alerta".into(),
        subscriber_id: "sub_01".into(),
        product_id: Some("MLB39410080".into()),
        target_price_cents: Some(65_000),
    };
    ok_alert.validates().unwrap();

    let bad_beacon = ExposureBeaconRequest {
        route: "/categorias/hardware/".into(),
        events: vec![],
        timestamp_ms: 1_763_252_000_000,
    };
    assert!(bad_beacon.validates().is_err());
}

#[test]
fn telegram_delivery_acks_follow_the_publication_state_machine() {
    use TelegramDeliveryStatus::*;
    let all = [
        AwaitingApproval,
        Queued,
        Sending,
        Published,
        Skipped,
        Failed,
    ];
    for status in all {
        assert_eq!(TelegramDeliveryStatus::parse(status.as_str()), Some(status));
    }
    let allowed = [
        (AwaitingApproval, Skipped),
        (Queued, Sending),
        (Queued, Skipped),
        (Sending, Published),
        (Sending, Failed),
    ];
    for from in all {
        for to in all {
            assert_eq!(
                TelegramDeliveryStatus::ack_transition_allowed(from, to),
                allowed.contains(&(from, to)),
                "{from:?} -> {to:?}"
            );
        }
    }
    // Acks can never approve, re-queue, or resend an uncertain claim.
    assert!(!TelegramDeliveryStatus::ack_transition_allowed(
        AwaitingApproval,
        Queued
    ));
    assert!(!TelegramDeliveryStatus::ack_transition_allowed(
        Sending, Queued
    ));
    assert!(!TelegramDeliveryStatus::ack_transition_allowed(
        Published, Queued
    ));
    assert!(!TelegramDeliveryStatus::ack_transition_allowed(
        Sending, Sending
    ));
}
