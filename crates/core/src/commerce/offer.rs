//! Bounded owner-imported offer facts. URL validation never proves platform eligibility.
use super::money::{Currency, Money};
use serde::{Deserialize, Serialize};
use url::Url;

/// Conservative default only when an owner import supplies no explicit validity time.
pub const DEFAULT_OFFER_VALIDITY_MS: i64 = 6 * 60 * 60 * 1000;
const MAX_OFFER_INPUT_BYTES: usize = 4096;
const MAX_AFFILIATE_URL_BYTES: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffiliateProgram {
    Shopee,
    MercadoLivre,
}
impl AffiliateProgram {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Shopee => "shopee",
            Self::MercadoLivre => "mercado_livre",
        }
    }

    /// Validates exact reviewed URL authorities, without fetching or modifying the URL.
    /// The literal `https://` prefix and a nonempty raw authority prevent parser normalization
    /// from admitting extra slash, encoded authority or empty-userinfo forms.
    /// Link provenance, program enrollment and channel eligibility require separate evidence.
    pub fn permits_url(self, value: &str) -> bool {
        if !value.starts_with("https://")
            || value.len() > MAX_AFFILIATE_URL_BYTES
            || value.chars().any(|c| c.is_control() || c.is_whitespace())
            || value.contains('\\')
            || value
                .strip_prefix("https://")
                .and_then(|v| v.split(['/', '?', '#']).next())
                .is_none_or(|authority| {
                    authority.is_empty() || authority.contains('@') || authority.contains('%')
                })
        {
            return false;
        }
        let Ok(url) = Url::parse(value) else {
            return false;
        };
        if url.scheme() != "https"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port_or_known_default() != Some(443)
            || url.fragment().is_some()
        {
            return false;
        }
        let Some(host) = url.host_str() else {
            return false;
        };
        match self {
            Self::Shopee => matches!(
                host,
                "shopee.com.br" | "www.shopee.com.br" | "s.shopee.com.br"
            ),
            Self::MercadoLivre => matches!(
                host,
                "mercadolivre.com.br"
                    | "www.mercadolivre.com.br"
                    | "mercadolivre.com"
                    | "www.mercadolivre.com"
            ),
        }
    }
}

/// Stable validation codes; no imported values are embedded in errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectReason {
    InputTooLarge,
    InvalidItemId,
    InvalidTitle,
    InvalidCategory,
    InvalidPrice,
    MissingObservation,
    InvalidObservation,
    InvalidValidity,
    InvalidAffiliateUrl,
    InvalidCommission,
    InvalidSubId,
    InvalidEvidence,
    InvalidBenefits,
}

/// Import-facing decimal amount and optional source timestamps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfferInput {
    pub program: AffiliateProgram,
    pub item_id: String,
    pub title: String,
    pub category: String,
    pub price: String,
    pub currency: Currency,
    pub observed_at: Option<i64>,
    pub valid_until: Option<i64>,
    pub affiliate_url: String,
    pub commission_bp: Option<u16>,
    pub sub_id: Option<String>,
    pub evidence_sha256: String,
    pub benefits: Vec<String>,
}

/// Canonical imported facts, revalidated against injected UTC milliseconds before use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offer {
    pub program: AffiliateProgram,
    pub item_id: String,
    pub title: String,
    pub category: String,
    pub price: Money,
    pub price_observed_at: i64,
    pub valid_until: i64,
    pub affiliate_url: String,
    pub commission_bp: Option<u16>,
    pub sub_id: Option<String>,
    pub evidence_sha256: String,
    pub benefits: Vec<String>,
}

fn safe_text(value: &str, limit: usize) -> bool {
    crate::ordinary_text(value, limit) && !value.chars().any(char::is_control)
}
fn valid_sub_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
fn valid_evidence(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && value.bytes().any(|b| b != b'0')
}
fn push(reasons: &mut Vec<RejectReason>, reason: RejectReason) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}
impl OfferInput {
    /// Removes title control characters, then validates all fields. Missing validity uses
    /// observation plus six hours; stale price age is independent of explicit offer expiry.
    pub fn validate(&self, now: i64) -> Result<Offer, Vec<RejectReason>> {
        if self.benefits.len() > 3 || self.title.len() > 1024 {
            return Err(vec![RejectReason::InputTooLarge]);
        }
        let total = [
            self.item_id.len(),
            self.title.len(),
            self.category.len(),
            self.price.len(),
            self.affiliate_url.len(),
            self.sub_id.as_ref().map_or(0, String::len),
            self.evidence_sha256.len(),
        ]
        .into_iter()
        .chain(self.benefits.iter().map(String::len))
        .try_fold(0_usize, usize::checked_add);
        if total.is_none_or(|n| n > MAX_OFFER_INPUT_BYTES) {
            return Err(vec![RejectReason::InputTooLarge]);
        }
        let mut reasons = Vec::new();
        let title = self.title.chars().filter(|c| !c.is_control()).collect();
        let price = Money::parse_decimal(&self.price, self.currency);
        if price.is_err() {
            reasons.push(RejectReason::InvalidPrice);
        }
        let observed = match self.observed_at {
            Some(value) => value,
            None => {
                reasons.push(RejectReason::MissingObservation);
                -1
            }
        };
        let validity = self
            .valid_until
            .or_else(|| observed.checked_add(DEFAULT_OFFER_VALIDITY_MS));
        if validity.is_none() {
            reasons.push(RejectReason::InvalidValidity);
        }
        let offer = Offer {
            program: self.program,
            item_id: self.item_id.clone(),
            title,
            category: self.category.clone(),
            price: price.unwrap_or(Money {
                minor: -1,
                currency: self.currency,
            }),
            price_observed_at: observed,
            valid_until: validity.unwrap_or(-1),
            affiliate_url: self.affiliate_url.clone(),
            commission_bp: self.commission_bp,
            sub_id: self.sub_id.clone(),
            evidence_sha256: self.evidence_sha256.clone(),
            benefits: self.benefits.clone(),
        };
        if let Err(errors) = offer.validate(now) {
            for reason in errors {
                if reason != RejectReason::InvalidObservation || self.observed_at.is_some() {
                    push(&mut reasons, reason);
                }
            }
        }
        if reasons.is_empty() {
            Ok(offer)
        } else {
            Err(reasons)
        }
    }
}
impl Offer {
    /// Checks canonical fields again; deserialization or public fields grant no validity.
    pub fn validate(&self, now: i64) -> Result<(), Vec<RejectReason>> {
        let mut reasons = Vec::new();
        if crate::validate_identifier(&self.item_id).is_err() {
            reasons.push(RejectReason::InvalidItemId);
        }
        if !safe_text(&self.title, 200) {
            reasons.push(RejectReason::InvalidTitle);
        }
        if !safe_text(&self.category, 64) {
            reasons.push(RejectReason::InvalidCategory);
        }
        if self.price.validate().is_err() {
            reasons.push(RejectReason::InvalidPrice);
        }
        if now < 0 || self.price_observed_at < 0 || self.price_observed_at > now {
            reasons.push(RejectReason::InvalidObservation);
        }
        if self.valid_until <= now || self.valid_until <= self.price_observed_at {
            reasons.push(RejectReason::InvalidValidity);
        }
        if !self.program.permits_url(&self.affiliate_url) {
            reasons.push(RejectReason::InvalidAffiliateUrl);
        }
        if self.commission_bp.is_some_and(|v| v > 10_000) {
            reasons.push(RejectReason::InvalidCommission);
        }
        if self.sub_id.as_deref().is_some_and(|v| !valid_sub_id(v)) {
            reasons.push(RejectReason::InvalidSubId);
        }
        if !valid_evidence(&self.evidence_sha256) {
            reasons.push(RejectReason::InvalidEvidence);
        }
        if !(1..=3).contains(&self.benefits.len())
            || self.benefits.iter().any(|b| !safe_text(b, 160))
        {
            reasons.push(RejectReason::InvalidBenefits);
        }
        if reasons.is_empty() {
            Ok(())
        } else {
            Err(reasons)
        }
    }
}
