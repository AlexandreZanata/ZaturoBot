use zaturo_core::commerce::{
    money::Currency,
    offer::{AffiliateProgram, Offer, OfferInput, RejectReason},
};

const NOW: i64 = 1_800_000_000_000;
fn input() -> OfferInput {
    OfferInput {
        program: AffiliateProgram::Shopee,
        item_id: "fixture-item".into(),
        title: "Synthetic USB adapter".into(),
        category: "accessories".into(),
        price: "89,90".into(),
        currency: Currency::Brl,
        observed_at: Some(NOW - 60_000),
        valid_until: Some(NOW + 600_000),
        affiliate_url: "https://s.shopee.com.br/fixture?sub_id=official%2Dvalue".into(),
        commission_bp: Some(500),
        sub_id: Some("official-value".into()),
        evidence_sha256: "a".repeat(64),
        benefits: vec!["USB connector documented in the listing".into()],
    }
}

#[test]
fn offer_without_official_link_or_timestamp_is_rejected() {
    let mut candidate = input();
    candidate.affiliate_url.clear();
    candidate.observed_at = None;
    let reasons = candidate.validate(NOW).unwrap_err();
    assert!(reasons.contains(&RejectReason::InvalidAffiliateUrl));
    assert!(reasons.contains(&RejectReason::MissingObservation));
}

#[test]
fn offer_url_with_foreign_host_is_rejected() {
    for raw in [
        "http://s.shopee.com.br/x",
        "https://s.shopee.com.br.evil.invalid/x",
        "https://evil.invalid/?next=https://shopee.com.br",
        "https://owner@s.shopee.com.br/x",
        "https://@s.shopee.com.br/x",
        "https:s.shopee.com.br/x",
        "https://s%2eshopee.com.br/x",
        "https:////@s.shopee.com.br/x",
        "https:////s%2eshopee.com.br/x",
        "https:///s.shopee.com.br/x",
        "https://s.shopee.com.br:444/x",
        "https://s.shopee.com.br/x#fragment",
        " https://s.shopee.com.br/x",
        "https://s.shopee.com.br\\@evil.invalid/x",
        "https://s.shopee.com.br/x\n",
    ] {
        let mut candidate = input();
        candidate.affiliate_url = raw.into();
        assert!(
            candidate
                .validate(NOW)
                .unwrap_err()
                .contains(&RejectReason::InvalidAffiliateUrl),
            "{raw}"
        );
    }
    let mut candidate = input();
    candidate.program = AffiliateProgram::MercadoLivre;
    assert!(candidate.validate(NOW).is_err());
    candidate.affiliate_url = "https://www.mercadolivre.com.br/sec/fixture".into();
    assert!(candidate.validate(NOW).is_ok());
}

#[test]
fn offer_url_is_preserved_byte_for_byte() {
    let candidate = input();
    let offer = candidate.validate(NOW).unwrap();
    assert_eq!(
        offer.affiliate_url.as_bytes(),
        candidate.affiliate_url.as_bytes()
    );
    assert_eq!(offer.price.minor, 8990);
    assert!(offer.validate(NOW).is_ok());
}

#[test]
fn offer_revalidates_mutated_and_deserialized_values_and_reports_all_reasons() {
    let mut offer = input().validate(NOW).unwrap();
    offer.evidence_sha256 = "0".repeat(64);
    offer.commission_bp = Some(10_001);
    offer.benefits = vec![];
    offer.valid_until = NOW;
    offer.price.minor = -1;
    let restored: Offer = serde_json::from_str(&serde_json::to_string(&offer).unwrap()).unwrap();
    let reasons = restored.validate(NOW).unwrap_err();
    for reason in [
        RejectReason::InvalidEvidence,
        RejectReason::InvalidCommission,
        RejectReason::InvalidBenefits,
        RejectReason::InvalidValidity,
        RejectReason::InvalidPrice,
    ] {
        assert!(reasons.contains(&reason));
    }
    let mut candidate = input();
    candidate.observed_at = Some(NOW + 1);
    assert!(candidate.validate(NOW).is_err());
    candidate.observed_at = Some(-1);
    assert!(candidate.validate(NOW).is_err());
}

#[test]
fn offer_strips_title_controls_bounds_input_and_uses_conservative_default_expiry() {
    let mut candidate = input();
    candidate.title = "Synthetic\n USB\0 adapter".into();
    candidate.valid_until = None;
    let offer = candidate.validate(NOW).unwrap();
    assert_eq!(offer.title, "Synthetic USB adapter");
    assert_eq!(offer.valid_until, NOW - 60_000 + 6 * 60 * 60 * 1000);
    candidate.title = "x".repeat(201);
    assert!(candidate.validate(NOW).is_err());
    candidate = input();
    candidate.benefits = vec!["x".repeat(161)];
    assert!(candidate.validate(NOW).is_err());
    candidate = input();
    candidate.valid_until = Some(NOW + 86_400_000);
    candidate.observed_at = Some(NOW - 12 * 60 * 60 * 1000);
    assert!(
        candidate.validate(NOW).is_ok(),
        "price freshness is separate from explicit offer expiry"
    );
    candidate.observed_at = Some(i64::MAX - 1);
    candidate.valid_until = None;
    assert!(candidate.validate(i64::MAX).is_err());
}

#[test]
fn offer_rejects_unknown_fields_noncanonical_hashes_and_invalid_benefits() {
    let valid = input();
    let mut value = serde_json::to_value(&valid).unwrap();
    value["owner_approved"] = serde_json::json!(true);
    assert!(serde_json::from_value::<OfferInput>(value).is_err());
    let mut value = serde_json::to_value(valid.validate(NOW).unwrap()).unwrap();
    value["authority"] = serde_json::json!("owner");
    assert!(serde_json::from_value::<Offer>(value).is_err());
    for hash in ["A".repeat(64), "f".repeat(63), "0".repeat(64)] {
        let mut candidate = input();
        candidate.evidence_sha256 = hash;
        assert!(
            candidate
                .validate(NOW)
                .unwrap_err()
                .contains(&RejectReason::InvalidEvidence)
        );
    }
    for benefits in [
        vec![],
        vec!["   ".into()],
        vec!["Listing\ntext".into()],
        vec!["one".into(); 4],
    ] {
        let mut candidate = input();
        candidate.benefits = benefits;
        assert!(candidate.validate(NOW).is_err());
    }
    let mut candidate = input();
    candidate.title = "\n\0".into();
    assert!(
        candidate
            .validate(NOW)
            .unwrap_err()
            .contains(&RejectReason::InvalidTitle)
    );
}
