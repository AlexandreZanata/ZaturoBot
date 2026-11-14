use chrono::NaiveDate;
use serde_json::json;
use zaturo_core::commerce::{
    policy::PolicyDocument,
    slots::{SlotDisposition, SlotKey, slots_for_date},
};

fn policy() -> PolicyDocument {
    serde_json::from_value(json!({
        "version": 1,
        "actions": {"import_offers": "auto", "sync_offers": "off", "rank_offers": "auto",
            "draft_template": "draft", "draft_model": "off", "schedule": "off",
            "publish": "off", "support_reply": "off", "import_conversions": "auto",
            "optimize": "off", "edit_published": "off"},
        "caps": {"posts_per_day": 2, "ai_calls_per_day": 0, "allowance_share_percent": 0,
            "offers_per_sync": 50, "max_price_age_hours": 6},
        "timezone": "America/Cuiaba",
        "windows": [{"start_minute": 1200, "end_minute": 1320},
                    {"start_minute": 30, "end_minute": 60}],
        "channels": [{"id": "fixture-channel", "program": "shopee"}],
        "categories": ["accessories"],
        "price_band": {"minimum": {"minor": 3000, "currency": "BRL"},
                       "maximum": {"minor": 30000, "currency": "BRL"}},
        "disclosure_text": "Link de afiliado: posso receber comissão se você comprar.",
        "kill_rules": {"no_clicks_days": 14, "no_commission_days": 45,
            "max_uncertain_age_hours": 24, "validator_rejections_per_day": 3,
            "source_outage_hours": 48}
    }))
    .unwrap()
}

#[test]
fn slots_for_cuiaba_are_stable_across_utc_midnight() {
    let p = policy();
    let date = NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
    let slots = slots_for_date(&p, "campaign-fixture", date).unwrap();
    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0].key.as_str(), "campaign-fixture/2026-11-14/0");
    assert_eq!(slots[1].key.as_str(), "campaign-fixture/2026-11-14/1");
    assert_eq!(slots[0].due_at, 1_794_630_600_000);
    assert_eq!(slots[1].due_at, 1_794_700_800_000);
    assert_eq!(slots[1].window_ends_at, 1_794_708_000_000);
    assert_eq!(slots, slots_for_date(&p, "campaign-fixture", date).unwrap());
}

#[test]
fn missed_slot_is_skipped_not_caught_up_pure() {
    let slot = slots_for_date(
        &policy(),
        "campaign-fixture",
        NaiveDate::from_ymd_opt(2026, 11, 14).unwrap(),
    )
    .unwrap()
    .remove(0);
    assert_eq!(
        slot.disposition(slot.due_at - 1).unwrap(),
        SlotDisposition::Pending
    );
    assert_eq!(slot.disposition(slot.due_at).unwrap(), SlotDisposition::Due);
    assert_eq!(
        slot.disposition(slot.window_ends_at - 1).unwrap(),
        SlotDisposition::Due
    );
    assert_eq!(
        slot.disposition(slot.window_ends_at).unwrap(),
        SlotDisposition::SkippedMissedWindow
    );
    assert_eq!(
        slot.disposition(slot.window_ends_at + 86_400_000).unwrap(),
        SlotDisposition::SkippedMissedWindow
    );
    assert!(slot.disposition(-1).is_err());
}

#[test]
fn slot_keys_and_windows_reject_invalid_or_excessive_input() {
    let date = NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
    assert!(SlotKey::new("../other", date, 0).is_err());
    assert!(SlotKey::new("campaign", date, 10).is_err());
    let mut p = policy();
    p.caps.posts_per_day = 1;
    assert!(slots_for_date(&p, "campaign", date).is_err());
    let mut p = policy();
    p.windows[1] = p.windows[0].clone();
    assert!(slots_for_date(&p, "campaign", date).is_err());
    assert!(slots_for_date(&policy(), "campaign", NaiveDate::MIN).is_err());
    assert!(slots_for_date(&policy(), "campaign", NaiveDate::MAX).is_err());
}

#[test]
fn maintained_timezone_resolution_rejects_gap_and_fold_without_guessing() {
    let mut p = policy();
    p.timezone = "America/New_York".into();
    p.windows.truncate(1);
    p.windows[0].start_minute = 150;
    p.windows[0].end_minute = 180;
    assert!(slots_for_date(&p, "campaign", NaiveDate::from_ymd_opt(2026, 3, 8).unwrap()).is_err());
    p.windows[0].start_minute = 90;
    p.windows[0].end_minute = 120;
    assert!(
        slots_for_date(
            &p,
            "campaign",
            NaiveDate::from_ymd_opt(2026, 11, 1).unwrap()
        )
        .is_err()
    );
}

#[test]
fn midnight_window_end_is_next_local_day_and_keys_never_collide() {
    let mut p = policy();
    p.windows.truncate(1);
    p.windows[0].start_minute = 1380;
    p.windows[0].end_minute = 1440;
    let day = NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
    let first = slots_for_date(&p, "campaign", day).unwrap().remove(0);
    let second = slots_for_date(&p, "campaign", day.succ_opt().unwrap())
        .unwrap()
        .remove(0);
    assert_eq!(first.window_ends_at - first.due_at, 3_600_000);
    assert_ne!(first.key, second.key);
}
