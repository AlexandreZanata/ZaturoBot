//! Pure daily-window calculations. No clock access, task admission or dispatch occurs here.
use super::policy::PolicyDocument;
use chrono::{Datelike, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct SlotKey(String);
impl SlotKey {
    pub fn new(campaign: &str, local_date: NaiveDate, index: u16) -> Result<Self, SlotError> {
        if crate::validate_identifier(campaign).is_err()
            || !(1970..=9999).contains(&local_date.year())
            || index >= 10
        {
            return Err(SlotError::Key);
        }
        Ok(Self(format!("{campaign}/{local_date}/{index}")))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for SlotKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EditorialSlot {
    pub key: SlotKey,
    pub due_at: i64,
    pub window_ends_at: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotDisposition {
    Pending,
    Due,
    SkippedMissedWindow,
}
impl EditorialSlot {
    pub fn disposition(&self, now: i64) -> Result<SlotDisposition, SlotError> {
        if now < 0 || self.due_at < 0 || self.window_ends_at <= self.due_at {
            return Err(SlotError::Time);
        }
        Ok(if now < self.due_at {
            SlotDisposition::Pending
        } else if now < self.window_ends_at {
            SlotDisposition::Due
        } else {
            SlotDisposition::SkippedMissedWindow
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotError {
    Policy,
    Key,
    Time,
    LocalTime,
}
impl fmt::Display for SlotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Policy => "invalid commerce scheduling policy",
            Self::Key => "invalid bounded commerce occurrence key",
            Self::Time => "commerce occurrence time is outside supported bounds",
            Self::LocalTime => "local window time is ambiguous or nonexistent",
        })
    }
}
impl std::error::Error for SlotError {}

/// One occurrence per sorted daily window; missed windows are never shifted forward.
/// Dates outside 1970..=9999 and non-single timezone resolutions are rejected.
pub fn slots_for_date(
    policy: &PolicyDocument,
    campaign: &str,
    local_date: NaiveDate,
) -> Result<Vec<EditorialSlot>, SlotError> {
    policy.validate().map_err(|_| SlotError::Policy)?;
    SlotKey::new(campaign, local_date, 0)?;
    let zone = policy
        .timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| SlotError::Policy)?;
    let mut windows: Vec<_> = policy.windows.iter().collect();
    windows.sort_by_key(|window| window.start_minute);
    let mut slots = Vec::with_capacity(windows.len());
    for (index, window) in windows.into_iter().enumerate() {
        let due_at = resolve(zone, local_date, window.start_minute)?;
        let window_ends_at = resolve(zone, local_date, window.end_minute)?;
        if due_at < 0 || window_ends_at <= due_at {
            return Err(SlotError::Time);
        }
        slots.push(EditorialSlot {
            key: SlotKey::new(
                campaign,
                local_date,
                u16::try_from(index).map_err(|_| SlotError::Key)?,
            )?,
            due_at,
            window_ends_at,
        });
    }
    Ok(slots)
}

fn resolve(zone: chrono_tz::Tz, date: NaiveDate, minute: u16) -> Result<i64, SlotError> {
    let (date, minute) = if minute == 1440 {
        let next = date.succ_opt().ok_or(SlotError::Time)?;
        if next.year() > 9999 {
            return Err(SlotError::Time);
        }
        (next, 0)
    } else {
        (date, minute)
    };
    let local = date
        .and_hms_opt(u32::from(minute / 60), u32::from(minute % 60), 0)
        .ok_or(SlotError::Time)?;
    zone.from_local_datetime(&local)
        .single()
        .map(|time| time.timestamp_millis())
        .ok_or(SlotError::LocalTime)
}
