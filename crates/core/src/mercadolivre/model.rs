use super::*;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MercadoLivreProductState {
    Candidate,
    Approved,
    Stale,
    Withdrawn,
}

impl MercadoLivreProductState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Approved => "approved",
            Self::Stale => "stale",
            Self::Withdrawn => "withdrawn",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "candidate" => Self::Candidate,
            "approved" => Self::Approved,
            "stale" => Self::Stale,
            "withdrawn" => Self::Withdrawn,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelegramDeliveryStatus {
    /// Ingested offer waiting for the owner's per-offer publication approval.
    AwaitingApproval,
    /// Owner-approved offer that a publisher may claim.
    Queued,
    /// Claimed by a publisher before the external send; never re-queued automatically.
    Sending,
    Published,
    Skipped,
    Failed,
}

impl TelegramDeliveryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AwaitingApproval => "awaiting_approval",
            Self::Queued => "queued",
            Self::Sending => "sending",
            Self::Published => "published",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "awaiting_approval" => Self::AwaitingApproval,
            "queued" => Self::Queued,
            "sending" => Self::Sending,
            "published" => Self::Published,
            "skipped" => Self::Skipped,
            "failed" => Self::Failed,
            _ => return None,
        })
    }

    /// Terminal states never change again.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Published | Self::Skipped | Self::Failed)
    }

    /// Transitions a publisher acknowledgement may request. Approval (`AwaitingApproval ->
    /// Queued`) is an owner-only operation and is deliberately not an acknowledgement. A
    /// `Sending` delivery may only finish, so an uncertain send is never repeated.
    pub fn ack_transition_allowed(from: Self, to: Self) -> bool {
        matches!(
            (from, to),
            (Self::AwaitingApproval, Self::Skipped)
                | (Self::Queued, Self::Sending)
                | (Self::Queued, Self::Skipped)
                | (Self::Sending, Self::Published)
                | (Self::Sending, Self::Failed)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MercadoLivreSpec {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MercadoLivreVariant {
    #[serde(rename = "productId")]
    pub variant_product_id: String,
    pub sku: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voltage: Option<String>,
    #[serde(rename = "priceBrlCents")]
    pub price_brl_cents: i64,
    #[serde(default = "default_brl")]
    pub currency: String,
    pub availability: String,
    #[serde(
        rename = "returnDays",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub return_days: Option<u16>,
    #[serde(
        rename = "returnFees",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub return_fees: Option<String>,
    #[serde(
        rename = "canonicalUrl",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub canonical_url: Option<String>,
    #[serde(default)]
    pub highlights: Vec<String>,
}

fn default_brl() -> String {
    "BRL".into()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinioProductImage {
    pub position: u16,
    pub bucket: String,
    pub object_key: String,
    pub public_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_origin_url: Option<String>,
    pub content_type: String,
    pub sha256_hex: String,
    pub size_bytes: u64,
    pub alt_text: String,
}

impl MinioProductImage {
    pub fn validates(&self) -> Result<(), String> {
        if !valid_minio_bucket(&self.bucket) {
            return Err("invalid minio bucket name".into());
        }
        if !valid_minio_object_key(&self.object_key) {
            return Err("invalid minio object key".into());
        }
        if !validate_asset_public_url(&self.public_url) {
            return Err("invalid minio public_url".into());
        }
        if let Some(origin) = &self.source_origin_url
            && (!validate_asset_public_url(origin) || !no_local_path_leak(origin))
        {
            return Err("invalid image source_origin_url".into());
        }
        if !matches!(
            self.content_type.as_str(),
            "image/jpeg" | "image/png" | "image/webp"
        ) {
            return Err("unsupported image content_type".into());
        }
        if !crate::valid_sha256(&self.sha256_hex) {
            return Err("invalid image sha256".into());
        }
        if self.size_bytes == 0 || self.size_bytes > MAX_ML_IMAGE_BYTES {
            return Err("image size out of bounds".into());
        }
        if !crate::ordinary_text(&self.alt_text, 240) || !no_local_path_leak(&self.alt_text) {
            return Err("invalid image alt_text".into());
        }
        Ok(())
    }
}

/// Raw batch schema emitted by `tools/ytbot/commercial/mercadolivre/products.py`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MercadoLivreBotBatch {
    #[serde(default)]
    pub schema: Option<u32>,
    #[serde(default)]
    pub network: String,
    #[serde(default)]
    pub query: String,
    pub captured_at: String,
    #[serde(default)]
    pub count: usize,
    pub products: Vec<MercadoLivreBotProduct>,
}

/// Raw product entry produced by the Mercado Livre scraper. Local `photos` paths stay
/// strictly inside the worker boundary and are replaced by MinIO URLs before Hub ingestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MercadoLivreBotProduct {
    #[serde(default)]
    pub position: Option<u32>,
    #[serde(default)]
    pub product_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub product_url: Option<String>,
    #[serde(default)]
    pub canonical_url: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(default)]
    pub commission: Option<String>,
    #[serde(default)]
    pub sponsored: bool,
    #[serde(default)]
    pub card_text: Option<String>,
    #[serde(default)]
    pub share_link: Option<String>,
    #[serde(default)]
    pub share_id: Option<String>,
    #[serde(default)]
    pub share_text: Option<String>,
    #[serde(default)]
    pub price: Option<serde_json::Value>,
    #[serde(default)]
    pub meta_price: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub availability: Option<String>,
    #[serde(default)]
    pub price_valid_until: Option<String>,
    #[serde(default)]
    pub return_days: Option<u16>,
    #[serde(default)]
    pub seller: Option<String>,
    #[serde(default)]
    pub short_description: Option<String>,
    #[serde(default)]
    pub long_description: Option<String>,
    #[serde(default)]
    pub specs: Vec<MercadoLivreSpec>,
    #[serde(default)]
    pub variants: Vec<serde_json::Value>,
    #[serde(default)]
    pub gallery: Vec<String>,
    #[serde(default)]
    pub photos: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedCardText {
    pub card_badge: Option<String>,
    pub marketplace_rating: Option<String>,
    pub marketplace_sales_label: Option<String>,
    pub reference_price_cents: Option<i64>,
    pub card_price_cents: Option<i64>,
    pub discount_label: Option<String>,
    pub installment_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidatedMercadoLivreProduct {
    pub product_id: String,
    pub slug: String,
    pub title: String,
    pub brand: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub exact_variant: String,
    pub category_slug: String,
    pub canonical_url: String,
    pub seller_name: String,
    pub share_link: String,
    pub share_id: String,
    pub share_text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commission_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commission_bp: Option<u16>,
    pub sponsored: bool,
    pub price_brl_cents: i64,
    pub pix_price_cents: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_price_cents: Option<i64>,
    pub currency: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketplace_rating: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketplace_sales_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_badge: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installment_summary: Option<String>,
    pub payment_conditions: String,
    pub short_condition: String,
    pub freight_status: String,
    pub availability: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_valid_until: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_days: Option<u16>,
    pub short_description: String,
    pub long_description: String,
    pub one_line_reason: String,
    pub for_whom: String,
    pub supported_benefits: Vec<String>,
    pub limitations: Vec<String>,
    pub unknowns: Vec<String>,
    pub tags: Vec<String>,
    pub specs: Vec<MercadoLivreSpec>,
    pub variants: Vec<MercadoLivreVariant>,
    pub images: Vec<MinioProductImage>,
    pub observed_at_ms: i64,
    pub freshness_ttl_minutes: u32,
    pub evidence_sha256: String,
    pub state: MercadoLivreProductState,
}

impl ValidatedMercadoLivreProduct {
    pub fn validates(&self) -> Result<(), String> {
        if !validate_ml_product_id(&self.product_id) {
            return Err("invalid Mercado Livre product_id".into());
        }
        if !valid_slug(&self.slug) {
            return Err("invalid product slug".into());
        }
        if !crate::ordinary_text(&self.title, 280) || !no_local_path_leak(&self.title) {
            return Err("invalid product title".into());
        }
        if !crate::ordinary_text(&self.brand, 80) || !no_local_path_leak(&self.brand) {
            return Err("invalid brand".into());
        }
        if let Some(model) = &self.model
            && (!crate::ordinary_text(model, 80) || !no_local_path_leak(model))
        {
            return Err("invalid model".into());
        }
        if !crate::ordinary_text(&self.exact_variant, 280)
            || !no_local_path_leak(&self.exact_variant)
        {
            return Err("invalid exact_variant".into());
        }
        if !valid_slug(&self.category_slug) {
            return Err("invalid category_slug".into());
        }
        if !validate_ml_canonical_url(&self.canonical_url) {
            return Err("invalid Mercado Livre canonical_url".into());
        }
        if !crate::ordinary_text(&self.seller_name, 140) || !no_local_path_leak(&self.seller_name) {
            return Err("invalid seller_name".into());
        }
        if !validate_meli_share_link(&self.share_link) {
            return Err("invalid official meli.la share_link".into());
        }
        if !validate_ml_share_id(&self.share_id) {
            return Err("invalid Mercado Livre share_id".into());
        }
        if !crate::ordinary_text(&self.share_text, 600)
            || !no_local_path_leak(&self.share_text)
            || !self.share_text.contains(&self.share_link)
        {
            return Err("share_text must preserve official share_link cleanly".into());
        }
        if let Some(bp) = self.commission_bp
            && bp > 10_000
        {
            return Err("commission_bp out of bounds".into());
        }
        if self.currency != "BRL" || self.price_brl_cents <= 0 || self.pix_price_cents <= 0 {
            return Err("invalid BRL offer price".into());
        }
        if let Some(reference) = self.reference_price_cents
            && reference <= 0
        {
            return Err("invalid reference_price_cents".into());
        }
        if let Some(valid_until) = &self.price_valid_until
            && NaiveDate::parse_from_str(valid_until, "%Y-%m-%d").is_err()
        {
            return Err("price_valid_until must be YYYY-MM-DD".into());
        }
        for (field_name, value, limit) in [
            ("payment_conditions", &self.payment_conditions, 280),
            ("short_condition", &self.short_condition, 160),
            ("freight_status", &self.freight_status, 240),
            ("availability", &self.availability, 64),
            ("short_description", &self.short_description, 280),
            ("long_description", &self.long_description, 16_384),
            ("one_line_reason", &self.one_line_reason, 320),
            ("for_whom", &self.for_whom, 320),
        ] {
            if !crate::ordinary_text(value, limit) || !no_local_path_leak(value) {
                return Err(format!("invalid {field_name}"));
            }
        }
        if self.supported_benefits.is_empty()
            || self.supported_benefits.len() > 8
            || self.limitations.is_empty()
            || self.limitations.len() > 8
            || self.unknowns.len() > 8
            || self.tags.len() > 16
        {
            return Err("editorial list bounds exceeded".into());
        }
        for entry in self
            .supported_benefits
            .iter()
            .chain(self.limitations.iter())
            .chain(self.unknowns.iter())
        {
            if !crate::ordinary_text(entry, 320) || !no_local_path_leak(entry) {
                return Err("invalid editorial text item".into());
            }
        }
        for tag in &self.tags {
            if !valid_slug(tag) {
                return Err("invalid product tag slug".into());
            }
        }
        if self.specs.len() > MAX_ML_SPECS_PER_PRODUCT {
            return Err("too many product specs".into());
        }
        for spec in &self.specs {
            if !crate::ordinary_text(&spec.name, 120)
                || !crate::ordinary_text(&spec.value, 240)
                || !no_local_path_leak(&spec.value)
            {
                return Err("invalid product spec entry".into());
            }
        }
        if self.variants.len() > MAX_ML_VARIANTS_PER_PRODUCT {
            return Err("too many product variants".into());
        }
        for variant in &self.variants {
            if variant.price_brl_cents <= 0
                || variant.currency != "BRL"
                || !crate::ordinary_text(&variant.variant_product_id, 40)
                || !crate::ordinary_text(&variant.sku, 40)
                || !crate::ordinary_text(&variant.name, 280)
                || !no_local_path_leak(&variant.name)
            {
                return Err("invalid product variant".into());
            }
            if let Some(url) = &variant.canonical_url
                && !validate_ml_canonical_url(url)
            {
                return Err("invalid variant canonical_url".into());
            }
        }
        if self.images.is_empty() || self.images.len() > MAX_ML_IMAGES_PER_PRODUCT {
            return Err("at least one MinIO product image is required".into());
        }
        for image in &self.images {
            image.validates()?;
        }
        if self.observed_at_ms <= 0 || self.freshness_ttl_minutes == 0 {
            return Err("invalid observed_at_ms or freshness_ttl_minutes".into());
        }
        if !crate::valid_sha256(&self.evidence_sha256) {
            return Err("invalid evidence_sha256".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MercadoLivreIngestBatchRequest {
    pub worker_id: String,
    pub search_query: String,
    pub channel_id: String,
    pub captured_at_ms: i64,
    pub products: Vec<ValidatedMercadoLivreProduct>,
}

impl MercadoLivreIngestBatchRequest {
    pub fn validates(&self) -> Result<(), String> {
        crate::validate_identifier(&self.worker_id)?;
        if !crate::ordinary_text(&self.search_query, 160) || !no_local_path_leak(&self.search_query)
        {
            return Err("invalid search_query".into());
        }
        if !valid_telegram_channel(&self.channel_id) {
            return Err("invalid Telegram channel_id".into());
        }
        if self.captured_at_ms <= 0 {
            return Err("invalid captured_at_ms".into());
        }
        if self.products.is_empty() || self.products.len() > MAX_ML_BATCH_PRODUCTS {
            return Err("batch product count must be 1..=50".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for product in &self.products {
            product.validates()?;
            if !seen.insert(product.product_id.as_str()) {
                return Err("duplicate product_id in batch".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PriceTrendKind {
    #[default]
    New,
    Down,
    Up,
    Stable,
}

impl PriceTrendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Down => "down",
            Self::Up => "up",
            Self::Stable => "stable",
        }
    }

    pub fn from_prices(current_cents: i64, previous_cents: Option<i64>) -> (Self, i64, i16) {
        let Some(prev) = previous_cents.filter(|p| *p > 0) else {
            return (Self::New, 0, 0);
        };
        let delta = current_cents - prev;
        let pct = ((delta as f64 / prev as f64) * 100.0).round() as i16;
        let kind = if delta < 0 {
            Self::Down
        } else if delta > 0 {
            Self::Up
        } else {
            Self::Stable
        };
        (kind, delta, pct)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MercadoLivrePriceComparison {
    pub product_id: String,
    pub title: String,
    pub category_slug: String,
    pub subcategory_slug: String,
    pub current_price_cents: i64,
    pub pix_price_cents: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_price_cents: Option<i64>,
    pub price_delta_cents: i64,
    pub price_delta_pct: i16,
    pub lowest_price_30d_cents: i64,
    pub average_price_30d_cents: i64,
    pub highest_price_30d_cents: i64,
    pub observation_count: usize,
    pub trend: PriceTrendKind,
    pub telegram_enqueued: bool,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MercadoLivreIngestReport {
    pub batch_id: String,
    pub worker_id: String,
    pub ingested_at_ms: i64,
    pub product_count: usize,
    pub image_count: usize,
    pub telegram_queued_count: usize,
    /// Deliveries created by this batch that still need per-offer owner approval.
    #[serde(default)]
    pub telegram_awaiting_approval_count: usize,
    pub product_ids: Vec<String>,
    pub payload_sha256: String,
    #[serde(default)]
    pub new_product_count: usize,
    #[serde(default)]
    pub updated_product_count: usize,
    #[serde(default)]
    pub price_drop_count: usize,
    #[serde(default)]
    pub price_increase_count: usize,
    #[serde(default)]
    pub unchanged_price_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comparisons: Vec<MercadoLivrePriceComparison>,
}
