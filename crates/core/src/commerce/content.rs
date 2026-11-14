//! Draft lifecycle and deterministic, bounded Telegram-text preparation; no publication.
use super::claims::{
    escape_html, fresh_price_line, has_unsafe_text, number_tokens, url_like_tokens,
    valid_disclosure,
};
use super::{ClaimContext, ClaimViolation, Offer, lint_claims};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentState {
    Draft,
    Approved,
    Scheduled,
    Publishing,
    Published,
    Rejected,
    Failed,
    Uncertain,
    Withdrawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentError {
    IllegalTransition,
    InvalidResolution,
}

impl ContentState {
    pub fn can_transition_to(self, next: Self) -> bool {
        use ContentState::*;
        matches!(
            (self, next),
            (Draft, Approved | Rejected | Withdrawn)
                | (Approved, Scheduled | Withdrawn)
                | (Scheduled, Publishing | Failed | Withdrawn)
                | (Publishing, Published | Failed | Uncertain)
        )
    }
    pub fn transition_to(self, next: Self) -> Result<Self, ContentError> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(ContentError::IllegalTransition)
        }
    }
    /// The caller must authenticate owner authority before accepting this explicit evidence.
    /// Resolution reports a known outcome and can never schedule or trigger a resend.
    pub fn resolve_uncertain(
        self,
        resolution: &OwnerContentResolution,
    ) -> Result<Self, ContentError> {
        if self != Self::Uncertain
            || resolution.evidence.trim().is_empty()
            || resolution.evidence.len() > 512
            || has_unsafe_text(&resolution.evidence, false)
        {
            return Err(ContentError::InvalidResolution);
        }
        Ok(match resolution.outcome {
            OwnerResolutionOutcome::ConfirmedPublished => Self::Published,
            OwnerResolutionOutcome::ConfirmedNotPublished => Self::Failed,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OwnerResolutionOutcome {
    ConfirmedPublished,
    ConfirmedNotPublished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerContentResolution {
    pub outcome: OwnerResolutionOutcome,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrandText {
    pub name: String,
    pub revision: String,
    pub disclosure_text: String,
}

impl BrandText {
    pub fn validate(&self) -> bool {
        !self.name.is_empty()
            && self.name.len() <= 80
            && self.name.trim() == self.name
            && !has_unsafe_text(&self.name, false)
            && !self.name.contains(['<', '>', '&'])
            && url_like_tokens(&self.name).is_empty()
            && number_tokens(&self.name).is_empty()
            && !self.revision.is_empty()
            && self.revision.len() <= 64
            && self
                .revision
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            && valid_disclosure(&self.disclosure_text)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateId {
    ListingV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderedContent {
    pub text: String,
    pub brand_revision: String,
    pub template_id: TemplateId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    InvalidOffer,
    InvalidBrand,
    InvalidPriceAge,
    InvalidTime,
    Claims(Vec<ClaimViolation>),
}

/// Renders verified listing facts only. It neither authorizes nor performs a send.
pub fn render(
    template_id: TemplateId,
    offer: &Offer,
    brand: &BrandText,
    now: i64,
    max_price_age_millis: i64,
) -> Result<RenderedContent, RenderError> {
    if offer.validate(now).is_err() {
        return Err(RenderError::InvalidOffer);
    }
    if !brand.validate() {
        return Err(RenderError::InvalidBrand);
    }
    if max_price_age_millis <= 0 {
        return Err(RenderError::InvalidPriceAge);
    }
    let age = now
        .checked_sub(offer.price_observed_at)
        .filter(|age| *age >= 0)
        .ok_or(RenderError::InvalidTime)?;
    let price = if age <= max_price_age_millis {
        fresh_price_line(offer).ok_or(RenderError::InvalidTime)?
    } else {
        "confira o preço atual".into()
    };
    let mut lines = vec![
        "#publi".into(),
        escape_html(&brand.disclosure_text),
        escape_html(&offer.title),
    ];
    lines.extend(
        offer
            .benefits
            .iter()
            .map(|benefit| format!("- {}", escape_html(benefit))),
    );
    lines.extend([
        escape_html(&price),
        format!("Curadoria: {}", escape_html(&brand.name)),
        escape_html(&offer.affiliate_url),
    ]);
    let text = lines.join("\n");
    let violations = lint_claims(
        &text,
        offer,
        ClaimContext {
            now_millis: now,
            max_price_age_millis,
            channel_program: offer.program,
            disclosure_text: &brand.disclosure_text,
        },
    );
    if !violations.is_empty() {
        return Err(RenderError::Claims(violations));
    }
    Ok(RenderedContent {
        text,
        brand_revision: brand.revision.clone(),
        template_id,
    })
}
