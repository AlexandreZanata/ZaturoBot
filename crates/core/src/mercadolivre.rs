//! Mercado Livre affiliate product contracts, card/JSON-LD parsers, MinIO media metadata,
//! and static site / Telegram queue projections. Pure validation only; no network or disk I/O.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

pub const MAX_ML_BATCH_PRODUCTS: usize = 50;
pub const MAX_ML_SPECS_PER_PRODUCT: usize = 64;
pub const MAX_ML_VARIANTS_PER_PRODUCT: usize = 16;
pub const MAX_ML_IMAGES_PER_PRODUCT: usize = 16;
pub const MAX_ML_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
pub const DEFAULT_ML_FRESHNESS_TTL_MINUTES: u32 = 30;
pub const AFFILIATE_DISCLOSURE_PT_BR: &str =
    "Publicidade · Link de afiliado: podemos receber comissão se você comprar.";

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
    Queued,
    Published,
    Skipped,
    Failed,
}

impl TelegramDeliveryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Published => "published",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "queued" => Self::Queued,
            "published" => Self::Published,
            "skipped" => Self::Skipped,
            "failed" => Self::Failed,
            _ => return None,
        })
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MercadoLivreIngestReport {
    pub batch_id: String,
    pub worker_id: String,
    pub ingested_at_ms: i64,
    pub product_count: usize,
    pub image_count: usize,
    pub telegram_queued_count: usize,
    pub product_ids: Vec<String>,
    pub payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteCatalogCategory {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub editable_by_owner: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intended_budget_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteCatalogGalleryItem {
    pub path: String,
    pub alt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteCatalogPriceObservation {
    pub id: String,
    pub offer_id: String,
    pub variant_id: String,
    pub seller_name: String,
    pub payment_basis: String,
    pub freight_note: String,
    pub price_brl_cents: i64,
    pub observed_at: String,
    pub evidence_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteCatalogEvidenceSource {
    pub id: String,
    pub source_type: String,
    pub title: String,
    pub publisher: String,
    pub source_url: String,
    pub retrieved_at: String,
    pub verified_facts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteCatalogMediaRight {
    pub id: String,
    pub asset_description: String,
    pub license_type: String,
    pub rights_verified: bool,
    pub owner_permitted_use: String,
    pub attribution_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PriceLevelKind {
    LowestPrice,
    GoodPrice,
    #[default]
    TypicalPrice,
}

impl PriceLevelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LowestPrice => "lowest_price",
            Self::GoodPrice => "good_price",
            Self::TypicalPrice => "typical_price",
        }
    }

    pub fn label_pt_br(self) -> &'static str {
        match self {
            Self::LowestPrice => "Menor preço",
            Self::GoodPrice => "Bom preço",
            Self::TypicalPrice => "Preço na média",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PriceIntelligence {
    pub window_days: u16,
    pub current_price_cents: i64,
    pub lowest_price_30d_cents: i64,
    pub average_price_30d_cents: i64,
    pub highest_price_30d_cents: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_price_cents: Option<i64>,
    pub discount_pct: u8,
    pub savings_vs_reference_cents: i64,
    pub savings_vs_average_cents: i64,
    pub price_level: PriceLevelKind,
    pub price_level_label: String,
    pub is_price_drop: bool,
    pub verified_at_display: String,
    pub svg_sparkline: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductFaqItem {
    pub question: String,
    pub answer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubcategoryMetadata {
    pub slug: String,
    pub name: String,
    pub parent_category_slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FacetBucketCount {
    pub key: String,
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryTaxonomyNode {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub offer_count: usize,
    pub subcategories: Vec<FacetBucketCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordClusterNode {
    pub slug: String,
    pub label: String,
    pub offer_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogTaxonomyResponse {
    pub generated_at: String,
    pub cache_generation: u64,
    pub categories: Vec<CategoryTaxonomyNode>,
    pub keyword_clusters: Vec<KeywordClusterNode>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSearchQuery {
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub categoria: Option<String>,
    #[serde(default)]
    pub subcategoria: Option<String>,
    #[serde(default)]
    pub loja: Option<String>,
    #[serde(default)]
    pub ordem: Option<String>,
    #[serde(default)]
    pub desconto: Option<u8>,
    #[serde(default)]
    pub preco: Option<String>,
    #[serde(default)]
    pub avaliacao: Option<String>,
    #[serde(default)]
    pub colecao: Option<String>,
    #[serde(default)]
    pub pagina: Option<u32>,
    #[serde(default, alias = "por_pagina")]
    pub por_pagina: Option<u32>,
    #[serde(default)]
    pub formato: Option<String>,
}

impl CatalogSearchQuery {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(q) = &self.q
            && (q.chars().count() > 120 || q.contains('\0'))
        {
            return Err("invalid search query length".into());
        }
        for slug in [
            &self.categoria,
            &self.subcategoria,
            &self.loja,
            &self.colecao,
        ]
        .into_iter()
        .flatten()
        {
            let trimmed = slug.trim();
            if trimmed.len() > 80 || trimmed.contains('\0') {
                return Err("invalid taxonomy filter".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogFacetsSummary {
    pub categories: Vec<FacetBucketCount>,
    pub subcategories: Vec<FacetBucketCount>,
    pub stores: Vec<FacetBucketCount>,
    pub price_ranges: Vec<FacetBucketCount>,
    pub discount_tiers: Vec<FacetBucketCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSearchResponse {
    pub total_matches: usize,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
    pub cache_generation: u64,
    pub facets: CatalogFacetsSummary,
    pub items: Vec<SiteCatalogOffer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_fragment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreadcrumbItem {
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductDetailResponse {
    pub cache_generation: u64,
    pub offer: SiteCatalogOffer,
    pub breadcrumb: Vec<BreadcrumbItem>,
    pub similar_offers: Vec<SiteCatalogOffer>,
    pub alert_count: usize,
    pub impressions: u64,
    pub clicks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExposureEventItem {
    pub product_id: String,
    pub store: String,
    pub pos: u32,
    #[serde(default)]
    pub discount_pct: u8,
    #[serde(default)]
    pub price_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExposureBeaconRequest {
    pub route: String,
    pub events: Vec<ExposureEventItem>,
    pub timestamp_ms: i64,
}

impl ExposureBeaconRequest {
    pub fn validates(&self) -> Result<(), String> {
        if !crate::ordinary_text(&self.route, 160) || !no_local_path_leak(&self.route) {
            return Err("invalid exposure route".into());
        }
        if self.events.is_empty() || self.events.len() > 60 {
            return Err("exposure events count must be 1..=60".into());
        }
        for ev in &self.events {
            if !validate_ml_product_id(&ev.product_id) || !crate::ordinary_text(&ev.store, 48) {
                return Err("invalid exposure event item".into());
            }
        }
        if self.timestamp_ms <= 0 {
            return Err("invalid exposure timestamp_ms".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExposureBeaconResponse {
    pub accepted: usize,
    pub route: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PriceAlertActionRequest {
    pub action: String,
    pub subscriber_id: String,
    #[serde(default)]
    pub product_id: Option<String>,
    #[serde(default)]
    pub target_price_cents: Option<i64>,
}

impl PriceAlertActionRequest {
    pub fn validates(&self) -> Result<(), String> {
        if !matches!(
            self.action.as_str(),
            "criar-alerta" | "remover-alerta" | "listar-alertas"
        ) {
            return Err("unsupported price alert action".into());
        }
        if self.subscriber_id.is_empty()
            || self.subscriber_id.len() > 80
            || !self
                .subscriber_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        {
            return Err("invalid subscriber_id".into());
        }
        if let Some(pid) = &self.product_id
            && !validate_ml_product_id(pid)
        {
            return Err("invalid alert product_id".into());
        }
        if matches!(self.action.as_str(), "criar-alerta" | "remover-alerta")
            && self.product_id.is_none()
        {
            return Err("product_id is required for criar-alerta / remover-alerta".into());
        }
        if self.action == "criar-alerta" {
            let Some(target) = self.target_price_cents else {
                return Err("target_price_cents is required for criar-alerta".into());
            };
            if target <= 0 {
                return Err("target_price_cents must be positive".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceAlertSubscription {
    pub subscriber_id: String,
    pub product_id: String,
    pub target_price_cents: i64,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TriggeredPriceAlert {
    pub subscriber_id: String,
    pub product_id: String,
    pub title: String,
    pub target_price_cents: i64,
    pub current_price_cents: i64,
    pub share_link: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceAlertResponse {
    pub action: String,
    pub subscriber_id: String,
    pub active_alerts: Vec<PriceAlertSubscription>,
    #[serde(default)]
    pub triggered: Vec<TriggeredPriceAlert>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteCatalogOffer {
    pub id: String,
    pub slug: String,
    pub is_demo_fixture: bool,
    pub product_id: String,
    pub brand: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub title: String,
    pub exact_variant: String,
    pub category_slug: String,
    #[serde(default)]
    pub subcategory_slug: String,
    #[serde(default)]
    pub subcategory_name: String,
    pub merchant: String,
    pub marketplace_label: String,
    pub seller_name: String,
    pub store_short: String,
    pub program: String,
    pub official_affiliate_url: String,
    pub canonical_url: String,
    pub share_id: String,
    pub share_text: String,
    pub sponsored: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commission_rate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketplace_rating: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rating_milli: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marketplace_sales_label: Option<String>,
    #[serde(default)]
    pub sales_volume_estimate: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_badge: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_percent_label: Option<String>,
    #[serde(default)]
    pub discount_pct: u8,
    #[serde(default)]
    pub trust_badges: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installment_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub return_days: Option<u16>,
    pub availability: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_valid_until: Option<String>,
    pub image_path: String,
    pub image_alt: String,
    pub gallery_paths: Vec<SiteCatalogGalleryItem>,
    pub price_brl_cents: i64,
    pub pix_price_cents: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_price_cents: Option<i64>,
    pub currency: String,
    pub payment_conditions: String,
    pub short_condition: String,
    pub short_use_line: String,
    pub short_checked_at: String,
    pub cta_dest_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coupon_condition: Option<String>,
    pub freight_status: String,
    pub observed_at: String,
    pub observed_at_display: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_until_display: Option<String>,
    pub freshness_ttl_minutes: u32,
    pub evidence_refs: Vec<String>,
    pub media_rights_ref: String,
    pub state: String,
    pub approval_revision: String,
    pub one_line_reason: String,
    pub for_whom_it_makes_sense: String,
    pub supported_benefits: Vec<String>,
    pub limitations: Vec<String>,
    pub unknowns: Vec<String>,
    pub tags: Vec<String>,
    pub has_verified_seven_day_sample: bool,
    pub seven_day_coverage_note: String,
    pub price_observations: Vec<SiteCatalogPriceObservation>,
    pub specs: Vec<MercadoLivreSpec>,
    pub variants: Vec<MercadoLivreVariant>,
    pub long_description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_intelligence: Option<PriceIntelligence>,
    #[serde(default)]
    pub dynamic_faq: Vec<ProductFaqItem>,
    #[serde(default)]
    pub similar_offer_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SiteCatalogSnapshot {
    pub generated_at: String,
    pub categories: Vec<SiteCatalogCategory>,
    pub offers: Vec<SiteCatalogOffer>,
    pub evidence_sources: Vec<SiteCatalogEvidenceSource>,
    pub media_rights: Vec<SiteCatalogMediaRight>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelegramOfferQueueItem {
    pub delivery_id: String,
    pub product_id: String,
    pub offer_id: String,
    pub channel_id: String,
    pub title: String,
    pub brand: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub price_brl_cents: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_price_cents: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_label: Option<String>,
    pub share_link: String,
    pub share_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_public_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_object_key: Option<String>,
    pub message_text: String,
    pub status: TelegramDeliveryStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_message_id: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub published_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelegramDeliveryAck {
    pub status: TelegramDeliveryStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

impl TelegramDeliveryAck {
    pub fn validates(&self) -> Result<(), String> {
        if let Some(id) = &self.external_message_id
            && !crate::ordinary_text(id, 64)
        {
            return Err("invalid external_message_id".into());
        }
        if let Some(code) = &self.error_code
            && !crate::ordinary_text(code, 64)
        {
            return Err("invalid error_code".into());
        }
        Ok(())
    }
}

pub fn format_utc_sigv4(unix_seconds: i64) -> Result<(String, String), String> {
    let dt = DateTime::<Utc>::from_timestamp(unix_seconds, 0)
        .ok_or_else(|| "invalid UTC timestamp".to_owned())?;
    Ok((
        dt.format("%Y%m%dT%H%M%SZ").to_string(),
        dt.format("%Y%m%d").to_string(),
    ))
}

pub fn format_utc_iso_ms(ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(ms)
        .unwrap_or_default()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

pub fn parse_iso_to_ms(value: &str) -> Result<i64, String> {
    let trimmed = value.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.timestamp_millis());
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S") {
        return Ok(naive.and_utc().timestamp_millis());
    }
    Err("expected ISO-8601 timestamp".into())
}

pub fn format_cuiaba_short(ms: i64) -> String {
    let offset = FixedOffset::west_opt(4 * 3600).expect("valid UTC-4 offset");
    let dt = DateTime::<Utc>::from_timestamp_millis(ms)
        .unwrap_or_default()
        .with_timezone(&offset);
    dt.format("%d/%m às %Hh%M").to_string()
}

pub fn format_cuiaba_display(ms: i64) -> String {
    let offset = FixedOffset::west_opt(4 * 3600).expect("valid UTC-4 offset");
    let dt = DateTime::<Utc>::from_timestamp_millis(ms)
        .unwrap_or_default()
        .with_timezone(&offset);
    dt.format("%d/%m/%Y às %H:%M (horário de Cuiabá)")
        .to_string()
}

pub fn format_brl_cents(cents: i64) -> String {
    let safe = cents.max(0);
    let reais = safe / 100;
    let rem = safe % 100;
    let whole_str = reais.to_string();
    let mut grouped = String::with_capacity(whole_str.len() + whole_str.len() / 3);
    for (idx, ch) in whole_str.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    let formatted_whole: String = grouped.chars().rev().collect();
    format!("R$ {formatted_whole},{rem:02}")
}

pub fn validate_ml_product_id(value: &str) -> bool {
    value.strip_prefix("MLB").is_some_and(|digits| {
        (6..=16).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit())
    })
}

pub fn validate_meli_share_link(value: &str) -> bool {
    let Some(slug) = value.strip_prefix("https://meli.la/") else {
        return false;
    };
    !slug.is_empty()
        && slug.len() <= 32
        && slug
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

pub fn validate_ml_canonical_url(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    if rest.is_empty()
        || value.len() > 1024
        || !no_local_path_leak(value)
        || value
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '@' | '?' | '#' | '\\'))
    {
        return false;
    }
    let host = rest.split('/').next().unwrap_or("");
    matches!(
        host,
        "www.mercadolivre.com.br" | "mercadolivre.com.br" | "produto.mercadolivre.com.br"
    )
}

pub fn validate_ml_share_id(value: &str) -> bool {
    (4..=32).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
}

pub fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn valid_minio_bucket(value: &str) -> bool {
    (3..=63).contains(&value.len())
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn valid_minio_object_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.starts_with('/')
        && !value.contains("..")
        && !value.contains('\\')
        && no_local_path_leak(value)
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.'))
}

pub fn validate_asset_public_url(value: &str) -> bool {
    if value.len() > 1024
        || !no_local_path_leak(value)
        || value
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '@' | '?' | '#' | '\\'))
    {
        return false;
    }
    if let Some(rest) = value.strip_prefix("https://") {
        let host = rest.split(['/', ':']).next().unwrap_or("");
        return !host.is_empty() && rest.contains('/');
    }
    if let Some(rest) = value.strip_prefix("http://") {
        return (rest.starts_with("127.0.0.1") || rest.starts_with("[::1]")) && rest.contains('/');
    }
    false
}

pub fn valid_telegram_channel(value: &str) -> bool {
    if let Some(handle) = value.strip_prefix('@') {
        return (5..=32).contains(&handle.len())
            && handle
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_');
    }
    if let Some(id) = value.strip_prefix('-') {
        return (6..=23).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit());
    }
    false
}

pub fn no_local_path_leak(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    !value.contains('\0')
        && !lower.contains(".local/")
        && !lower.contains("/home/")
        && !lower.contains("file://")
        && !lower.contains("javascript:")
        && !lower.contains(".secrets")
        && !lower.contains("matt_tool=")
}

pub fn slugify_pt_br(title: &str, product_id: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = true;
    for ch in title.chars() {
        let mapped = match ch {
            'a'..='z' | '0'..='9' => Some(ch),
            'A'..='Z' => Some(ch.to_ascii_lowercase()),
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => Some('a'),
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => Some('e'),
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => Some('i'),
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => Some('o'),
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => Some('u'),
            'ç' | 'Ç' => Some('c'),
            _ => None,
        };
        if let Some(c) = mapped {
            if slug.len() < 72 {
                slug.push(c);
                prev_dash = false;
            }
        } else if !prev_dash && slug.len() < 72 {
            slug.push('-');
            prev_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    let suffix = product_id.to_ascii_lowercase();
    if slug.is_empty() {
        suffix
    } else if slug.ends_with(&suffix) {
        slug
    } else {
        format!("{slug}-{suffix}")
    }
}

pub fn extract_product_id_from_url(url: &str) -> Option<String> {
    for part in url.split(['/', '?', '#']) {
        if validate_ml_product_id(part) {
            return Some(part.to_owned());
        }
    }
    None
}

pub fn parse_commission_bp(value: &str) -> Option<u16> {
    let clean = value.trim().trim_end_matches('%').trim().replace(',', ".");
    if clean.is_empty() {
        return None;
    }
    let cents = parse_decimal_to_hundredths(&clean)?;
    u16::try_from(cents).ok().filter(|bp| *bp <= 10_000)
}

pub fn parse_price_value_cents(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(num) => {
            if let Some(whole) = num.as_i64() {
                return whole.checked_mul(100);
            }
            parse_decimal_to_hundredths(&num.to_string())
        }
        serde_json::Value::String(s) => parse_brl_amount_cents(s),
        _ => None,
    }
}

pub fn parse_brl_amount_cents(raw: &str) -> Option<i64> {
    let trimmed = raw
        .trim()
        .trim_start_matches("R$")
        .trim()
        .replace('\u{a0}', "");
    if trimmed.is_empty() {
        return None;
    }
    let normalized = if trimmed.contains(',') {
        trimmed.replace('.', "").replace(',', ".")
    } else if let Some((left, right)) = trimmed.split_once('.') {
        // Distinguish "1.379" (thousands) from "699.90" (cents)
        if right.len() == 3 && left.chars().all(|c| c.is_ascii_digit()) {
            format!("{left}{right}")
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    parse_decimal_to_hundredths(&normalized)
}

fn parse_decimal_to_hundredths(raw: &str) -> Option<i64> {
    let s = raw.trim();
    if s.is_empty() || s.starts_with('-') {
        return None;
    }
    let (whole, frac) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        None => (s, ""),
    };
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
        || frac.len() > 2
    {
        return None;
    }
    let whole_val: i64 = whole.parse().ok()?;
    let frac_val: i64 = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()? * 10,
        2 => frac.parse().ok()?,
        _ => return None,
    };
    whole_val.checked_mul(100)?.checked_add(frac_val)
}

/// Parses raw Mercado Livre search-result card text into structured badges, social proof,
/// price points, discount percent, and installment summary.
pub fn parse_card_text(card_text: &str) -> ParsedCardText {
    let mut parsed = ParsedCardText::default();
    let lines: Vec<&str> = card_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    for line in &lines {
        let upper = line.to_ascii_uppercase();
        if parsed.card_badge.is_none() {
            if upper.contains("MAIS VENDIDO") {
                parsed.card_badge = Some("Mais vendido".into());
            } else if upper.contains("OFERTA DO DIA") {
                parsed.card_badge = Some("Oferta do dia".into());
            } else if upper.contains("MAIS PROCURADO") {
                parsed.card_badge = Some("Mais procurado".into());
            }
        }
        if parsed.marketplace_rating.is_none()
            && line.contains('|')
            && line.to_lowercase().contains("vendid")
        {
            let (left, right) = line.split_once('|').unwrap_or(("", ""));
            let rating_clean = left.trim().replace('.', ",");
            if rating_clean.chars().any(|c| c.is_ascii_digit()) {
                parsed.marketplace_rating = Some(format!("{rating_clean} / 5"));
            }
            let sales = normalize_sales_label(right.trim());
            if !sales.is_empty() {
                parsed.marketplace_sales_label = Some(sales);
            }
        }
        if parsed.discount_label.is_none() && upper.contains("% OFF") {
            parsed.discount_label = Some(line.trim().to_owned());
        }
    }

    // Reconstruct both split ("R$\n699\n,\n90") and inline ("R$ 699,90") price tokens.
    let mut amounts_cents: Vec<i64> = Vec::new();
    let mut idx = 0;
    while idx < lines.len() {
        let line = lines[idx];
        let prev_is_installment = idx > 0
            && (lines[idx - 1].to_ascii_lowercase().ends_with('x')
                || lines[idx - 1].to_ascii_lowercase().ends_with('×'));
        if line == "R$" && idx + 1 < lines.len() {
            let mut amount = lines[idx + 1].to_owned();
            let mut consumed = 2;
            if idx + 3 < lines.len() && lines[idx + 2] == "," {
                amount.push(',');
                amount.push_str(lines[idx + 3]);
                consumed = 4;
            } else if idx + 2 < lines.len() && lines[idx + 2].starts_with(',') {
                amount.push_str(lines[idx + 2]);
                consumed = 3;
            }
            if let Some(cents) = parse_brl_amount_cents(&amount) {
                if !prev_is_installment {
                    amounts_cents.push(cents);
                } else if parsed.installment_summary.is_none() {
                    let count_tok = lines[idx - 1].replace('x', "×");
                    let has_sem_juros = lines[idx + consumed..lines.len().min(idx + consumed + 3)]
                        .iter()
                        .any(|l| l.to_ascii_lowercase().contains("sem juros"));
                    let suffix = if has_sem_juros { " sem juros" } else { "" };
                    parsed.installment_summary = Some(format!(
                        "{count_tok} de {}{suffix}",
                        format_brl_cents(cents)
                    ));
                }
            }
            idx += consumed;
            continue;
        }
        if let Some(pos) = line.find("R$") {
            let after = line[pos + 2..].trim();
            let num_token: String = after
                .chars()
                .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | ','))
                .collect();
            if let Some(cents) = parse_brl_amount_cents(&num_token) {
                let lower = line.to_ascii_lowercase();
                if let Some((prefix, _)) = lower.split_once("r$")
                    && (prefix.contains("x ") || prefix.contains("× "))
                {
                    if parsed.installment_summary.is_none() {
                        parsed.installment_summary = Some(line.trim().to_owned());
                    }
                } else {
                    amounts_cents.push(cents);
                }
            }
        }
        idx += 1;
    }

    if amounts_cents.len() >= 2 && amounts_cents[0] > amounts_cents[1] {
        parsed.reference_price_cents = Some(amounts_cents[0]);
        parsed.card_price_cents = Some(amounts_cents[1]);
    } else if let Some(first) = amounts_cents.first() {
        parsed.card_price_cents = Some(*first);
    }

    parsed
}

fn normalize_sales_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let with_space = trimmed
        .replace("mil ", " mil ")
        .replace("milvend", " mil vend");
    if with_space.to_ascii_lowercase().contains("no ml") {
        with_space
    } else {
        format!("{with_space} no ML")
    }
}

pub fn parse_jsonld_variant(raw: &serde_json::Value) -> Option<MercadoLivreVariant> {
    let obj = raw.as_object()?;
    let name = obj.get("name")?.as_str()?.trim().to_owned();
    let sku = obj
        .get("sku")
        .or_else(|| obj.get("productID"))
        .and_then(|v| v.as_str())?
        .trim()
        .to_owned();
    let variant_product_id = obj
        .get("productID")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| sku.clone());
    let color = obj
        .get("color")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());

    let offers = obj.get("offers").and_then(|v| v.as_object());
    let price_brl_cents = offers
        .and_then(|o| o.get("price"))
        .and_then(parse_price_value_cents)
        .filter(|c| *c > 0)?;
    let currency = offers
        .and_then(|o| o.get("priceCurrency"))
        .and_then(|v| v.as_str())
        .unwrap_or("BRL")
        .to_owned();
    let availability = offers
        .and_then(|o| o.get("availability"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().rsplit('/').next().unwrap_or("InStock").to_owned())
        .unwrap_or_else(|| "InStock".into());
    let policy = offers
        .and_then(|o| o.get("hasMerchantReturnPolicy"))
        .and_then(|v| v.as_object());
    let return_days = policy
        .and_then(|p| p.get("merchantReturnDays"))
        .and_then(|v| v.as_u64())
        .and_then(|d| u16::try_from(d).ok());
    let return_fees = policy
        .and_then(|p| p.get("returnFees"))
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.contains("FreeReturn") {
                "Grátis".into()
            } else {
                s.rsplit('/')
                    .next()
                    .unwrap_or("Conforme anúncio")
                    .to_owned()
            }
        });
    let canonical_url = offers
        .and_then(|o| o.get("url"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .filter(|u| validate_ml_canonical_url(u));

    let mut voltage: Option<String> = None;
    let mut highlights = Vec::new();
    if let Some(props) = obj.get("additionalProperty").and_then(|v| v.as_array()) {
        for prop in props {
            let Some(p) = prop.as_object() else { continue };
            let pname = p.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
            let pval = p.get("value").and_then(|v| v.as_str()).unwrap_or("").trim();
            if pname.is_empty() && pval.is_empty() {
                continue;
            }
            if pname.eq_ignore_ascii_case("Voltagem") && !pval.is_empty() {
                voltage = Some(pval.to_owned());
            }
            if !pval.is_empty() && highlights.len() < 4 {
                highlights.push(pval.to_owned());
            }
        }
    }

    Some(MercadoLivreVariant {
        variant_product_id,
        sku,
        name,
        color,
        voltage,
        price_brl_cents,
        currency,
        availability,
        return_days,
        return_fees,
        canonical_url,
        highlights,
    })
}

pub const TELEGRAM_CAPTION_MAX_CHARS: usize = 1024;
pub const TELEGRAM_MESSAGE_MAX_CHARS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramInlineButton {
    pub text: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramInlineKeyboardMarkup {
    pub inline_keyboard: Vec<Vec<TelegramInlineButton>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelegramBotCommand {
    Start,
    Help,
    Offers,
    Hardware,
    Search(String),
    WhatsappDigest,
}

impl TelegramBotCommand {
    pub fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        if !trimmed.starts_with('/') {
            return None;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let cmd_raw = parts.next()?.trim();
        let arg = parts.next().map(str::trim).unwrap_or("");
        let cmd = cmd_raw.split('@').next().unwrap_or("").to_ascii_lowercase();
        match cmd.as_str() {
            "/start" => Some(Self::Start),
            "/ajuda" | "/help" => Some(Self::Help),
            "/ofertas" | "/achados" => Some(Self::Offers),
            "/hardware" => Some(Self::Hardware),
            "/buscar" | "/procurar" => {
                if arg.is_empty() {
                    None
                } else {
                    Some(Self::Search(truncate_chars(arg, 80)))
                }
            }
            "/whatsapp" => Some(Self::WhatsappDigest),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsappAssistedDraft {
    pub delivery_id: String,
    pub product_id: String,
    pub slug: String,
    pub title: String,
    pub price_brl_cents: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_price_cents: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_label: Option<String>,
    pub share_link: String,
    pub share_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_public_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site_offer_url: Option<String>,
    pub whatsapp_text: String,
    pub operator_checklist: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsappAssistedPackage {
    pub generated_at: String,
    pub mode: String,
    pub channel_id: String,
    pub draft_count: usize,
    pub combined_digest_text: String,
    pub drafts: Vec<WhatsappAssistedDraft>,
}

pub fn escape_telegram_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

fn truncate_chars(input: &str, max_chars: usize) -> String {
    let trimmed = input.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_owned();
    }
    let keep = max_chars.saturating_sub(3);
    let mut out: String = trimmed.chars().take(keep).collect();
    out.push_str("...");
    out
}

fn rendered_html_char_count(html: &str) -> usize {
    let mut count = 0usize;
    let mut chars = html.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            for inner in chars.by_ref() {
                if inner == '>' {
                    break;
                }
            }
        } else if ch == '&' {
            for inner in chars.by_ref() {
                if inner == ';' {
                    break;
                }
            }
            count += 1;
        } else {
            count += 1;
        }
    }
    count
}

pub fn build_site_offer_url(
    site_base_url: Option<&str>,
    title: &str,
    product_id: &str,
) -> Option<String> {
    let base = site_base_url?.trim().trim_end_matches('/');
    if !(base.starts_with("https://") || base.starts_with("http://")) {
        return None;
    }
    let slug = slugify_pt_br(title, product_id);
    Some(format!("{base}/ofertas/{slug}/index.html"))
}

pub fn build_telegram_offer_keyboard(
    item: &TelegramOfferQueueItem,
    site_base_url: Option<&str>,
) -> TelegramInlineKeyboardMarkup {
    let mut rows = vec![vec![TelegramInlineButton {
        text: "🔥 PEGAR OFERTA NO MERCADO LIVRE 🛒".to_owned(),
        url: item.share_link.clone(),
    }]];
    if let Some(offer_url) = build_site_offer_url(site_base_url, &item.title, &item.product_id) {
        rows.push(vec![TelegramInlineButton {
            text: "📋 Ver Ficha Completa no Procurali".to_owned(),
            url: offer_url,
        }]);
    }
    TelegramInlineKeyboardMarkup {
        inline_keyboard: rows,
    }
}

pub fn format_telegram_html_caption_pt_br(
    item: &TelegramOfferQueueItem,
    site_base_url: Option<&str>,
) -> String {
    let render = |title_limit: usize, include_extra: bool| {
        let mut lines = Vec::new();
        lines.push("🚨 <b>OFERTA RELÂMPAGO · ACHADO NO MERCADO LIVRE!</b> 🔥".to_owned());
        lines.push(String::new());
        lines.push(format!(
            "📦 <b>{}</b>",
            escape_telegram_html(&truncate_chars(&item.title, title_limit))
        ));

        let mut id_parts = vec![format!(
            "🏷️ <b>Marca:</b> {}",
            escape_telegram_html(&item.brand)
        )];
        if let Some(model) = &item.model {
            id_parts.push(format!("<b>Modelo:</b> {}", escape_telegram_html(model)));
        }
        id_parts.push(format!(
            "<b>Cód. ML:</b> <code>{}</code>",
            escape_telegram_html(&item.product_id)
        ));
        lines.push(id_parts.join(" · "));
        lines.push(String::new());

        let price_str = format_brl_cents(item.price_brl_cents);
        match item.reference_price_cents {
            Some(ref_cents) if ref_cents > item.price_brl_cents => {
                let savings_cents = ref_cents - item.price_brl_cents;
                let savings_str = format_brl_cents(savings_cents);
                let computed_pct =
                    ((savings_cents as f64 / ref_cents as f64) * 100.0).round() as u32;
                let badge = item
                    .discount_label
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| format!("{computed_pct}% OFF"));
                lines.push(format!(
                    "❌ De: <s>{}</s>",
                    escape_telegram_html(&format_brl_cents(ref_cents))
                ));
                lines.push(format!(
                    "✅ <b>POR APENAS: {}</b> 🔥 <b>({} · Economia de {}!)</b>",
                    escape_telegram_html(&price_str),
                    escape_telegram_html(&badge),
                    escape_telegram_html(&savings_str),
                ));
            }
            _ => {
                if let Some(discount) = item
                    .discount_label
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    lines.push(format!(
                        "✅ <b>PREÇO EM OFERTA: {}</b> 🔥 <b>({})</b>",
                        escape_telegram_html(&price_str),
                        escape_telegram_html(discount),
                    ));
                } else {
                    lines.push(format!(
                        "✅ <b>PREÇO EM OFERTA: {}</b> 🔥",
                        escape_telegram_html(&price_str),
                    ));
                }
            }
        }

        if include_extra {
            for extra in item.message_text.lines().filter(|line| {
                let t = line.trim();
                t.starts_with("💳") || t.starts_with("🏪") || t.starts_with("⚠️")
            }) {
                lines.push(escape_telegram_html(&truncate_chars(extra.trim(), 140)));
            }
        }

        lines.push(format!(
            "🕐 Atualizado em {}",
            escape_telegram_html(&format_cuiaba_display(item.updated_at_ms))
        ));
        lines.push(String::new());
        lines.push(
            "⏳ <b>CORRA! Preço e estoque promocional sujeitos a alteração a qualquer momento!</b>"
                .to_owned(),
        );
        lines.push(format!(
            "👉 <b>GARANTA O SEU AQUI:</b> <a href=\"{}\">Clique para abrir a oferta oficial no Mercado Livre</a>",
            escape_telegram_html(&item.share_link)
        ));
        lines.push(format!(
            "🔍 <b>Código rápido no app ML:</b> <code>{}</code>",
            escape_telegram_html(&item.share_id)
        ));
        if let Some(offer_url) = build_site_offer_url(site_base_url, &item.title, &item.product_id)
        {
            lines.push(format!(
                "📋 <a href=\"{}\">Ver ficha técnica e histórico no Procurali</a>",
                escape_telegram_html(&offer_url)
            ));
        }
        lines.push(format!(
            "<i>{}</i>",
            escape_telegram_html(AFFILIATE_DISCLOSURE_PT_BR)
        ));
        lines.join("\n")
    };

    for (title_limit, include_extra) in [
        (160usize, true),
        (110usize, true),
        (110usize, false),
        (75usize, false),
    ] {
        let candidate = render(title_limit, include_extra);
        if rendered_html_char_count(&candidate) <= TELEGRAM_CAPTION_MAX_CHARS {
            return candidate;
        }
    }
    render(55, false)
}

fn sanitize_whatsapp_inline(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(['*', '_', '~', '`'], "")
        .trim()
        .to_owned()
}

pub fn build_whatsapp_assisted_draft(
    item: &TelegramOfferQueueItem,
    site_base_url: Option<&str>,
) -> WhatsappAssistedDraft {
    let slug = slugify_pt_br(&item.title, &item.product_id);
    let site_offer_url = build_site_offer_url(site_base_url, &item.title, &item.product_id);
    let safe_title = sanitize_whatsapp_inline(&item.title);
    let safe_brand = sanitize_whatsapp_inline(&item.brand);
    let price_str = format_brl_cents(item.price_brl_cents);

    let price_line = match (item.reference_price_cents, item.discount_label.as_deref()) {
        (Some(ref_cents), Some(discount)) if ref_cents > item.price_brl_cents => {
            let safe_disc = sanitize_whatsapp_inline(discount);
            format!(
                "💰 *{price_str}* (~De {}~ · *{safe_disc}*)",
                format_brl_cents(ref_cents)
            )
        }
        (Some(ref_cents), None) if ref_cents > item.price_brl_cents => {
            format!("💰 *{price_str}* (~De {}~)", format_brl_cents(ref_cents))
        }
        (_, Some(discount)) => {
            let safe_disc = sanitize_whatsapp_inline(discount);
            format!("💰 *{price_str}* (*{safe_disc}*)")
        }
        _ => format!("💰 *{price_str}*"),
    };

    let mut lines = vec![
        format!("_{}_", AFFILIATE_DISCLOSURE_PT_BR),
        String::new(),
        format!("🔥 *{safe_title}*"),
        match &item.model {
            Some(model) => format!(
                "Marca: *{safe_brand}* · Modelo: *{}*",
                sanitize_whatsapp_inline(model)
            ),
            None => format!("Marca: *{safe_brand}*"),
        },
        price_line,
    ];

    for extra in item.message_text.lines().filter(|line| {
        let t = line.trim();
        t.starts_with("💳") || t.starts_with("🏪") || t.starts_with("⚠️")
    }) {
        lines.push(sanitize_whatsapp_inline(extra));
    }

    lines.push(format!(
        "_Observado em {}_",
        format_cuiaba_display(item.updated_at_ms)
    ));
    lines.push(String::new());
    lines.push(format!("🔎 Código ML: ```{}```", item.share_id.trim()));
    lines.push(format!("🔗 Oferta oficial: {}", item.share_link.trim()));
    if let Some(url) = &site_offer_url {
        lines.push(format!("📋 Ficha completa no Procurali: {url}"));
    }

    let operator_checklist = vec![
        "Envio humano assistido no WhatsApp Web (proibido usar API da Meta ou automação não oficial no WhatsApp Web).".to_owned(),
        format!(
            "Conferir se preço ({price_str}) e estoque no link oficial {} ainda estão válidos (< 30 min).",
            item.share_link
        ),
        match &item.image_public_url {
            Some(img) => format!("Opcional: anexar foto estática verificada do MinIO ({img}) ou aguardar preview Open Graph do link Procurali."),
            None => "Sem foto estática: aguardar card de pré-visualização Open Graph do link Procurali ao colar no WhatsApp Web.".to_owned(),
        },
        "Colar a mensagem manualmente no grupo autorizado pelo proprietário no WhatsApp Web e revisar antes de apertar Enviar.".to_owned(),
    ];

    WhatsappAssistedDraft {
        delivery_id: item.delivery_id.clone(),
        product_id: item.product_id.clone(),
        slug,
        title: item.title.clone(),
        price_brl_cents: item.price_brl_cents,
        reference_price_cents: item.reference_price_cents,
        discount_label: item.discount_label.clone(),
        share_link: item.share_link.clone(),
        share_id: item.share_id.clone(),
        image_public_url: item.image_public_url.clone(),
        site_offer_url,
        whatsapp_text: lines.join("\n"),
        operator_checklist,
    }
}

pub fn build_whatsapp_assisted_package(
    items: &[TelegramOfferQueueItem],
    channel_id: &str,
    site_base_url: Option<&str>,
    now_ms: i64,
) -> WhatsappAssistedPackage {
    let drafts: Vec<WhatsappAssistedDraft> = items
        .iter()
        .map(|item| build_whatsapp_assisted_draft(item, site_base_url))
        .collect();

    let combined_digest_text = if drafts.is_empty() {
        format!(
            "_Procurali · Resumo manual para WhatsApp Web_\nNenhuma oferta elegível na fila do canal {}.",
            sanitize_whatsapp_inline(channel_id)
        )
    } else {
        let mut digest = vec![
            format!(
                "*Seleção Procurali · Achados verificados* ({})",
                format_cuiaba_short(now_ms)
            ),
            format!("_{}_", AFFILIATE_DISCLOSURE_PT_BR),
            String::new(),
        ];
        for (idx, draft) in drafts.iter().enumerate() {
            let safe_title = sanitize_whatsapp_inline(&truncate_chars(&draft.title, 90));
            let price_str = format_brl_cents(draft.price_brl_cents);
            let badge = draft
                .discount_label
                .as_deref()
                .map(|d| format!(" ({})", sanitize_whatsapp_inline(d)))
                .unwrap_or_default();
            digest.push(format!("{}. *{safe_title}*", idx + 1));
            digest.push(format!(
                "   💰 *{price_str}*{badge} · 🔗 {}",
                draft.share_link
            ));
            if let Some(url) = &draft.site_offer_url {
                digest.push(format!("   📋 Ficha: {url}"));
            }
        }
        digest.push(String::new());
        digest.push(
            "_Revisão humana obrigatória no WhatsApp Web antes de enviar (sem API da Meta)._"
                .to_owned(),
        );
        digest.join("\n")
    };

    WhatsappAssistedPackage {
        generated_at: format_utc_iso_ms(now_ms),
        mode: "whatsapp_web_human_assisted_no_meta_api".to_owned(),
        channel_id: channel_id.to_owned(),
        draft_count: drafts.len(),
        combined_digest_text,
        drafts,
    }
}

pub fn format_telegram_bot_command_reply_pt_br(
    command: &TelegramBotCommand,
    items: &[TelegramOfferQueueItem],
    site_base_url: Option<&str>,
) -> (String, Option<TelegramInlineKeyboardMarkup>) {
    let active_items: Vec<&TelegramOfferQueueItem> = {
        let filtered: Vec<&TelegramOfferQueueItem> = items
            .iter()
            .filter(|i| {
                matches!(
                    i.status,
                    TelegramDeliveryStatus::Published | TelegramDeliveryStatus::Queued
                )
            })
            .collect();
        if filtered.is_empty() {
            items.iter().collect()
        } else {
            filtered
        }
    };

    let catalog_button_row = || -> Option<Vec<TelegramInlineButton>> {
        let base = site_base_url?.trim().trim_end_matches('/');
        if base.starts_with("https://") || base.starts_with("http://") {
            Some(vec![TelegramInlineButton {
                text: "🌐 Abrir catálogo completo no Procurali".to_owned(),
                url: format!("{base}/ofertas/index.html"),
            }])
        } else {
            None
        }
    };

    match command {
        TelegramBotCommand::Start => {
            let mut lines = vec![
                "👋 <b>Bem-vindo ao bot oficial do Procurali!</b>".to_owned(),
                format!("<i>{}</i>", escape_telegram_html(AFFILIATE_DISCLOSURE_PT_BR)),
                String::new(),
                "Comandos disponíveis:".to_owned(),
                "• <code>/ofertas</code> — últimas ofertas verificadas".to_owned(),
                "• <code>/hardware</code> — achados de hardware e periféricos".to_owned(),
                "• <code>/buscar &lt;termo&gt;</code> — procurar produto (ex.: <code>/buscar ssd</code>)".to_owned(),
                "• <code>/whatsapp</code> — gerar rascunho pronto para colar manualmente no WhatsApp Web".to_owned(),
                "• <code>/ajuda</code> — regras de transparência e links oficiais".to_owned(),
            ];
            let mut rows = Vec::new();
            if !active_items.is_empty() {
                lines.push(String::new());
                lines.push("🔥 <b>Destaques recentes:</b>".to_owned());
                for item in active_items.iter().take(3) {
                    let short = truncate_chars(&item.title, 65);
                    lines.push(format!(
                        "• <b>{}</b> — <b>{}</b> · <a href=\"{}\">Ver no ML</a>",
                        escape_telegram_html(&short),
                        escape_telegram_html(&format_brl_cents(item.price_brl_cents)),
                        escape_telegram_html(&item.share_link),
                    ));
                    rows.push(vec![TelegramInlineButton {
                        text: format!("🛒 {}", truncate_chars(&item.title, 36)),
                        url: item.share_link.clone(),
                    }]);
                }
            }
            if let Some(cat_row) = catalog_button_row() {
                rows.push(cat_row);
            }
            let markup = (!rows.is_empty()).then_some(TelegramInlineKeyboardMarkup {
                inline_keyboard: rows,
            });
            (
                truncate_chars(&lines.join("\n"), TELEGRAM_MESSAGE_MAX_CHARS),
                markup,
            )
        }
        TelegramBotCommand::Help => {
            let lines = [
                "ℹ️ <b>Como funciona o Procurali (@procurali)</b>".to_owned(),
                format!("<i>{}</i>", escape_telegram_html(AFFILIATE_DISCLOSURE_PT_BR)),
                String::new(),
                "• Todos os links do Mercado Livre preservam o endereço oficial <code>meli.la</code> sem alterações.".to_owned(),
                "• Preços são registrados com horário de observação em Cuiabá e podem variar após o envio.".to_owned(),
                "• Para WhatsApp: use <code>/whatsapp</code> para copiar o pacote assistido e enviar manualmente no WhatsApp Web (sem API da Meta).".to_owned(),
            ];
            let mut rows = Vec::new();
            if let Some(cat_row) = catalog_button_row() {
                rows.push(cat_row);
            }
            let markup = (!rows.is_empty()).then_some(TelegramInlineKeyboardMarkup {
                inline_keyboard: rows,
            });
            (
                truncate_chars(&lines.join("\n"), TELEGRAM_MESSAGE_MAX_CHARS),
                markup,
            )
        }
        TelegramBotCommand::Offers | TelegramBotCommand::Hardware => {
            if active_items.is_empty() {
                return (
                    "Ainda não há ofertas ativas na fila do Hub no momento. Tente novamente em instantes.".to_owned(),
                    catalog_button_row().map(|row| TelegramInlineKeyboardMarkup {
                        inline_keyboard: vec![row],
                    }),
                );
            }
            let header = if matches!(command, TelegramBotCommand::Hardware) {
                "🖥️ <b>Achados de Hardware e Periféricos</b>"
            } else {
                "⚡ <b>Ofertas ativas verificadas pelo Procurali</b>"
            };
            let mut lines = vec![
                header.to_owned(),
                format!(
                    "<i>{}</i>",
                    escape_telegram_html(AFFILIATE_DISCLOSURE_PT_BR)
                ),
                String::new(),
            ];
            let mut rows = Vec::new();
            for item in active_items.iter().take(4) {
                let disc = item
                    .discount_label
                    .as_deref()
                    .map(|d| format!(" ({})", escape_telegram_html(d)))
                    .unwrap_or_default();
                lines.push(format!(
                    "• <b>{}</b>\n  💰 <b>{}</b>{disc} · Cód: <code>{}</code> · <a href=\"{}\">Abrir oferta</a>",
                    escape_telegram_html(&truncate_chars(&item.title, 85)),
                    escape_telegram_html(&format_brl_cents(item.price_brl_cents)),
                    escape_telegram_html(&item.share_id),
                    escape_telegram_html(&item.share_link),
                ));
                rows.push(vec![TelegramInlineButton {
                    text: format!("🛒 {}", truncate_chars(&item.title, 36)),
                    url: item.share_link.clone(),
                }]);
            }
            if let Some(cat_row) = catalog_button_row() {
                rows.push(cat_row);
            }
            (
                truncate_chars(&lines.join("\n"), TELEGRAM_MESSAGE_MAX_CHARS),
                Some(TelegramInlineKeyboardMarkup {
                    inline_keyboard: rows,
                }),
            )
        }
        TelegramBotCommand::Search(term) => {
            let needle = term.to_ascii_lowercase();
            let matches: Vec<&TelegramOfferQueueItem> = active_items
                .iter()
                .copied()
                .filter(|item| {
                    let hay = format!(
                        "{} {} {} {} {}",
                        item.title,
                        item.brand,
                        item.model.as_deref().unwrap_or(""),
                        item.product_id,
                        item.share_id
                    )
                    .to_ascii_lowercase();
                    hay.contains(&needle)
                })
                .take(4)
                .collect();
            if matches.is_empty() {
                return (
                    format!(
                        "Nenhum achado encontrado para <code>{}</code> agora. Use <code>/ofertas</code> para listar todos os itens ativos.",
                        escape_telegram_html(term)
                    ),
                    catalog_button_row().map(|row| TelegramInlineKeyboardMarkup {
                        inline_keyboard: vec![row],
                    }),
                );
            }
            let mut lines = vec![
                format!(
                    "🔎 <b>Resultados para &quot;{}&quot;</b>",
                    escape_telegram_html(term)
                ),
                format!(
                    "<i>{}</i>",
                    escape_telegram_html(AFFILIATE_DISCLOSURE_PT_BR)
                ),
                String::new(),
            ];
            let mut rows = Vec::new();
            for item in matches {
                lines.push(format!(
                    "• <b>{}</b> — <b>{}</b> · <a href=\"{}\">Ver no ML</a>",
                    escape_telegram_html(&truncate_chars(&item.title, 80)),
                    escape_telegram_html(&format_brl_cents(item.price_brl_cents)),
                    escape_telegram_html(&item.share_link),
                ));
                rows.push(vec![TelegramInlineButton {
                    text: format!("🛒 {}", truncate_chars(&item.title, 36)),
                    url: item.share_link.clone(),
                }]);
            }
            (
                truncate_chars(&lines.join("\n"), TELEGRAM_MESSAGE_MAX_CHARS),
                Some(TelegramInlineKeyboardMarkup {
                    inline_keyboard: rows,
                }),
            )
        }
        TelegramBotCommand::WhatsappDigest => {
            let owned: Vec<TelegramOfferQueueItem> =
                active_items.iter().take(3).copied().cloned().collect();
            let now_ms = owned.first().map(|i| i.updated_at_ms).unwrap_or(1);
            let pkg = build_whatsapp_assisted_package(&owned, "@procurali", site_base_url, now_ms);
            let text = format!(
                "📋 <b>Rascunho pronto para auxiliar humano no WhatsApp Web</b>\n<i>Sem API da Meta e sem automação no WhatsApp Web — copie e revise antes de colar no grupo:</i>\n\n<pre>{}</pre>",
                escape_telegram_html(&truncate_chars(&pkg.combined_digest_text, 3400))
            );
            (truncate_chars(&text, TELEGRAM_MESSAGE_MAX_CHARS), None)
        }
    }
}

pub fn format_telegram_caption_pt_br(product: &ValidatedMercadoLivreProduct) -> String {
    let mut lines = Vec::new();
    lines.push("🚨 OFERTA RELÂMPAGO · ACHADO NO MERCADO LIVRE! 🔥".to_owned());
    lines.push(String::new());
    lines.push(format!("📦 {}", product.title));

    let mut identity_parts = vec![format!("🏷️ Marca: {}", product.brand)];
    if let Some(model) = &product.model {
        identity_parts.push(format!("Modelo: {model}"));
    }
    identity_parts.push(format!("Cód. ML: {}", product.product_id));
    lines.push(identity_parts.join(" · "));
    lines.push(String::new());

    let price_str = format_brl_cents(product.price_brl_cents);
    match product.reference_price_cents {
        Some(ref_cents) if ref_cents > product.price_brl_cents => {
            let savings_cents = ref_cents - product.price_brl_cents;
            let savings_str = format_brl_cents(savings_cents);
            let computed_pct = ((savings_cents as f64 / ref_cents as f64) * 100.0).round() as u32;
            let badge = product
                .discount_label
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("{computed_pct}% OFF"));
            lines.push(format!("❌ De: {}", format_brl_cents(ref_cents)));
            lines.push(format!(
                "✅ POR APENAS: {price_str} 🔥 ({badge} · Economia de {savings_str}!)"
            ));
        }
        _ => {
            if let Some(discount) = product
                .discount_label
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                lines.push(format!("✅ PREÇO EM OFERTA: {price_str} 🔥 ({discount})"));
            } else {
                lines.push(format!("✅ PREÇO EM OFERTA: {price_str} 🔥"));
            }
        }
    }

    lines.push(format!("💳 {}", product.payment_conditions));

    let mut store_parts = vec![format!("🏪 {}", product.seller_name)];
    if let Some(rating) = &product.marketplace_rating {
        let sales = product
            .marketplace_sales_label
            .as_deref()
            .map(|s| format!(" ({s})"))
            .unwrap_or_default();
        store_parts.push(format!("⭐ {rating}{sales}"));
    }
    lines.push(store_parts.join(" · "));

    if let Some(first_limitation) = product.limitations.first() {
        lines.push(format!("⚠️ Atenção antes de comprar: {first_limitation}"));
    }

    lines.push(format!(
        "🕐 Atualizado em {}",
        product.observed_at_display_for_telegram()
    ));
    lines.push(String::new());
    lines.push(
        "⏳ CORRA! Preço e estoque promocional sujeitos a alteração a qualquer momento!".to_owned(),
    );
    lines.push(format!("👉 GARANTA O SEU AQUI: {}", product.share_link));
    lines.push(format!("🔍 Código rápido no app ML: {}", product.share_id));
    lines.push(AFFILIATE_DISCLOSURE_PT_BR.to_owned());
    lines.join("\n")
}

impl ValidatedMercadoLivreProduct {
    pub fn observed_at_display_for_telegram(&self) -> String {
        format_cuiaba_display(self.observed_at_ms)
    }

    pub fn to_site_catalog_offer(
        &self,
        evidence_id: &str,
        media_rights_id: &str,
    ) -> SiteCatalogOffer {
        let lead = &self.images[0];
        let is_in_stock = self.availability == "InStock";
        let return_note = self
            .return_days
            .map(|d| format!("Devolução grátis {d}d"))
            .unwrap_or_else(|| "Devolução conforme anúncio".into());
        let short_use_line = if self.short_description.chars().count() > 140 {
            let truncated: String = self.short_description.chars().take(137).collect();
            format!("{truncated}...")
        } else {
            self.short_description.clone()
        };
        let mut offer = SiteCatalogOffer {
            id: format!("offer-ml-{}", self.product_id),
            slug: self.slug.clone(),
            is_demo_fixture: false,
            product_id: self.product_id.clone(),
            brand: self.brand.clone(),
            model: self.model.clone(),
            title: self.title.clone(),
            exact_variant: self.exact_variant.clone(),
            category_slug: self.category_slug.clone(),
            subcategory_slug: String::new(),
            subcategory_name: String::new(),
            merchant: format!("Mercado Livre · {}", self.seller_name),
            marketplace_label: "Mercado Livre".into(),
            seller_name: self.seller_name.clone(),
            store_short: self.seller_name.clone(),
            program: "Mercado Livre Afiliados (ingestão validada via Hub)".into(),
            official_affiliate_url: self.share_link.clone(),
            canonical_url: self.canonical_url.clone(),
            share_id: self.share_id.clone(),
            share_text: self.share_text.clone(),
            sponsored: self.sponsored,
            commission_rate: self.commission_label.clone(),
            marketplace_rating: self.marketplace_rating.clone(),
            rating_milli: None,
            marketplace_sales_label: self.marketplace_sales_label.clone(),
            sales_volume_estimate: 0,
            card_badge: self.card_badge.clone(),
            discount_percent_label: self.discount_label.clone(),
            discount_pct: 0,
            trust_badges: Vec::new(),
            installment_summary: self.installment_summary.clone(),
            return_days: self.return_days,
            availability: self.availability.clone(),
            price_valid_until: self.price_valid_until.clone(),
            image_path: lead.public_url.clone(),
            image_alt: lead.alt_text.clone(),
            gallery_paths: self
                .images
                .iter()
                .map(|img| SiteCatalogGalleryItem {
                    path: img.public_url.clone(),
                    alt: img.alt_text.clone(),
                    bucket: Some(img.bucket.clone()),
                    object_key: Some(img.object_key.clone()),
                    sha256: Some(img.sha256_hex.clone()),
                })
                .collect(),
            price_brl_cents: self.price_brl_cents,
            pix_price_cents: self.pix_price_cents,
            reference_price_cents: self.reference_price_cents,
            currency: self.currency.clone(),
            payment_conditions: self.payment_conditions.clone(),
            short_condition: self.short_condition.clone(),
            short_use_line,
            short_checked_at: format_cuiaba_short(self.observed_at_ms),
            cta_dest_label: "Ver oferta no Mercado Livre".into(),
            coupon_condition: None,
            freight_status: if is_in_stock {
                self.freight_status.clone()
            } else {
                format!("{} · {return_note}", self.freight_status)
            },
            observed_at: format_utc_iso_ms(self.observed_at_ms),
            observed_at_display: format_cuiaba_display(self.observed_at_ms),
            valid_until: self
                .price_valid_until
                .as_ref()
                .map(|d| format!("{d}T23:59:59-03:00")),
            valid_until_display: self
                .price_valid_until
                .as_ref()
                .map(|d| format!("Validade registrada até {d}")),
            freshness_ttl_minutes: self.freshness_ttl_minutes,
            evidence_refs: vec![evidence_id.to_owned()],
            media_rights_ref: media_rights_id.to_owned(),
            state: self.state.as_str().to_owned(),
            approval_revision: format!(
                "ml-ingested-{}",
                self.evidence_sha256.chars().take(12).collect::<String>()
            ),
            one_line_reason: self.one_line_reason.clone(),
            for_whom_it_makes_sense: self.for_whom.clone(),
            supported_benefits: self.supported_benefits.clone(),
            limitations: self.limitations.clone(),
            unknowns: self.unknowns.clone(),
            tags: self.tags.clone(),
            has_verified_seven_day_sample: false,
            seven_day_coverage_note:
                "Ingestão validada via Hub Mercado Livre; série contínua de 7 dias acumulada progressivamente."
                    .into(),
            price_observations: vec![SiteCatalogPriceObservation {
                id: format!("obs-ml-{}-{}", self.product_id, self.observed_at_ms),
                offer_id: format!("offer-ml-{}", self.product_id),
                variant_id: self
                    .model
                    .clone()
                    .unwrap_or_else(|| self.product_id.clone()),
                seller_name: self.seller_name.clone(),
                payment_basis: self.payment_conditions.clone(),
                freight_note: self.short_condition.clone(),
                price_brl_cents: self.price_brl_cents,
                observed_at: format_utc_iso_ms(self.observed_at_ms),
                evidence_id: evidence_id.to_owned(),
            }],
            specs: self.specs.clone(),
            variants: self.variants.clone(),
            long_description: self.long_description.clone(),
            price_intelligence: None,
            dynamic_faq: Vec::new(),
            similar_offer_ids: Vec::new(),
        };
        enrich_site_catalog_offer(&mut offer, self.observed_at_ms);
        offer
    }
}

/// Classifies an offer into a two-level category/subcategory taxonomy inspired by cloo.be.
pub fn classify_product_subcategory(
    title: &str,
    category_slug: &str,
    specs: &[MercadoLivreSpec],
) -> (String, String) {
    let mut hay = title.to_ascii_lowercase();
    for spec in specs.iter().take(12) {
        hay.push(' ');
        hay.push_str(&spec.name.to_ascii_lowercase());
        hay.push(' ');
        hay.push_str(&spec.value.to_ascii_lowercase());
    }
    let rules: &[(&[&str], &str, &str)] = &[
        (
            &[
                "monitor",
                "ultragear",
                "aoc agon",
                "144hz",
                "200hz",
                "displayport",
            ],
            "monitores",
            "Monitores",
        ),
        (
            &[
                "ssd",
                "nvme",
                "m.2",
                "pcie",
                "disco sólido",
                "armazenamento",
                "hd externo",
            ],
            "ssd-e-armazenamento",
            "SSDs e Armazenamento",
        ),
        (
            &[
                "teclado",
                "mouse",
                "kumara",
                "switch",
                "mecanico",
                "mecânico",
            ],
            "teclados-e-mouses",
            "Teclados e Mouses",
        ),
        (
            &[
                "headset",
                "fone",
                "audio",
                "áudio",
                "microfone",
                "drivers 53mm",
            ],
            "headsets-e-audio",
            "Headsets e Áudio",
        ),
        (
            &[
                "carregador",
                "gan",
                "cabo",
                "filtro de linha",
                "dps",
                "tomada",
                "hub usb",
            ],
            "cabos-e-energia",
            "Cabos, Hubs e Energia",
        ),
        (
            &["suporte", "ergonomia", "articulado", "braço", "stand"],
            "suportes-e-ergonomia",
            "Suportes e Ergonomia",
        ),
        (
            &["luminaria", "luminária", "lightbar", "led"],
            "iluminacao",
            "Iluminação de Bancada",
        ),
    ];
    for (keywords, slug, label) in rules {
        if keywords.iter().any(|kw| hay.contains(kw)) {
            return ((*slug).to_owned(), (*label).to_owned());
        }
    }
    match category_slug {
        "ergonomia-e-suportes" => ("suportes-e-ergonomia".into(), "Suportes e Ergonomia".into()),
        "energia-e-cabos" => ("cabos-e-energia".into(), "Cabos, Hubs e Energia".into()),
        "iluminacao-de-bancada" => ("iluminacao".into(), "Iluminação de Bancada".into()),
        _ => (
            "acessorios-gerais".into(),
            "Acessórios e Periféricos".into(),
        ),
    }
}

/// Parses marketplace rating string such as `"4,9 / 5"` into milli-stars (`4900`).
pub fn parse_rating_milli(rating: Option<&str>) -> Option<u16> {
    let raw = rating?.trim();
    let score_part = raw.split('/').next()?.trim().replace(',', ".");
    let parsed: f64 = score_part.parse().ok()?;
    if !(0.0..=5.0).contains(&parsed) {
        return None;
    }
    Some((parsed * 1000.0).round() as u16)
}

/// Parses marketplace sales labels such as `"+10 mil vendidos no ML"` into integer volume.
pub fn parse_sales_volume_estimate(sales: Option<&str>) -> u32 {
    let Some(raw) = sales else {
        return 0;
    };
    let lower = raw.to_ascii_lowercase();
    let digits: String = lower
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .filter(|c| c.is_ascii_digit())
        .collect();
    let Ok(base) = digits.parse::<u32>() else {
        return 0;
    };
    if lower.contains("mil") && base < 1000 {
        base.saturating_mul(1000)
    } else {
        base
    }
}

/// Resolves integer discount percentage (`0..=99`) from prices or explicit badge text.
pub fn compute_discount_pct(
    price_cents: i64,
    reference_cents: Option<i64>,
    discount_label: Option<&str>,
) -> u8 {
    if let Some(ref_c) = reference_cents
        && ref_c > price_cents
        && price_cents > 0
    {
        let pct = ((ref_c - price_cents) as f64 / ref_c as f64 * 100.0).round() as u32;
        return u8::try_from(pct.clamp(1, 99)).unwrap_or(99);
    }
    if let Some(lbl) = discount_label {
        let digits: String = lbl
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(pct) = digits.parse::<u8>() {
            return pct.min(99);
        }
    }
    0
}

pub fn format_verified_at_cuiaba(ms: i64) -> String {
    let offset = FixedOffset::west_opt(4 * 3600).expect("valid UTC-4 offset");
    let dt = DateTime::<Utc>::from_timestamp_millis(ms)
        .unwrap_or_default()
        .with_timezone(&offset);
    format!("Preços verificados em {}", dt.format("%d/%m/%Y, %H:%M"))
}

/// Renders a zero-dependency server-side `<svg viewBox="0 0 300 80">` sparkline chart.
pub fn build_price_sparkline_svg(
    chronological_prices: &[i64],
    lowest_cents: i64,
    current_cents: i64,
) -> String {
    let samples: Vec<i64> = if chronological_prices.len() >= 2 {
        chronological_prices.to_vec()
    } else if chronological_prices.len() == 1 && lowest_cents != current_cents {
        vec![lowest_cents.max(current_cents), current_cents]
    } else {
        vec![current_cents, current_cents]
    };

    let min_v = samples.iter().copied().min().unwrap_or(current_cents);
    let max_v = samples.iter().copied().max().unwrap_or(current_cents);
    let span = (max_v - min_v).max(1) as f64;
    let width = 280.0f64;
    let pad_x = 10.0f64;
    let height = 56.0f64;
    let pad_top = 12.0f64;
    let count = samples.len().max(2);

    let mut coords = Vec::with_capacity(samples.len());
    let mut area_points = Vec::with_capacity(samples.len() + 2);
    let mut min_idx = 0usize;
    for (idx, price) in samples.iter().enumerate() {
        if *price == min_v {
            min_idx = idx;
        }
        let x = pad_x + (idx as f64 / (count - 1) as f64) * width;
        let norm_y = if max_v == min_v {
            0.5
        } else {
            (*price - min_v) as f64 / span
        };
        let y = pad_top + (1.0 - norm_y) * height;
        coords.push((x, y));
        area_points.push(format!("{x:.1},{y:.1}"));
    }

    let points_attr = area_points.join(" ");
    let last_x = coords.last().map(|p| p.0).unwrap_or(290.0);
    let first_x = coords.first().map(|p| p.0).unwrap_or(10.0);
    let base_y = pad_top + height + 6.0;
    let fill_attr = format!("{first_x:.1},{base_y:.1} {points_attr} {last_x:.1},{base_y:.1}");
    let (min_x, min_y) = coords[min_idx];
    let (cur_x, cur_y) = coords[coords.len() - 1];

    format!(
        "<svg viewBox=\"0 0 300 80\" width=\"300\" height=\"80\" role=\"img\" aria-label=\"Histórico de preço observado nos últimos 30 dias\" xmlns=\"http://www.w3.org/2000/svg\"><polygon points=\"{fill_attr}\" fill=\"rgba(22, 101, 52, 0.10)\"/><polyline fill=\"none\" stroke=\"#166534\" stroke-width=\"2.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\" points=\"{points_attr}\"/><circle cx=\"{min_x:.1}\" cy=\"{min_y:.1}\" r=\"3.5\" fill=\"#65a30d\"/><circle cx=\"{cur_x:.1}\" cy=\"{cur_y:.1}\" r=\"4\" fill=\"#c2410c\"/></svg>"
    )
}

/// Computes 30-day window pricing intelligence, classification, savings, and SVG sparkline.
pub fn compute_price_intelligence(
    current_price_cents: i64,
    reference_price_cents: Option<i64>,
    discount_label: Option<&str>,
    observed_at_ms: i64,
    observations: &[SiteCatalogPriceObservation],
) -> PriceIntelligence {
    let mut chronological: Vec<i64> = observations
        .iter()
        .rev()
        .map(|obs| obs.price_brl_cents)
        .filter(|c| *c > 0)
        .collect();
    if chronological.is_empty() && current_price_cents > 0 {
        chronological.push(current_price_cents);
    }

    let obs_min = chronological
        .iter()
        .copied()
        .min()
        .unwrap_or(current_price_cents);
    let obs_max = chronological
        .iter()
        .copied()
        .max()
        .unwrap_or(current_price_cents);
    let obs_sum: i128 = chronological.iter().map(|c| i128::from(*c)).sum();
    let avg_cents = if chronological.is_empty() {
        current_price_cents
    } else {
        i64::try_from(obs_sum / i128::try_from(chronological.len()).unwrap_or(1))
            .unwrap_or(current_price_cents)
    };

    let lowest_price_30d_cents = obs_min.min(current_price_cents);
    let highest_price_30d_cents = reference_price_cents
        .filter(|r| *r > 0)
        .map_or(obs_max.max(current_price_cents), |r| {
            obs_max.max(current_price_cents).max(r)
        });
    let discount_pct =
        compute_discount_pct(current_price_cents, reference_price_cents, discount_label);
    let savings_vs_reference_cents = reference_price_cents
        .filter(|r| *r > current_price_cents)
        .map_or(0, |r| r - current_price_cents);
    let savings_vs_average_cents = (avg_cents - current_price_cents).max(0);

    // Price drop occurs if the prior observed snapshot was higher or discount is meaningful.
    let prev_higher = observations
        .get(1)
        .is_some_and(|prev| prev.price_brl_cents > current_price_cents);
    let is_price_drop = prev_higher || discount_pct >= 20;

    let price_level = if chronological.len() >= 2 {
        if current_price_cents <= lowest_price_30d_cents && current_price_cents < avg_cents {
            PriceLevelKind::LowestPrice
        } else if current_price_cents.saturating_mul(100) <= avg_cents.saturating_mul(92)
            || discount_pct >= 20
        {
            PriceLevelKind::GoodPrice
        } else {
            PriceLevelKind::TypicalPrice
        }
    } else if discount_pct >= 25 {
        PriceLevelKind::LowestPrice
    } else if discount_pct >= 10 {
        PriceLevelKind::GoodPrice
    } else {
        PriceLevelKind::TypicalPrice
    };

    let sparkline_points = if chronological.len() >= 2 {
        chronological
    } else if let Some(ref_c) = reference_price_cents.filter(|r| *r > current_price_cents) {
        vec![ref_c, current_price_cents]
    } else {
        vec![current_price_cents]
    };
    let svg_sparkline = build_price_sparkline_svg(
        &sparkline_points,
        highest_price_30d_cents,
        current_price_cents,
    );

    PriceIntelligence {
        window_days: 30,
        current_price_cents,
        lowest_price_30d_cents,
        average_price_30d_cents: avg_cents,
        highest_price_30d_cents,
        reference_price_cents,
        discount_pct,
        savings_vs_reference_cents,
        savings_vs_average_cents,
        price_level,
        price_level_label: price_level.label_pt_br().to_owned(),
        is_price_drop,
        verified_at_display: format_verified_at_cuiaba(observed_at_ms),
        svg_sparkline,
    }
}

pub fn build_trust_badges(offer: &SiteCatalogOffer, intel: &PriceIntelligence) -> Vec<String> {
    let mut badges = Vec::new();
    if intel.discount_pct >= 5 {
        badges.push(format!("-{}%", intel.discount_pct));
    }
    let seller_lower = offer.seller_name.to_ascii_lowercase();
    if seller_lower.contains("oficial") {
        badges.push("OFICIAL".to_owned());
    }
    if offer.card_badge.is_some() {
        badges.push("MAIS VENDIDO".to_owned());
    }
    if intel.price_level == PriceLevelKind::LowestPrice {
        badges.push("MENOR PREÇO".to_owned());
    } else if intel.is_price_drop {
        badges.push("BAIXOU DE PREÇO".to_owned());
    }
    if offer.return_days.is_some_and(|d| d >= 7) {
        badges.push("DEVOLUÇÃO GRÁTIS".to_owned());
    }
    badges
}

pub fn build_dynamic_product_faq(
    offer: &SiteCatalogOffer,
    intel: &PriceIntelligence,
) -> Vec<ProductFaqItem> {
    let cur = format_brl_cents(offer.price_brl_cents);
    let low = format_brl_cents(intel.lowest_price_30d_cents);
    let avg = format_brl_cents(intel.average_price_30d_cents);
    let mut faq = vec![ProductFaqItem {
        question: format!("O preço atual de {} está bom hoje?", offer.brand),
        answer: format!(
            "No monitoramento dos últimos {} dias, a oferta atual ({cur}) recebeu classificação \"{}\". O menor preço registrado no período foi {low} e a média observada é {avg}. {}",
            intel.window_days, intel.price_level_label, intel.verified_at_display
        ),
    }];
    faq.push(ProductFaqItem {
        question: "O vendedor e o link de compra são oficiais?".to_owned(),
        answer: format!(
            "Sim. O produto {} ({}) direciona exclusivamente para o anúncio oficial {} em {} sem encurtadores paralelos.",
            offer.product_id, offer.seller_name, offer.official_affiliate_url, offer.marketplace_label
        ),
    });
    faq.push(ProductFaqItem {
        question: "Qual é a política de pagamento, frete e devolução?".to_owned(),
        answer: format!("{} · {}.", offer.payment_conditions, offer.freight_status),
    });
    if let Some(lim) = offer.limitations.first() {
        faq.push(ProductFaqItem {
            question: "O que devo conferir antes de comprar?".to_owned(),
            answer: lim.clone(),
        });
    }
    faq
}

/// Populates taxonomy, ratings, sales volume, trust badges, price intelligence, and FAQ cleanly.
pub fn enrich_site_catalog_offer(offer: &mut SiteCatalogOffer, observed_at_ms: i64) {
    let (sub_slug, sub_name) =
        classify_product_subcategory(&offer.title, &offer.category_slug, &offer.specs);
    offer.subcategory_slug = sub_slug;
    offer.subcategory_name = sub_name;
    offer.rating_milli = parse_rating_milli(offer.marketplace_rating.as_deref());
    offer.sales_volume_estimate =
        parse_sales_volume_estimate(offer.marketplace_sales_label.as_deref());
    let intel = compute_price_intelligence(
        offer.pix_price_cents.min(offer.price_brl_cents),
        offer.reference_price_cents,
        offer.discount_percent_label.as_deref(),
        observed_at_ms,
        &offer.price_observations,
    );
    offer.discount_pct = intel.discount_pct;
    offer.trust_badges = build_trust_badges(offer, &intel);
    offer.dynamic_faq = build_dynamic_product_faq(offer, &intel);
    offer.price_intelligence = Some(intel);
}

/// Renders zero-hydration Cloobe-style `<article class="offer-card">` HTML fragments for incremental pagination.
pub fn render_offer_cards_html_fragment(offers: &[SiteCatalogOffer], start_pos: usize) -> String {
    let mut html = String::with_capacity(offers.len().saturating_mul(640));
    for (idx, offer) in offers.iter().enumerate() {
        let pos = start_pos.saturating_add(idx).saturating_add(1);
        let badges_html = offer
            .trust_badges
            .iter()
            .take(3)
            .map(|b| format!("<span class=\"badge\">{}</span>", escape_telegram_html(b)))
            .collect::<Vec<_>>()
            .join("");
        let ref_html = offer
            .reference_price_cents
            .filter(|r| *r > offer.price_brl_cents)
            .map(|r| format!("<del>{}</del>", escape_telegram_html(&format_brl_cents(r))))
            .unwrap_or_default();
        let intel_badge = offer
            .price_intelligence
            .as_ref()
            .map(|i| escape_telegram_html(&i.price_level_label))
            .unwrap_or_default();
        html.push_str(&format!(
            "<article class=\"offer-card\" data-store=\"mercadolivre\" data-id=\"{}\" data-pos=\"{pos}\" data-subcat=\"{}\"><a class=\"offer-card__link\" href=\"/ofertas/{}/index.html\"><img src=\"{}\" alt=\"{}\" loading=\"lazy\" decoding=\"async\"><div class=\"offer-card__badges\">{badges_html}</div><h3>{}</h3><p class=\"offer-card__price\">{ref_html}<strong>{}</strong></p><p class=\"offer-card__meta\">{} · {}</p></a></article>",
            escape_telegram_html(&offer.product_id),
            escape_telegram_html(&offer.subcategory_slug),
            escape_telegram_html(&offer.slug),
            escape_telegram_html(&offer.image_path),
            escape_telegram_html(&offer.image_alt),
            escape_telegram_html(&offer.title),
            escape_telegram_html(&format_brl_cents(offer.price_brl_cents)),
            intel_badge,
            escape_telegram_html(&offer.short_condition),
        ));
    }
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_ml_ids_urls_and_rejects_local_leaks() {
        assert!(validate_ml_product_id("MLB39410080"));
        assert!(!validate_ml_product_id("MLA123456"));
        assert!(!validate_ml_product_id("MLB12"));

        assert!(validate_meli_share_link("https://meli.la/1iWQZbJ"));
        assert!(!validate_meli_share_link("https://evil.test/1iWQZbJ"));
        assert!(!validate_meli_share_link("https://meli.la/abc?extra=1"));

        assert!(validate_ml_canonical_url(
            "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb-m2-2280-nvme-pcie-40/p/MLB39410080"
        ));
        assert!(!validate_ml_canonical_url(
            "https://www.mercadolivre.com.br/item?matt_tool=123"
        ));
        assert!(no_local_path_leak(
            "https://media.procurali.com.br/procurali-media/MLB1/01.jpg"
        ));
        assert!(!no_local_path_leak(
            "/home/iiii/.local/commercial/mercadolivre/images/1.jpg"
        ));
    }

    #[test]
    fn parses_multiline_mercado_livre_card_text() {
        let raw = "MAIS VENDIDO\nSSD Kingston NV3 500GB\n4.9 | +10mil vendidos\nR$\n1.379\nR$\n699\n,\n90\n49% OFF\n18x \nR$\n46\n,\n26\nCompartilhar";
        let parsed = parse_card_text(raw);
        assert_eq!(parsed.card_badge.as_deref(), Some("Mais vendido"));
        assert_eq!(parsed.marketplace_rating.as_deref(), Some("4,9 / 5"));
        assert_eq!(
            parsed.marketplace_sales_label.as_deref(),
            Some("+10 mil vendidos no ML")
        );
        assert_eq!(parsed.reference_price_cents, Some(137_900));
        assert_eq!(parsed.card_price_cents, Some(69_990));
        assert_eq!(parsed.discount_label.as_deref(), Some("49% OFF"));
        assert_eq!(
            parsed.installment_summary.as_deref(),
            Some("18× de R$ 46,26")
        );
        assert_eq!(parse_commission_bp("5%"), Some(500));
    }

    #[test]
    fn parses_jsonld_variant_and_formats_sigv4_timestamps() {
        let variant_json = serde_json::json!({
            "@type": "Product",
            "name": "LG UltraGear 27\"",
            "sku": "MLB5359373501",
            "productID": "MLB73964684",
            "color": "Preto",
            "additionalProperty": [{"name": "Voltagem", "value": "220V"}],
            "offers": {
                "price": 3970,
                "priceCurrency": "BRL",
                "availability": "https://schema.org/InStock",
                "url": "https://www.mercadolivre.com.br/lg-ultragear/p/MLB73964684",
                "hasMerchantReturnPolicy": {
                    "merchantReturnDays": 30,
                    "returnFees": "https://schema.org/FreeReturn"
                }
            }
        });
        let variant = parse_jsonld_variant(&variant_json).unwrap();
        assert_eq!(variant.variant_product_id, "MLB73964684");
        assert_eq!(variant.voltage.as_deref(), Some("220V"));
        assert_eq!(variant.price_brl_cents, 397_000);
        assert_eq!(variant.return_days, Some(30));
        assert_eq!(variant.return_fees.as_deref(), Some("Grátis"));

        let (amz, stamp) = format_utc_sigv4(1_763_231_280).unwrap();
        assert!(amz.ends_with('Z'));
        assert_eq!(amz.len(), 16);
        assert_eq!(stamp.len(), 8);
    }

    #[test]
    fn formats_telegram_html_caption_commands_and_whatsapp_assisted_draft() {
        let item = TelegramOfferQueueItem {
            delivery_id: "tg-del-1".into(),
            product_id: "MLB39410080".into(),
            offer_id: "offer-ml-MLB39410080".into(),
            channel_id: "@procurali".into(),
            title: "SSD Kingston <NV3> 1TB & PCIe 4.0 \"Gen4\" ".repeat(8),
            brand: "Kingston".into(),
            model: Some("SNV3S/1000G".into()),
            price_brl_cents: 44_990,
            reference_price_cents: Some(59_900),
            discount_label: Some("25% OFF".into()),
            share_link: "https://meli.la/1iWQZbJ".into(),
            share_id: "1PBCGF-SMN5".into(),
            image_public_url: Some(
                "https://media.procurali.com.br/procurali-media/ml/MLB39410080/hero.jpg".into(),
            ),
            image_object_key: Some("ml/MLB39410080/hero.jpg".into()),
            message_text: "💳 10x sem juros\n🏪 Loja oficial\n⚠️ Confira slot M.2".into(),
            status: TelegramDeliveryStatus::Queued,
            external_message_id: None,
            created_at_ms: 1_763_231_280_000,
            updated_at_ms: 1_763_231_280_000,
            published_at_ms: None,
        };

        let caption = format_telegram_html_caption_pt_br(&item, Some("https://procurali.com.br"));
        assert!(caption.contains("&lt;NV3&gt;"));
        assert!(caption.contains("&amp;"));
        assert!(caption.contains("OFERTA RELÂMPAGO"));
        assert!(caption.contains("❌ De: <s>R$ 599,00</s>"));
        assert!(caption.contains("✅ <b>POR APENAS: R$ 449,90</b>"));
        assert!(caption.contains("Economia de R$ 149,10"));
        assert!(caption.contains("https://meli.la/1iWQZbJ"));
        assert!(rendered_html_char_count(&caption) <= TELEGRAM_CAPTION_MAX_CHARS);

        let kb = build_telegram_offer_keyboard(&item, Some("https://procurali.com.br"));
        assert_eq!(kb.inline_keyboard.len(), 2);
        assert_eq!(
            kb.inline_keyboard[0][0].text,
            "🔥 PEGAR OFERTA NO MERCADO LIVRE 🛒"
        );
        assert_eq!(kb.inline_keyboard[0][0].url, "https://meli.la/1iWQZbJ");
        assert!(
            kb.inline_keyboard[1][0]
                .url
                .starts_with("https://procurali.com.br/ofertas/")
        );

        assert_eq!(
            TelegramBotCommand::parse("/start@ProcuraliBot"),
            Some(TelegramBotCommand::Start)
        );
        assert_eq!(
            TelegramBotCommand::parse("/buscar@ProcuraliBot ssd kingston"),
            Some(TelegramBotCommand::Search("ssd kingston".into()))
        );
        assert_eq!(TelegramBotCommand::parse("/buscar   "), None);
        assert_eq!(
            TelegramBotCommand::parse("/whatsapp"),
            Some(TelegramBotCommand::WhatsappDigest)
        );

        let (reply, markup) = format_telegram_bot_command_reply_pt_br(
            &TelegramBotCommand::Search("kingston".into()),
            std::slice::from_ref(&item),
            Some("https://procurali.com.br"),
        );
        assert!(reply.contains("https://meli.la/1iWQZbJ"));
        assert!(markup.is_some());

        let pkg = build_whatsapp_assisted_package(
            &[item],
            "@procurali",
            Some("https://procurali.com.br"),
            1_763_231_280_000,
        );
        assert_eq!(pkg.mode, "whatsapp_web_human_assisted_no_meta_api");
        assert_eq!(pkg.draft_count, 1);
        assert!(pkg.drafts[0].whatsapp_text.contains("~De R$ 599,00~"));
        assert!(pkg.drafts[0].whatsapp_text.contains("```1PBCGF-SMN5```"));
        assert!(
            pkg.drafts[0]
                .whatsapp_text
                .contains("https://meli.la/1iWQZbJ")
        );
        assert!(pkg.drafts[0].whatsapp_text.contains("*R$ 449,90*"));
        assert!(pkg.drafts[0].whatsapp_text.contains("*Kingston*"));
        assert!(!pkg.drafts[0].whatsapp_text.contains("🔥 * "));
    }

    #[test]
    fn computes_cloobe_taxonomy_price_intelligence_sparkline_and_alerts() {
        let (sub_slug, sub_name) = classify_product_subcategory(
            "Monitor Gamer LG UltraGear 27 IPS 200Hz",
            "hardware-e-perifericos",
            &[],
        );
        assert_eq!(sub_slug, "monitores");
        assert_eq!(sub_name, "Monitores");
        assert_eq!(parse_rating_milli(Some("4,9 / 5")), Some(4900));
        assert_eq!(
            parse_sales_volume_estimate(Some("+10 mil vendidos no ML")),
            10_000
        );
        assert_eq!(compute_discount_pct(69_990, Some(137_900), None), 49);

        let obs = vec![
            SiteCatalogPriceObservation {
                id: "o3".into(),
                offer_id: "of1".into(),
                variant_id: "v1".into(),
                seller_name: "Oficial".into(),
                payment_basis: "Pix".into(),
                freight_note: "Grátis".into(),
                price_brl_cents: 69_990,
                observed_at: "2026-11-16T12:00:00Z".into(),
                evidence_id: "ev1".into(),
            },
            SiteCatalogPriceObservation {
                id: "o2".into(),
                offer_id: "of1".into(),
                variant_id: "v1".into(),
                seller_name: "Oficial".into(),
                payment_basis: "Pix".into(),
                freight_note: "Grátis".into(),
                price_brl_cents: 89_900,
                observed_at: "2026-11-10T12:00:00Z".into(),
                evidence_id: "ev1".into(),
            },
            SiteCatalogPriceObservation {
                id: "o1".into(),
                offer_id: "of1".into(),
                variant_id: "v1".into(),
                seller_name: "Oficial".into(),
                payment_basis: "Pix".into(),
                freight_note: "Grátis".into(),
                price_brl_cents: 94_900,
                observed_at: "2026-11-01T12:00:00Z".into(),
                evidence_id: "ev1".into(),
            },
        ];

        let intel = compute_price_intelligence(
            69_990,
            Some(137_900),
            Some("49% OFF"),
            1_763_252_000_000,
            &obs,
        );
        assert_eq!(intel.window_days, 30);
        assert_eq!(intel.lowest_price_30d_cents, 69_990);
        assert_eq!(intel.highest_price_30d_cents, 137_900);
        assert_eq!(intel.price_level, PriceLevelKind::LowestPrice);
        assert_eq!(intel.price_level_label, "Menor preço");
        assert!(intel.is_price_drop);
        assert!(
            intel
                .verified_at_display
                .starts_with("Preços verificados em ")
        );
        assert!(intel.svg_sparkline.contains("<svg viewBox=\"0 0 300 80\""));
        assert!(intel.svg_sparkline.contains("<polyline"));

        let ok_alert = PriceAlertActionRequest {
            action: "criar-alerta".into(),
            subscriber_id: "sub_01".into(),
            product_id: Some("MLB39410080".into()),
            target_price_cents: Some(65_000),
        };
        ok_alert.validates().unwrap();

        let bad_beacon = ExposureBeaconRequest {
            route: "/categorias/hardware/".into(),
            events: vec![],
            timestamp_ms: 1_763_252_000_000,
        };
        assert!(bad_beacon.validates().is_err());
    }
}
