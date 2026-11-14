use zaturo_core::commerce::{
    AffiliateProgram, BrandText, ClaimContext, Currency, DEFAULT_DISCLOSURE_TEXT, Money, Offer,
    TemplateId, lint_claims, render,
};

const NOW: i64 = 1_794_658_800_000;
fn offer() -> Offer {
    Offer {
        program: AffiliateProgram::Shopee,
        item_id: "fixture-17".into(),
        title: "Hub USB 3.0 com 4 portas".into(),
        category: "accessories".into(),
        price: Money {
            minor: 8990,
            currency: Currency::Brl,
        },
        price_observed_at: NOW - 60_000,
        valid_until: NOW + 86_400_000,
        affiliate_url: "https://s.shopee.com.br/fixture17?a=1&b=2".into(),
        commission_bp: None,
        sub_id: None,
        evidence_sha256: "a".repeat(64),
        benefits: vec!["Conexão USB 3.0".into(), "4 portas".into()],
    }
}
fn brand() -> BrandText {
    BrandText {
        name: "Curadoria Fixture".into(),
        revision: "brand-r1".into(),
        disclosure_text: DEFAULT_DISCLOSURE_TEXT.into(),
    }
}

#[test]
fn template_output_passes_lint_for_all_fixture_offers() {
    for (title, benefits) in [
        (
            "Hub USB 3.0 com 4 portas",
            vec!["Conexão USB 3.0", "4 portas"],
        ),
        ("Mouse modelo A17", vec!["Sensor modelo A17"]),
        ("Cabo USB-C", vec!["Conector USB-C"]),
    ] {
        let mut offer = offer();
        offer.title = title.into();
        offer.benefits = benefits.into_iter().map(str::to_owned).collect();
        let rendered = render(TemplateId::ListingV1, &offer, &brand(), NOW, 21_600_000).unwrap();
        assert!(rendered.text.starts_with("#publi\n"));
        assert!(rendered.text.contains("R$ 89,90"));
        assert!(rendered.text.contains("America/Cuiaba"));
        assert_eq!(rendered.brand_revision, "brand-r1");
        assert_eq!(rendered.template_id, TemplateId::ListingV1);
        assert!(rendered.text.len() <= 1500);
        assert!(
            lint_claims(
                &rendered.text,
                &offer,
                ClaimContext {
                    now_millis: NOW,
                    max_price_age_millis: 21_600_000,
                    channel_program: offer.program,
                    disclosure_text: &brand().disclosure_text
                }
            )
            .is_empty()
        );
    }
}

#[test]
fn stale_price_is_omitted_not_shown_but_expired_offer_is_rejected() {
    let mut expired = offer();
    expired.price_observed_at = NOW - 21_600_001;
    let text = render(TemplateId::ListingV1, &expired, &brand(), NOW, 21_600_000)
        .unwrap()
        .text;
    assert!(!text.contains("89,90"));
    assert!(text.contains("confira o preço atual"));
    expired.valid_until = NOW;
    assert!(render(TemplateId::ListingV1, &expired, &brand(), NOW, 21_600_000).is_err());
    let mut fresh = offer();
    fresh.price_observed_at = NOW - 21_600_000;
    assert!(
        render(TemplateId::ListingV1, &fresh, &brand(), NOW, 21_600_000)
            .unwrap()
            .text
            .contains("89,90")
    );
}

#[test]
fn html_escaping_prevents_markup_injection_from_offer_titles() {
    let mut offer = offer();
    offer.title = "Hub <b>USB 3.0 & \"4\" portas</b>".into();
    offer.benefits = vec!["Conexão <USB> & 4 portas".into()];
    let rendered = render(TemplateId::ListingV1, &offer, &brand(), NOW, 21_600_000).unwrap();
    assert!(!rendered.text.contains('<'));
    assert!(!rendered.text.contains('>'));
    assert!(rendered.text.contains("&lt;b&gt;"));
    assert!(rendered.text.contains("&amp;"));
    assert!(rendered.text.contains("&quot;4&quot;"));
    assert!(rendered.text.contains("?a=1&amp;b=2"));
}

#[test]
fn renderer_fails_closed_on_claims_invalid_brand_or_expanded_size() {
    for claim in ["Eu testei este produto", "O melhor hub", "20% de desconto"] {
        let mut offer = offer();
        offer.title = claim.into();
        assert!(render(TemplateId::ListingV1, &offer, &brand(), NOW, 21_600_000).is_err());
    }
    for invalid in [
        BrandText {
            name: "x\n#publi".into(),
            ..brand()
        },
        BrandText {
            name: "o melhor".into(),
            ..brand()
        },
        BrandText {
            revision: String::new(),
            ..brand()
        },
    ] {
        assert!(render(TemplateId::ListingV1, &offer(), &invalid, NOW, 21_600_000).is_err());
    }
    let mut expanded = offer();
    expanded.title = "&".repeat(190);
    expanded.benefits = vec!["&".repeat(150); 3];
    assert!(render(TemplateId::ListingV1, &expanded, &brand(), NOW, 21_600_000).is_err());
    assert!(render(TemplateId::ListingV1, &offer(), &brand(), NOW, -1).is_err());
}

#[test]
fn stale_prices_in_imported_title_or_benefits_never_leak_through_templates() {
    for title in [
        "Hub USB por R$ 89,90",
        "Hub por 89,90",
        "Hub por 89.90",
        "Hub por 89.9",
    ] {
        let mut stale = offer();
        stale.price_observed_at = NOW - 21_600_001;
        stale.title = title.into();
        assert!(
            render(TemplateId::ListingV1, &stale, &brand(), NOW, 21_600_000).is_err(),
            "{title}"
        );
    }
    for benefit in [
        "Preço R$ 89,90",
        "Valor em BRL",
        "Custa 89 reais",
        "Por 89.90",
    ] {
        let mut stale = offer();
        stale.price_observed_at = NOW - 21_600_001;
        stale.benefits = vec![benefit.into()];
        assert!(
            render(TemplateId::ListingV1, &stale, &brand(), NOW, 21_600_000).is_err(),
            "{benefit}"
        );
    }
    let mut version = offer();
    version.price.minor = 300;
    version.price_observed_at = NOW - 21_600_001;
    assert!(
        render(TemplateId::ListingV1, &version, &brand(), NOW, 21_600_000)
            .unwrap()
            .text
            .contains("USB 3.0")
    );
}

#[test]
fn only_reviewed_affirmative_affiliate_disclosures_are_renderable() {
    for disclosure in [
        "Não somos afiliados e não recebemos comissão.",
        "Publicidade: afiliados oficiais garantem comissão para quem comprar.",
        "Somos afiliados oficiais e garantimos o menor preço.",
    ] {
        let rejected = BrandText {
            disclosure_text: disclosure.into(),
            ..brand()
        };
        assert!(!rejected.validate(), "{disclosure}");
        assert!(render(TemplateId::ListingV1, &offer(), &rejected, NOW, 21_600_000).is_err());
    }
    let approved = BrandText {
        disclosure_text: "Link de afiliado: posso receber comissão se você comprar.".into(),
        ..brand()
    };
    assert!(render(TemplateId::ListingV1, &offer(), &approved, NOW, 21_600_000).is_ok());
}
