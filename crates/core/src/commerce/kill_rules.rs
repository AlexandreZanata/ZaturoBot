//! Pure stop recommendations from complete measured windows; unknown metrics stay unknown.
use serde::{Deserialize, Serialize};
const HOUR_MILLIS: i64 = 3_600_000;
const DAY_MILLIS: i64 = 24 * HOUR_MILLIS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KillRuleThresholds {
    pub no_clicks_days: u16,
    pub no_commission_days: u16,
    pub max_uncertain_age_hours: u16,
    pub validator_rejections_per_day: u16,
    pub source_outage_hours: u16,
}
impl Default for KillRuleThresholds {
    fn default() -> Self {
        Self {
            no_clicks_days: 14,
            no_commission_days: 45,
            max_uncertain_age_hours: 24,
            validator_rejections_per_day: 3,
            source_outage_hours: 48,
        }
    }
}
impl KillRuleThresholds {
    pub fn validate(&self) -> bool {
        (1..=366).contains(&self.no_clicks_days)
            && (1..=366).contains(&self.no_commission_days)
            && (1..=8760).contains(&self.max_uncertain_age_hours)
            && (1..=1000).contains(&self.validator_rejections_per_day)
            && (1..=8760).contains(&self.source_outage_hours)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnownCounter {
    pub value: u64,
    pub window_started_at: i64,
    pub observed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KillMetrics {
    pub campaign_started_at: i64,
    /// The current uninterrupted automatic-publication period; drafts are not revenue exposure.
    pub auto_started_at: Option<i64>,
    pub attributed_clicks: Option<KnownCounter>,
    pub approved_commission_minor: Option<KnownCounter>,
    pub oldest_uncertain_at: Option<i64>,
    pub validator_rejections_today: Option<KnownCounter>,
    pub source_outage_since: Option<i64>,
    pub channel_blocked: bool,
    pub model_allowance_exhausted: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KillAction {
    Pause,
    DemoteToApproval,
    OmitPrices,
    StopModel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KillReason {
    NoClicks,
    NoCommission,
    ChannelBlocked,
    UncertainAge,
    ValidatorRejections,
    SourceOutage,
    ModelAllowance,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KillDecision {
    pub action: KillAction,
    pub reason: KillReason,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillRuleError {
    InvalidThresholds,
    InvalidClock,
}

/// `observed_at == now` proves a current snapshot; short or stale windows cannot prove zero.
pub fn evaluate_kill_rules(
    thresholds: &KillRuleThresholds,
    metrics: &KillMetrics,
    now: i64,
) -> Result<Vec<KillDecision>, KillRuleError> {
    if !thresholds.validate() {
        return Err(KillRuleError::InvalidThresholds);
    }
    if now < 0
        || metrics.campaign_started_at < 0
        || metrics.campaign_started_at > now
        || metrics
            .auto_started_at
            .is_some_and(|time| time < metrics.campaign_started_at || time > now)
        || [metrics.oldest_uncertain_at, metrics.source_outage_since]
            .into_iter()
            .flatten()
            .any(|time| time < 0 || time > now)
        || [
            metrics.attributed_clicks,
            metrics.approved_commission_minor,
            metrics.validator_rejections_today,
        ]
        .into_iter()
        .flatten()
        .any(|counter| {
            counter.window_started_at < 0
                || counter.window_started_at > counter.observed_at
                || counter.observed_at > now
        })
    {
        return Err(KillRuleError::InvalidClock);
    }
    let mut decisions = Vec::new();
    let elapsed = now - metrics.campaign_started_at;
    let mut push = |action, reason| decisions.push(KillDecision { action, reason });
    let complete_zero = |counter: Option<KnownCounter>, days: u16| {
        let window = i64::from(days) * DAY_MILLIS;
        elapsed >= window
            && counter.is_some_and(|counter| {
                counter.value == 0
                    && counter.observed_at == now
                    && counter.window_started_at <= now - window
            })
    };
    if complete_zero(metrics.attributed_clicks, thresholds.no_clicks_days) {
        push(KillAction::Pause, KillReason::NoClicks);
    }
    if metrics.auto_started_at.is_some_and(|start| {
        now - start >= i64::from(thresholds.no_commission_days) * DAY_MILLIS
            && metrics.approved_commission_minor.is_some_and(|counter| {
                counter.value == 0
                    && counter.observed_at == now
                    && counter.window_started_at <= start
            })
    }) {
        push(KillAction::Pause, KillReason::NoCommission);
    }
    if metrics.channel_blocked {
        push(KillAction::DemoteToApproval, KillReason::ChannelBlocked);
    }
    if metrics.oldest_uncertain_at.is_some_and(|time| {
        now - time >= i64::from(thresholds.max_uncertain_age_hours) * HOUR_MILLIS
    }) {
        push(KillAction::DemoteToApproval, KillReason::UncertainAge);
    }
    let day_start = now.div_euclid(DAY_MILLIS) * DAY_MILLIS;
    if metrics.validator_rejections_today.is_some_and(|counter| {
        counter.observed_at == now
            && counter.window_started_at == day_start
            && counter.value >= u64::from(thresholds.validator_rejections_per_day)
    }) {
        push(
            KillAction::DemoteToApproval,
            KillReason::ValidatorRejections,
        );
    }
    if metrics
        .source_outage_since
        .is_some_and(|time| now - time >= i64::from(thresholds.source_outage_hours) * HOUR_MILLIS)
    {
        push(KillAction::OmitPrices, KillReason::SourceOutage);
    }
    if metrics.model_allowance_exhausted {
        push(KillAction::StopModel, KillReason::ModelAllowance);
    }
    Ok(decisions)
}
