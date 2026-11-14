use zaturo_core::commerce::{
    KillAction, KillDecision, KillMetrics, KillReason, KillRuleThresholds, KnownCounter,
    evaluate_kill_rules,
};
const DAY: i64 = 86_400_000;
const HOUR: i64 = 3_600_000;
const NOW: i64 = 100 * DAY;
fn metrics() -> KillMetrics {
    KillMetrics {
        campaign_started_at: NOW - 60 * DAY,
        auto_started_at: Some(NOW - 45 * DAY),
        attributed_clicks: None,
        approved_commission_minor: None,
        oldest_uncertain_at: None,
        validator_rejections_today: None,
        source_outage_since: None,
        channel_blocked: false,
        model_allowance_exhausted: false,
    }
}
fn counter(value: u64, started: i64) -> KnownCounter {
    KnownCounter {
        value,
        window_started_at: started,
        observed_at: NOW,
    }
}

#[test]
fn kill_rule_no_clicks_14_days_pauses() {
    let mut m = metrics();
    m.attributed_clicks = Some(counter(0, NOW - 14 * DAY));
    assert_eq!(
        evaluate_kill_rules(&KillRuleThresholds::default(), &m, NOW).unwrap(),
        vec![KillDecision {
            action: KillAction::Pause,
            reason: KillReason::NoClicks
        }]
    );
    m.attributed_clicks = Some(counter(0, NOW - 14 * DAY + 1));
    assert!(
        evaluate_kill_rules(&KillRuleThresholds::default(), &m, NOW)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn kill_rule_thresholds_are_inclusive_and_deterministic() {
    let thresholds = KillRuleThresholds::default();
    let mut m = metrics();
    m.attributed_clicks = Some(counter(0, NOW - 14 * DAY));
    m.approved_commission_minor = Some(counter(0, NOW - 45 * DAY));
    m.oldest_uncertain_at = Some(NOW - 24 * HOUR);
    m.validator_rejections_today = Some(counter(3, NOW));
    m.source_outage_since = Some(NOW - 48 * HOUR);
    m.channel_blocked = true;
    m.model_allowance_exhausted = true;
    let decisions = evaluate_kill_rules(&thresholds, &m, NOW).unwrap();
    assert_eq!(
        decisions.iter().map(|d| d.reason).collect::<Vec<_>>(),
        vec![
            KillReason::NoClicks,
            KillReason::NoCommission,
            KillReason::ChannelBlocked,
            KillReason::UncertainAge,
            KillReason::ValidatorRejections,
            KillReason::SourceOutage,
            KillReason::ModelAllowance
        ]
    );
    assert_eq!(
        decisions,
        evaluate_kill_rules(&thresholds, &m, NOW).unwrap()
    );
    m.oldest_uncertain_at = Some(NOW - 24 * HOUR + 1);
    m.validator_rejections_today = Some(counter(2, NOW));
    m.source_outage_since = Some(NOW - 48 * HOUR + 1);
    let reasons = evaluate_kill_rules(&thresholds, &m, NOW)
        .unwrap()
        .into_iter()
        .map(|d| d.reason)
        .collect::<Vec<_>>();
    assert!(!reasons.contains(&KillReason::UncertainAge));
    assert!(!reasons.contains(&KillReason::ValidatorRejections));
    assert!(!reasons.contains(&KillReason::SourceOutage));
}

#[test]
fn missing_stale_or_incomplete_metrics_are_not_zero() {
    let thresholds = KillRuleThresholds::default();
    assert!(
        evaluate_kill_rules(&thresholds, &metrics(), NOW)
            .unwrap()
            .is_empty()
    );
    let mut m = metrics();
    m.attributed_clicks = Some(counter(1, NOW - 14 * DAY));
    m.approved_commission_minor = Some(counter(1, NOW - 45 * DAY));
    assert!(
        evaluate_kill_rules(&thresholds, &m, NOW)
            .unwrap()
            .is_empty()
    );
    m.attributed_clicks = Some(KnownCounter {
        observed_at: NOW - 1,
        ..counter(0, NOW - 14 * DAY)
    });
    m.approved_commission_minor = Some(counter(0, NOW - 45 * DAY + 1));
    assert!(
        evaluate_kill_rules(&thresholds, &m, NOW)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn invalid_thresholds_and_future_clock_evidence_are_rejected() {
    assert!(
        !KillRuleThresholds {
            no_clicks_days: 0,
            ..KillRuleThresholds::default()
        }
        .validate()
    );
    let mut m = metrics();
    m.source_outage_since = Some(NOW + 1);
    assert!(evaluate_kill_rules(&KillRuleThresholds::default(), &m, NOW).is_err());
    m.source_outage_since = None;
    m.attributed_clicks = Some(counter(0, NOW + 1));
    assert!(evaluate_kill_rules(&KillRuleThresholds::default(), &m, NOW).is_err());
    assert!(evaluate_kill_rules(&KillRuleThresholds::default(), &metrics(), i64::MIN).is_err());
}

#[test]
fn no_commission_requires_a_complete_known_window_since_auto_activation() {
    let thresholds = KillRuleThresholds::default();
    let mut m = metrics();
    m.approved_commission_minor = Some(counter(0, NOW - 60 * DAY));
    for auto_started_at in [None, Some(NOW - 45 * DAY + 1), Some(NOW - DAY)] {
        m.auto_started_at = auto_started_at;
        assert!(
            !evaluate_kill_rules(&thresholds, &m, NOW)
                .unwrap()
                .iter()
                .any(|decision| decision.reason == KillReason::NoCommission)
        );
    }
    m.auto_started_at = Some(NOW - 45 * DAY);
    assert!(
        evaluate_kill_rules(&thresholds, &m, NOW)
            .unwrap()
            .iter()
            .any(|decision| decision.reason == KillReason::NoCommission)
    );
    m.auto_started_at = Some(NOW - 50 * DAY);
    m.approved_commission_minor = Some(counter(0, NOW - 45 * DAY));
    assert!(
        !evaluate_kill_rules(&thresholds, &m, NOW)
            .unwrap()
            .iter()
            .any(|decision| decision.reason == KillReason::NoCommission)
    );
    m.auto_started_at = Some(NOW - 61 * DAY);
    assert!(evaluate_kill_rules(&thresholds, &m, NOW).is_err());
    m.auto_started_at = Some(NOW + 1);
    assert!(evaluate_kill_rules(&thresholds, &m, NOW).is_err());
}
