use super::*;
use serde::{Deserialize, Serialize};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_price_cents: Option<i64>,
    #[serde(default)]
    pub price_delta_cents: i64,
    #[serde(default)]
    pub price_delta_pct: i16,
    #[serde(default)]
    pub observation_count: usize,
    #[serde(default)]
    pub price_trend: PriceTrendKind,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub first_observed_at_display: String,
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
pub struct CommerceReferenceItem {
    pub product_id: String,
    pub slug: String,
    pub title: String,
    pub brand: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub exact_variant: String,
    pub category_slug: String,
    pub subcategory_slug: String,
    pub seller_name: String,
    pub state: String,
    pub current_price_cents: i64,
    pub pix_price_cents: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_price_cents: Option<i64>,
    pub discount_pct: u8,
    pub lowest_price_30d_cents: i64,
    pub average_price_30d_cents: i64,
    pub highest_price_30d_cents: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_price_cents: Option<i64>,
    pub price_delta_cents: i64,
    pub price_delta_pct: i16,
    pub observation_count: usize,
    pub price_trend: PriceTrendKind,
    pub price_level: PriceLevelKind,
    pub price_level_label: String,
    pub observed_at: String,
    pub observed_at_ms: i64,
    pub official_affiliate_url: String,
    pub short_summary: String,
    #[serde(default)]
    pub telegram_published_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telegram_last_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommerceReferenceCatalog {
    pub generated_at: String,
    pub cache_generation: u64,
    pub total_products: usize,
    pub items: Vec<CommerceReferenceItem>,
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
