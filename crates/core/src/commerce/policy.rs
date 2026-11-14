//! Versioned pure policy proposals; validation never activates production authority.
use super::{
    claims::valid_disclosure,
    kill_rules::KillRuleThresholds,
    money::{Currency, Money},
    offer::AffiliateProgram,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionMode {
    Off,
    Draft,
    Approve,
    Auto,
}

/// Every action is explicit. Deserialization supplies no default permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionModes {
    pub import_offers: ActionMode,
    pub sync_offers: ActionMode,
    pub rank_offers: ActionMode,
    pub draft_template: ActionMode,
    pub draft_model: ActionMode,
    pub schedule: ActionMode,
    pub publish: ActionMode,
    pub support_reply: ActionMode,
    pub import_conversions: ActionMode,
    pub optimize: ActionMode,
    pub edit_published: ActionMode,
}
impl ActionModes {
    fn validates(&self) -> bool {
        use ActionMode::*;
        [
            self.import_offers,
            self.sync_offers,
            self.rank_offers,
            self.schedule,
            self.support_reply,
            self.import_conversions,
            self.optimize,
        ]
        .into_iter()
        .all(|mode| matches!(mode, Off | Auto))
            && matches!(self.draft_template, Off | Draft | Auto)
            && matches!(self.draft_model, Off | Draft | Auto)
            && matches!(self.edit_published, Off | Approve)
    }
}

/// Pilot hard bounds are local maxima, not an upstream allowance or spending grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyCaps {
    pub posts_per_day: u16,
    pub ai_calls_per_day: u16,
    pub allowance_share_percent: u8,
    pub offers_per_sync: u16,
    pub max_price_age_hours: u16,
}
impl PolicyCaps {
    fn validates(&self) -> bool {
        (1..=10).contains(&self.posts_per_day)
            && self.ai_calls_per_day <= 20
            && self.allowance_share_percent <= 10
            && (self.ai_calls_per_day == 0) == (self.allowance_share_percent == 0)
            && (1..=500).contains(&self.offers_per_sync)
            && (1..=24).contains(&self.max_price_age_hours)
    }
}

/// Half-open local window [start_minute, end_minute), within one calendar day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostingWindow {
    pub start_minute: u16,
    pub end_minute: u16,
}
impl PostingWindow {
    pub fn validates(&self) -> bool {
        self.start_minute < self.end_minute && self.end_minute <= 1440
    }
}

/// A logical registered channel identifier maps to exactly one affiliate program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelPolicy {
    pub id: String,
    pub program: AffiliateProgram,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PriceBand {
    pub minimum: Money,
    pub maximum: Money,
}
impl PriceBand {
    fn validates(&self) -> bool {
        self.minimum.currency == Currency::Brl
            && self.maximum.currency == Currency::Brl
            && self.minimum.minor >= 0
            && self.maximum.minor > 0
            && self.minimum.minor <= self.maximum.minor
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDocument {
    pub version: u16,
    pub actions: ActionModes,
    pub caps: PolicyCaps,
    pub timezone: String,
    pub windows: Vec<PostingWindow>,
    pub channels: Vec<ChannelPolicy>,
    pub categories: Vec<String>,
    pub price_band: PriceBand,
    pub disclosure_text: String,
    #[serde(default = "default_repost_cooldown")]
    pub repost_cooldown_days: u16,
    pub kill_rules: KillRuleThresholds,
}
fn default_repost_cooldown() -> u16 {
    14
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    Version,
    Modes,
    Caps,
    Timezone,
    Windows,
    Channels,
    Categories,
    PriceBand,
    Disclosure,
    Thresholds,
    Serialization,
}
impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Version => "unsupported commerce policy version",
            Self::Modes => "invalid or unbudgeted commerce action mode",
            Self::Caps => "commerce policy caps exceed pilot bounds",
            Self::Timezone => "commerce policy requires a known bounded IANA timezone",
            Self::Windows => "commerce windows are invalid, overlapping or exceed the daily cap",
            Self::Channels => "commerce channels require unique registered identifiers",
            Self::Categories => "commerce categories require unique bounded nonempty values",
            Self::PriceBand => "commerce price band must contain ordered nonnegative BRL amounts",
            Self::Disclosure => {
                "commerce disclosure violates the shared claim and template contract"
            }
            Self::Thresholds => "commerce cooldown or kill thresholds exceed bounds",
            Self::Serialization => "commerce policy serialization failed",
        })
    }
}
impl std::error::Error for PolicyError {}

impl PolicyDocument {
    pub fn validate(&self) -> Result<(), PolicyError> {
        if self.version != 1 {
            return Err(PolicyError::Version);
        }
        if !self.actions.validates()
            || (self.actions.draft_model != ActionMode::Off && self.caps.ai_calls_per_day == 0)
        {
            return Err(PolicyError::Modes);
        }
        if !self.caps.validates() {
            return Err(PolicyError::Caps);
        }
        if self.timezone.len() > 64 || self.timezone.parse::<chrono_tz::Tz>().is_err() {
            return Err(PolicyError::Timezone);
        }
        if self.windows.is_empty()
            || self.windows.len() > usize::from(self.caps.posts_per_day)
            || self.windows.iter().any(|window| !window.validates())
        {
            return Err(PolicyError::Windows);
        }
        let mut windows: Vec<_> = self.windows.iter().collect();
        windows.sort_by_key(|window| window.start_minute);
        if windows
            .windows(2)
            .any(|pair| pair[0].end_minute > pair[1].start_minute)
        {
            return Err(PolicyError::Windows);
        }
        let mut channels = HashSet::new();
        if self.channels.is_empty()
            || self.channels.len() > 8
            || self.channels.iter().any(|channel| {
                crate::validate_identifier(&channel.id).is_err() || !channels.insert(&channel.id)
            })
        {
            return Err(PolicyError::Channels);
        }
        let mut categories = HashSet::new();
        if self.categories.is_empty()
            || self.categories.len() > 32
            || self
                .categories
                .iter()
                .any(|category| !bounded_text(category, 64) || !categories.insert(category))
        {
            return Err(PolicyError::Categories);
        }
        if !self.price_band.validates() {
            return Err(PolicyError::PriceBand);
        }
        if !valid_disclosure(&self.disclosure_text) {
            return Err(PolicyError::Disclosure);
        }
        if !(1..=366).contains(&self.repost_cooldown_days) || !self.kill_rules.validate() {
            return Err(PolicyError::Thresholds);
        }
        Ok(())
    }

    /// Recursively sorted JSON object keys; array order remains policy data.
    pub fn canonical_json(&self) -> Result<String, PolicyError> {
        self.validate()?;
        let value = serde_json::to_value(self).map_err(|_| PolicyError::Serialization)?;
        let mut output = String::new();
        write_canonical(&value, &mut output)?;
        Ok(output)
    }

    pub fn sha256(&self) -> Result<String, PolicyError> {
        let hash = Sha256::digest(self.canonical_json()?.as_bytes());
        Ok(hash.iter().map(|byte| format!("{byte:02x}")).collect())
    }
}

fn bounded_text(value: &str, max_bytes: usize) -> bool {
    value.len() <= max_bytes
        && !value.trim().is_empty()
        && !value.chars().any(char::is_control)
        && value == value.trim()
}

fn write_canonical(value: &serde_json::Value, output: &mut String) -> Result<(), PolicyError> {
    match value {
        serde_json::Value::Object(map) => {
            output.push('{');
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output
                    .push_str(&serde_json::to_string(key).map_err(|_| PolicyError::Serialization)?);
                output.push(':');
                write_canonical(&map[key], output)?;
            }
            output.push('}');
        }
        serde_json::Value::Array(values) => {
            output.push('[');
            for (index, child) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_canonical(child, output)?;
            }
            output.push(']');
        }
        _ => {
            output.push_str(&serde_json::to_string(value).map_err(|_| PolicyError::Serialization)?)
        }
    }
    Ok(())
}
