use serde_json::json;
use zaturo_core::commerce::{
    content::BrandText,
    policy::{ActionMode, PolicyDocument, PolicyError},
};

fn valid() -> serde_json::Value {
    json!({
        "version": 1,
        "actions": {
            "import_offers": "auto", "sync_offers": "off", "rank_offers": "auto",
            "draft_template": "draft", "draft_model": "off", "schedule": "off",
            "publish": "off", "support_reply": "off", "import_conversions": "auto",
            "optimize": "off", "edit_published": "off"
        },
        "caps": {"posts_per_day": 2, "ai_calls_per_day": 0, "allowance_share_percent": 0,
            "offers_per_sync": 50, "max_price_age_hours": 6},
        "timezone": "America/Cuiaba",
        "windows": [{"start_minute": 690, "end_minute": 750},
                    {"start_minute": 1140, "end_minute": 1260}],
        "channels": [{"id": "fixture-channel", "program": "shopee"}],
        "categories": ["accessories"],
        "price_band": {"minimum": {"minor": 3000, "currency": "BRL"},
                       "maximum": {"minor": 30000, "currency": "BRL"}},
        "disclosure_text": "Link de afiliado: posso receber comissão se você comprar.",
        "kill_rules": {"no_clicks_days": 14, "no_commission_days": 45,
            "max_uncertain_age_hours": 24, "validator_rejections_per_day": 3,
            "source_outage_hours": 48}
    })
}

fn policy(value: serde_json::Value) -> PolicyDocument {
    serde_json::from_value(value).unwrap()
}

#[test]
fn policy_document_rejects_unknown_fields_and_out_of_range_caps() {
    let mut value = valid();
    value["silent_production_grant"] = json!(true);
    assert!(serde_json::from_value::<PolicyDocument>(value).is_err());
    let mut value = valid();
    value["caps"]["extra_usage"] = json!(true);
    assert!(serde_json::from_value::<PolicyDocument>(value).is_err());
    for (field, rejected) in [
        ("posts_per_day", 0),
        ("posts_per_day", 11),
        ("ai_calls_per_day", 21),
        ("allowance_share_percent", 11),
        ("offers_per_sync", 0),
        ("offers_per_sync", 501),
        ("max_price_age_hours", 0),
        ("max_price_age_hours", 25),
    ] {
        let mut value = valid();
        value["caps"][field] = json!(rejected);
        assert!(policy(value).validate().is_err(), "{field}={rejected}");
    }
    let mut value = valid();
    value["version"] = json!(2);
    assert!(policy(value).validate().is_err());
}

#[test]
fn policy_requires_explicit_modes_and_does_not_activate_authority() {
    let value = valid();
    let p = policy(value.clone());
    assert_eq!(p.actions.publish, ActionMode::Off);
    assert_eq!(p.repost_cooldown_days, 14);
    assert!(p.validate().is_ok());
    let mut missing = value;
    missing["actions"]
        .as_object_mut()
        .unwrap()
        .remove("publish");
    assert!(serde_json::from_value::<PolicyDocument>(missing).is_err());
    let mut invalid = valid();
    invalid["actions"]["edit_published"] = json!("auto");
    assert!(policy(invalid).validate().is_err());
    let mut invalid = valid();
    invalid["actions"]["draft_model"] = json!("auto");
    assert!(policy(invalid).validate().is_err());
}

#[test]
fn policy_rejects_duplicate_channels_and_unbounded_or_empty_categories() {
    let mut value = valid();
    value["channels"] = json!([
        {"id": "fixture-channel", "program": "shopee"},
        {"id": "fixture-channel", "program": "mercado_livre"}
    ]);
    assert!(policy(value).validate().is_err());
    for rejected in [
        json!([]),
        json!(["accessories", "accessories"]),
        json!([""]),
        json!(["accessories\nignored"]),
    ] {
        let mut value = valid();
        value["categories"] = rejected;
        assert!(policy(value).validate().is_err());
    }
}

#[test]
fn policy_rejects_empty_disclosure_invalid_price_band_and_bad_thresholds() {
    let mut value = valid();
    value["disclosure_text"] = json!("   ");
    assert!(policy(value).validate().is_err());
    let mut value = valid();
    value["price_band"]["minimum"]["minor"] = json!(30001);
    assert!(policy(value).validate().is_err());
    let mut value = valid();
    value["price_band"]["minimum"]["minor"] = json!(-1);
    assert!(policy(value).validate().is_err());
    let mut value = valid();
    value["kill_rules"]["no_clicks_days"] = json!(0);
    assert!(policy(value).validate().is_err());
}

#[test]
fn policy_rejects_overlapping_windows_and_unknown_timezone() {
    let mut value = valid();
    value["windows"] = json!([
        {"start_minute": 690, "end_minute": 750},
        {"start_minute": 749, "end_minute": 800}
    ]);
    assert!(policy(value).validate().is_err());
    for (start, end) in [(100, 100), (1300, 100), (0, 1441)] {
        let mut value = valid();
        value["windows"] = json!([{"start_minute": start, "end_minute": end}]);
        assert!(policy(value).validate().is_err());
    }
    let mut value = valid();
    value["timezone"] = json!("America/Unknown");
    assert!(policy(value).validate().is_err());
}

#[test]
fn policy_hash_is_canonical_and_changes_with_material_scope() {
    let p = policy(valid());
    let reversed = valid()
        .as_object()
        .unwrap()
        .iter()
        .rev()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<serde_json::Map<_, _>>();
    let same = policy(serde_json::Value::Object(reversed));
    assert_eq!(p.canonical_json().unwrap(), same.canonical_json().unwrap());
    assert_eq!(p.sha256().unwrap(), same.sha256().unwrap());
    assert_eq!(p.sha256().unwrap().len(), 64);
    let mut changed = p.clone();
    changed.actions.publish = ActionMode::Approve;
    assert_ne!(p.sha256().unwrap(), changed.sha256().unwrap());
    let mut invalid = p;
    invalid.channels.clear();
    assert!(invalid.sha256().is_err());
}

#[test]
fn policy_and_template_share_disclosure_acceptance() {
    let accepted = policy(valid());
    let brand = |disclosure_text: String| BrandText {
        name: "Fixture Agency".into(),
        revision: "fixture-v1".into(),
        disclosure_text,
    };
    assert!(accepted.validate().is_ok());
    assert!(brand(accepted.disclosure_text.clone()).validate());
    for rejected in [
        format!("{}{}", accepted.disclosure_text, "x".repeat(240)),
        "Link de afiliado: posso receber comissão de 5 por cento.".into(),
        "Link de afiliado: posso receber comissão em https://outside.example.".into(),
        "Link de afiliado: posso receber comissão.\u{200b}".into(),
        "<span hidden>Link de afiliado: posso receber comissão.</span>".into(),
        "Aproveite esta oportunidade.".into(),
        "Não somos afiliados e não recebemos comissão.".into(),
        "Publicidade: afiliados oficiais garantem comissão para quem comprar.".into(),
    ] {
        let mut changed = accepted.clone();
        changed.disclosure_text = rejected.clone();
        assert_eq!(changed.validate(), Err(PolicyError::Disclosure));
        assert!(!brand(rejected).validate());
    }
}
