//! Checked commercial amounts; decimal text is parsed without floating-point arithmetic.
use serde::{Deserialize, Serialize};
use std::fmt;

/// Reviewed currency set for the first commerce profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currency {
    /// Brazilian real, with two decimal minor-unit places.
    #[serde(rename = "BRL")]
    Brl,
}

/// A nonnegative integer amount in a supported currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Money {
    pub minor: i64,
    pub currency: Currency,
}

/// Safe amount-validation failure without echoing imported source values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoneyError {
    InvalidDecimal,
    NegativeAmount,
    Overflow,
}
impl fmt::Display for MoneyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidDecimal => "amount requires bounded decimal digits and at most two places",
            Self::NegativeAmount => "amount cannot be negative",
            Self::Overflow => "amount exceeds checked minor-unit capacity",
        })
    }
}
impl std::error::Error for MoneyError {}

impl Money {
    /// Accepts an ungrouped decimal string with a comma or dot decimal separator.
    /// Signs, whitespace, exponent notation and more than two decimal places are refused.
    pub fn parse_decimal(value: &str, currency: Currency) -> Result<Self, MoneyError> {
        if value.starts_with('-') {
            return Err(MoneyError::NegativeAmount);
        }
        if value.is_empty() || value.len() > 32 {
            return Err(MoneyError::InvalidDecimal);
        }
        let mut parts = value.split(['.', ',']);
        let whole = parts.next().ok_or(MoneyError::InvalidDecimal)?;
        let fraction = parts.next();
        if parts.next().is_some()
            || whole.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || fraction.is_some_and(|v| {
                v.is_empty() || v.len() > 2 || !v.bytes().all(|b| b.is_ascii_digit())
            })
        {
            return Err(MoneyError::InvalidDecimal);
        }
        let mut units = 0_i64;
        for digit in whole.bytes() {
            units = units
                .checked_mul(10)
                .and_then(|n| n.checked_add(i64::from(digit - b'0')))
                .ok_or(MoneyError::Overflow)?;
        }
        let mut cents = 0_i64;
        if let Some(fraction) = fraction {
            for digit in fraction.bytes() {
                cents = cents * 10 + i64::from(digit - b'0');
            }
            if fraction.len() == 1 {
                cents *= 10;
            }
        }
        let minor = units
            .checked_mul(100)
            .and_then(|n| n.checked_add(cents))
            .ok_or(MoneyError::Overflow)?;
        Ok(Self { minor, currency })
    }

    /// Revalidates directly constructed or deserialized values before use.
    pub fn validate(&self) -> Result<(), MoneyError> {
        if self.minor < 0 {
            Err(MoneyError::NegativeAmount)
        } else {
            Ok(())
        }
    }

    /// Formats BRL decimal text without grouping; callers validate domain values first.
    pub fn format_decimal(&self) -> String {
        let absolute = self.minor.unsigned_abs();
        let sign = if self.minor < 0 { "-" } else { "" };
        format!("{sign}{},{:02}", absolute / 100, absolute % 100)
    }
}
