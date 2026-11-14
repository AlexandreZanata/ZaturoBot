//! Pure commerce rules. These contracts grant no account, model or publication authority.
//!
//! Time is supplied by callers in UTC milliseconds. Persistence and executable workflows
//! remain in their existing layers; this module performs no I/O.

pub mod attribution;
pub mod claims;
pub mod content;
pub mod kill_rules;
pub mod money;
pub mod offer;
pub mod policy;
pub mod slots;

pub use attribution::{AttributionError, AttributionMap};
pub use claims::{
    ClaimContext, ClaimViolation, DEFAULT_DISCLOSURE_TEXT, MAX_CONTENT_BYTES, lint_claims,
};
pub use content::{
    BrandText, ContentError, ContentState, OwnerContentResolution, OwnerResolutionOutcome,
    RenderError, RenderedContent, TemplateId, render,
};
pub use kill_rules::{
    KillAction, KillDecision, KillMetrics, KillReason, KillRuleError, KillRuleThresholds,
    KnownCounter, evaluate_kill_rules,
};
pub use money::{Currency, Money, MoneyError};
pub use offer::{AffiliateProgram, Offer, OfferInput, RejectReason};
pub use policy::{
    ActionMode, ActionModes, ChannelPolicy, PolicyCaps, PolicyDocument, PolicyError, PostingWindow,
    PriceBand,
};
pub use slots::{EditorialSlot, SlotDisposition, SlotError, SlotKey, slots_for_date};
