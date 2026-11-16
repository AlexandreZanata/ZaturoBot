//! Transactional Mercado Livre product ingestion, static-site catalog projection,
//! Cloobe-inspired in-memory inverted-index hot caching, faceted search, telemetry, price alerts,
//! and Telegram delivery queue orchestration.

use crate::{Engine, Error, Result};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};
use uuid::Uuid;
use zaturo_core::{
    BreadcrumbItem, CatalogFacetsSummary, CatalogSearchQuery, CatalogSearchResponse,
    CatalogTaxonomyResponse, CategoryTaxonomyNode, CommerceReferenceCatalog, CommerceReferenceItem,
    ExposureBeaconRequest, ExposureBeaconResponse, FacetBucketCount, FeedItem, FeedKind,
    KeywordClusterNode, MercadoLivreIngestBatchRequest, MercadoLivreIngestReport,
    PriceAlertActionRequest, PriceAlertResponse, PriceAlertSubscription, PriceLevelKind,
    ProductDetailResponse, SiteCatalogOffer, SiteCatalogSnapshot, TelegramDeliveryAck,
    TelegramDeliveryStatus, TelegramOfferQueueItem, TriggeredPriceAlert,
    build_commerce_reference_item, render_offer_cards_html_fragment, valid_sha256,
    valid_telegram_channel, validate_ml_product_id,
};

const MAX_PRICE_ALERTS: usize = 4096;

#[derive(Debug, Default)]
pub(crate) struct ProductTelemetryStats {
    pub impressions: u64,
    pub clicks: u64,
}

#[derive(Debug, Clone)]
pub struct CompiledCommerceIndex {
    pub generation: u64,
    pub etag: String,
    pub reference_etag: String,
    pub snapshot: Arc<SiteCatalogSnapshot>,
    pub snapshot_json: Arc<String>,
    pub taxonomy: Arc<CatalogTaxonomyResponse>,
    pub taxonomy_json: Arc<String>,
    pub reference_catalog: Arc<CommerceReferenceCatalog>,
    pub reference_catalog_json: Arc<String>,
    pub by_product_id: HashMap<String, usize>,
    pub by_slug: HashMap<String, usize>,
    pub search_texts: Vec<String>,
    pub inverted_tokens: HashMap<String, Vec<usize>>,
    pub by_relevance: Vec<usize>,
    pub by_price_asc: Vec<usize>,
    pub by_price_desc: Vec<usize>,
    pub by_discount_desc: Vec<usize>,
    pub by_sales_desc: Vec<usize>,
    pub by_rating_desc: Vec<usize>,
}

#[derive(Debug, Default)]
pub(crate) struct CommerceHotCache {
    generation: AtomicU64,
    index: RwLock<Option<Arc<CompiledCommerceIndex>>>,
    telemetry: RwLock<BTreeMap<String, ProductTelemetryStats>>,
    price_alerts: RwLock<Vec<PriceAlertSubscription>>,
}

impl CommerceHotCache {
    pub fn invalidate(&self) {
        if let Ok(mut guard) = self.index.write() {
            *guard = None;
        }
    }

    pub fn build_and_store(
        &self,
        snapshot: SiteCatalogSnapshot,
        telegram_stats: &HashMap<String, (u32, Option<String>)>,
    ) -> Result<Arc<CompiledCommerceIndex>> {
        let generation_num = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        let offers = &snapshot.offers;

        let mut by_product_id = HashMap::with_capacity(offers.len());
        let mut by_slug = HashMap::with_capacity(offers.len().saturating_mul(2));
        let mut search_texts = Vec::with_capacity(offers.len());
        let mut inverted_tokens: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, offer) in offers.iter().enumerate() {
            by_product_id.insert(offer.product_id.clone(), idx);
            by_slug.insert(offer.slug.to_ascii_lowercase(), idx);
            by_slug.insert(
                format!("{}-mercadolivre-{}", offer.slug, offer.product_id).to_ascii_lowercase(),
                idx,
            );
            let haystack = format!(
                "{} {} {} {} {} {}",
                offer.title,
                offer.brand,
                offer.model.as_deref().unwrap_or(""),
                offer.subcategory_slug,
                offer.product_id,
                offer.tags.join(" ")
            )
            .to_ascii_lowercase();

            let mut uniq_tokens = BTreeSet::new();
            for tok in haystack
                .split(|c: char| !c.is_alphanumeric())
                .filter(|t| t.len() >= 2)
            {
                uniq_tokens.insert(tok.to_owned());
            }
            for tok in uniq_tokens {
                inverted_tokens.entry(tok).or_default().push(idx);
            }
            search_texts.push(haystack);
        }

        let all_indices: Vec<usize> = (0..offers.len()).collect();
        let mut by_relevance = all_indices.clone();
        by_relevance.sort_by(|&a, &b| {
            offers[b]
                .discount_pct
                .cmp(&offers[a].discount_pct)
                .then_with(|| {
                    offers[b]
                        .sales_volume_estimate
                        .cmp(&offers[a].sales_volume_estimate)
                })
        });

        let mut by_price_asc = all_indices.clone();
        by_price_asc.sort_by_key(|&i| offers[i].price_brl_cents);

        let mut by_price_desc = by_price_asc.clone();
        by_price_desc.reverse();

        let mut by_discount_desc = all_indices.clone();
        by_discount_desc.sort_by(|&a, &b| {
            offers[b]
                .discount_pct
                .cmp(&offers[a].discount_pct)
                .then_with(|| offers[a].price_brl_cents.cmp(&offers[b].price_brl_cents))
        });

        let mut by_sales_desc = all_indices.clone();
        by_sales_desc.sort_by(|&a, &b| {
            offers[b]
                .sales_volume_estimate
                .cmp(&offers[a].sales_volume_estimate)
        });

        let mut by_rating_desc = all_indices;
        by_rating_desc.sort_by(|&a, &b| {
            offers[b]
                .rating_milli
                .unwrap_or(0)
                .cmp(&offers[a].rating_milli.unwrap_or(0))
        });

        // Build Cloobe-style 2-level category + subcategory + keyword cluster taxonomy tree.
        let mut sub_counts_by_cat: HashMap<&str, BTreeMap<(String, String), usize>> =
            HashMap::new();
        let mut cat_offer_counts: HashMap<&str, usize> = HashMap::new();
        let mut cluster_counts: BTreeMap<String, usize> = BTreeMap::new();
        for offer in offers {
            *cat_offer_counts
                .entry(offer.category_slug.as_str())
                .or_default() += 1;
            *sub_counts_by_cat
                .entry(offer.category_slug.as_str())
                .or_default()
                .entry((
                    offer.subcategory_slug.clone(),
                    offer.subcategory_name.clone(),
                ))
                .or_default() += 1;
            for tag in &offer.tags {
                *cluster_counts.entry(tag.clone()).or_default() += 1;
            }
        }
        let category_nodes = snapshot
            .categories
            .iter()
            .map(|cat| {
                let subcategories = sub_counts_by_cat
                    .get(cat.slug.as_str())
                    .map(|m| {
                        m.iter()
                            .map(|((s, label), count)| FacetBucketCount {
                                key: s.clone(),
                                label: label.clone(),
                                count: *count,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                CategoryTaxonomyNode {
                    slug: cat.slug.clone(),
                    name: cat.name.clone(),
                    description: cat.description.clone(),
                    offer_count: cat_offer_counts
                        .get(cat.slug.as_str())
                        .copied()
                        .unwrap_or(0),
                    subcategories,
                }
            })
            .collect();
        let keyword_clusters = cluster_counts
            .into_iter()
            .map(|(slug, count)| KeywordClusterNode {
                label: slug.replace('-', " "),
                slug,
                offer_count: count,
            })
            .collect();

        let taxonomy = Arc::new(CatalogTaxonomyResponse {
            generated_at: snapshot.generated_at.clone(),
            cache_generation: generation_num,
            categories: category_nodes,
            keyword_clusters,
        });
        let reference_items: Vec<CommerceReferenceItem> = offers
            .iter()
            .map(|offer| {
                let (pub_count, last_status) = telegram_stats
                    .get(&offer.product_id)
                    .cloned()
                    .unwrap_or((0, None));
                build_commerce_reference_item(offer, pub_count, last_status)
            })
            .collect();
        let reference_catalog = Arc::new(CommerceReferenceCatalog {
            generated_at: snapshot.generated_at.clone(),
            cache_generation: generation_num,
            total_products: reference_items.len(),
            items: reference_items,
        });
        let snapshot_json =
            Arc::new(serde_json::to_string(&snapshot).map_err(zaturo_db::Error::Json)?);
        let taxonomy_json =
            Arc::new(serde_json::to_string(&*taxonomy).map_err(zaturo_db::Error::Json)?);
        let reference_catalog_json =
            Arc::new(serde_json::to_string(&*reference_catalog).map_err(zaturo_db::Error::Json)?);
        let etag = format!("\"ml-gen-{generation_num}-{}\"", snapshot_json.len());
        let reference_etag = format!(
            "\"ref-gen-{generation_num}-{}\"",
            reference_catalog_json.len()
        );
        let compiled = Arc::new(CompiledCommerceIndex {
            generation: generation_num,
            etag,
            reference_etag,
            snapshot: Arc::new(snapshot),
            snapshot_json,
            taxonomy,
            taxonomy_json,
            reference_catalog,
            reference_catalog_json,
            by_product_id,
            by_slug,
            search_texts,
            inverted_tokens,
            by_relevance,
            by_price_asc,
            by_price_desc,
            by_discount_desc,
            by_sales_desc,
            by_rating_desc,
        });

        if let Ok(mut guard) = self.index.write() {
            *guard = Some(compiled.clone());
        }
        Ok(compiled)
    }
}

fn valid_idempotency_key(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 || !value.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(Error::Invalid(
            "idempotency_key must be 1..=128 ASCII graphic characters".into(),
        ));
    }
    Ok(())
}

impl Engine {
    /// Validate and atomically store a Mercado Livre ingest batch with idempotency semantics.
    pub fn ingest_mercadolivre_batch(
        &self,
        request: MercadoLivreIngestBatchRequest,
        idempotency_key: &str,
        payload_sha256: &str,
        now_ms: i64,
    ) -> Result<MercadoLivreIngestReport> {
        valid_idempotency_key(idempotency_key)?;
        if !valid_sha256(payload_sha256) {
            return Err(Error::Invalid(
                "payload_sha256 must be a 64-character lowercase hex digest".into(),
            ));
        }
        if now_ms <= 0 {
            return Err(Error::Invalid("now_ms must be positive".into()));
        }
        request.validates().map_err(Error::Invalid)?;

        let report = self.store.transaction(|repo| {
            if let Some((existing_sha, existing_report)) =
                repo.ml_batch_by_idempotency_key(idempotency_key)?
            {
                return if existing_sha == payload_sha256 {
                    Ok(existing_report)
                } else {
                    Err(Error::Conflict(
                        "mercadolivre ingest idempotency key reused with different payload".into(),
                    ))
                };
            }

            let batch_id = Uuid::new_v4().to_string();
            let report = repo.save_ml_ingest_batch(
                &batch_id,
                idempotency_key,
                payload_sha256,
                &request,
                now_ms,
            )?;

            let feed_item = FeedItem {
                id: Uuid::new_v4().to_string(),
                created_at: now_ms,
                kind: FeedKind::Service,
                role: "system".into(),
                text: format!(
                    "Mercado Livre batch {} ingested ({} products, {} images, {} Telegram offers awaiting approval)",
                    report.batch_id,
                    report.product_count,
                    report.image_count,
                    report.telegram_awaiting_approval_count
                ),
                task_id: None,
                artifact_id: None,
                sensitive: false,
                capture: None,
            };
            repo.insert_feed(&feed_item, &format!("ml-ingest:{}", report.batch_id))?;
            Ok(report)
        })?;
        self.commerce_cache.invalidate();
        Ok(report)
    }

    /// Build the normalized Procurali static-site catalog snapshot from relational storage.
    pub fn mercadolivre_site_catalog(&self, now_ms: i64) -> Result<SiteCatalogSnapshot> {
        let compiled = self.mercadolivre_site_catalog_cached(now_ms)?;
        Ok((*compiled.snapshot).clone())
    }

    /// Returns an already-compiled hot cache snapshot without touching SQLite or blocking threads.
    pub fn mercadolivre_site_catalog_warm(&self) -> Option<Arc<CompiledCommerceIndex>> {
        self.commerce_cache
            .index
            .read()
            .ok()
            .and_then(|guard| guard.clone())
    }

    /// Serves the precompiled zero-copy `Arc<CompiledCommerceIndex>` on warm reads, avoiding
    /// repeated SQLite scans and serialization overhead under high concurrent traffic.
    pub fn mercadolivre_site_catalog_cached(
        &self,
        now_ms: i64,
    ) -> Result<Arc<CompiledCommerceIndex>> {
        if now_ms <= 0 {
            return Err(Error::Invalid("now_ms must be positive".into()));
        }
        if let Some(cached) = self.mercadolivre_site_catalog_warm() {
            return Ok(cached);
        }
        let (fresh, telegram_stats) = self.store.read(|repo| -> Result<_> {
            let catalog = repo.ml_site_catalog(now_ms)?;
            let tg_stats = repo.ml_telegram_product_stats()?;
            Ok((catalog, tg_stats))
        })?;
        self.commerce_cache.build_and_store(fresh, &telegram_stats)
    }

    pub fn mercadolivre_reference_catalog(
        &self,
        now_ms: i64,
    ) -> Result<Arc<CompiledCommerceIndex>> {
        self.mercadolivre_site_catalog_cached(now_ms)
    }

    /// Returns the Cloobe-style 2-level taxonomy tree and keyword clusters from the warm index.
    pub fn mercadolivre_taxonomy(&self, now_ms: i64) -> Result<Arc<CompiledCommerceIndex>> {
        self.mercadolivre_site_catalog_cached(now_ms)
    }

    /// High-throughput Cloobe-style faceted search & pagination over the precompiled warm index.
    pub fn mercadolivre_search(
        &self,
        query: &CatalogSearchQuery,
        now_ms: i64,
    ) -> Result<CatalogSearchResponse> {
        query.validate().map_err(Error::Invalid)?;
        let index = self.mercadolivre_site_catalog_cached(now_ms)?;
        let offers = &index.snapshot.offers;

        let base_order: &[usize] = match query.ordem.as_deref() {
            Some("preco-asc") => &index.by_price_asc,
            Some("preco-desc") => &index.by_price_desc,
            Some("desconto") => &index.by_discount_desc,
            Some("mais-vendidos") => &index.by_sales_desc,
            Some("melhor-avaliados") => &index.by_rating_desc,
            _ => &index.by_relevance,
        };

        let query_tokens: Vec<String> = query
            .q
            .as_deref()
            .unwrap_or("")
            .to_ascii_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        // Fast O(postings) intersection when all tokens exist in the inverted index.
        let token_candidates: Option<std::collections::HashSet<usize>> = if !query_tokens.is_empty()
            && query_tokens
                .iter()
                .all(|tok| index.inverted_tokens.contains_key(tok))
        {
            let mut lists: Vec<&Vec<usize>> = query_tokens
                .iter()
                .filter_map(|tok| index.inverted_tokens.get(tok))
                .collect();
            lists.sort_by_key(|v| v.len());
            let mut acc: std::collections::HashSet<usize> = lists[0].iter().copied().collect();
            for list in &lists[1..] {
                acc.retain(|id| list.binary_search(id).is_ok());
            }
            Some(acc)
        } else {
            None
        };

        let min_rating_milli: Option<u16> = query.avaliacao.as_deref().and_then(|s| {
            let clean = s.trim().replace(',', ".");
            clean
                .parse::<f64>()
                .ok()
                .map(|f| (f * 1000.0).round() as u16)
        });
        let cat_filter = query
            .categoria
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "todas");
        let sub_filter = query
            .subcategoria
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "todas");
        let store_filter = query
            .loja
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "todas");

        let mut matched = Vec::new();
        for &idx in base_order {
            let offer = &offers[idx];
            if offer.state != "approved" {
                continue;
            }
            if let Some(cands) = &token_candidates {
                if !cands.contains(&idx) {
                    continue;
                }
            } else if !query_tokens.is_empty() {
                let hay = &index.search_texts[idx];
                if !query_tokens.iter().all(|tok| hay.contains(tok.as_str())) {
                    continue;
                }
            }
            if let Some(cat) = cat_filter
                && !offer.category_slug.eq_ignore_ascii_case(cat)
            {
                continue;
            }
            if let Some(sub) = sub_filter
                && !offer.subcategory_slug.eq_ignore_ascii_case(sub)
            {
                continue;
            }
            if let Some(store) = store_filter {
                let s = store.to_ascii_lowercase();
                if !(s == "mercadolivre" || s == "mercado-livre" || s == "ml") {
                    continue;
                }
            }
            if let Some(min_disc) = query.desconto
                && offer.discount_pct < min_disc
            {
                continue;
            }
            if let Some(min_r) = min_rating_milli
                && offer.rating_milli.unwrap_or(0) < min_r
            {
                continue;
            }
            if let Some(price_band) = query.preco.as_deref() {
                let p = offer.price_brl_cents;
                let ok = match price_band {
                    "ate-50" => p <= 5_000,
                    "50-100" => (5_001..=10_000).contains(&p),
                    "100-250" => (10_001..=25_000).contains(&p),
                    "250-500" => (25_001..=50_000).contains(&p),
                    "acima-250" => p > 25_000,
                    "acima-500" => p > 50_000,
                    _ => true,
                };
                if !ok {
                    continue;
                }
            }
            if let Some(col) = query.colecao.as_deref() {
                let ok = match col {
                    "baixou-de-preco" => offer
                        .price_intelligence
                        .as_ref()
                        .is_some_and(|i| i.is_price_drop),
                    "menor-preco" => offer
                        .price_intelligence
                        .as_ref()
                        .is_some_and(|i| i.price_level == PriceLevelKind::LowestPrice),
                    "mais-vendidos" => {
                        offer.card_badge.is_some() || offer.sales_volume_estimate >= 1000
                    }
                    _ => true,
                };
                if !ok {
                    continue;
                }
            }
            matched.push(idx);
        }

        let facets = build_facets_summary(&matched, offers);
        let per_page = query.por_pagina.unwrap_or(20).clamp(1, 50);
        let page = query.pagina.unwrap_or(1).clamp(1, 100);
        let total_matches = matched.len();
        let total_pages =
            u32::try_from(total_matches.div_ceil(per_page as usize).max(1)).unwrap_or(1);
        let start = usize::try_from((page - 1).saturating_mul(per_page)).unwrap_or(usize::MAX);
        let end = start.saturating_add(per_page as usize).min(total_matches);
        let items: Vec<SiteCatalogOffer> = if start < total_matches {
            matched[start..end]
                .iter()
                .map(|&i| offers[i].clone())
                .collect()
        } else {
            Vec::new()
        };
        let html_fragment = if query.formato.as_deref() == Some("html-fragment") {
            Some(render_offer_cards_html_fragment(&items, start))
        } else {
            None
        };

        Ok(CatalogSearchResponse {
            total_matches,
            page,
            per_page,
            total_pages,
            cache_generation: index.generation,
            facets,
            items,
            html_fragment,
        })
    }

    /// Resolves a PDP payload with similar offers, breadcrumbs, alert count, and telemetry stats.
    pub fn mercadolivre_product_detail(
        &self,
        id_or_slug: &str,
        now_ms: i64,
    ) -> Result<ProductDetailResponse> {
        let key = id_or_slug.trim();
        if key.is_empty() || key.len() > 180 || key.contains('\0') {
            return Err(Error::Invalid("invalid product id or slug".into()));
        }
        let index = self.mercadolivre_site_catalog_cached(now_ms)?;
        let offers = &index.snapshot.offers;
        let idx = resolve_offer_index(&index, key).ok_or(Error::NotFound)?;
        let offer = offers[idx].clone();

        let similar_offers = offer
            .similar_offer_ids
            .iter()
            .filter_map(|pid| index.by_product_id.get(pid).map(|&i| offers[i].clone()))
            .collect();

        let breadcrumb = vec![
            BreadcrumbItem {
                label: "Início".into(),
                path: "/".into(),
            },
            BreadcrumbItem {
                label: offer.category_slug.replace('-', " "),
                path: format!("/categorias/{}/", offer.category_slug),
            },
            BreadcrumbItem {
                label: offer.subcategory_name.clone(),
                path: format!(
                    "/categorias/{}/{}/",
                    offer.category_slug, offer.subcategory_slug
                ),
            },
        ];

        let (impressions, clicks) = self
            .commerce_cache
            .telemetry
            .read()
            .ok()
            .and_then(|map| {
                map.get(&offer.product_id)
                    .map(|s| (s.impressions, s.clicks))
            })
            .unwrap_or((0, 0));
        let alert_count = self
            .commerce_cache
            .price_alerts
            .read()
            .ok()
            .map(|list| {
                list.iter()
                    .filter(|a| a.product_id == offer.product_id)
                    .count()
            })
            .unwrap_or(0);

        Ok(ProductDetailResponse {
            cache_generation: index.generation,
            offer,
            breadcrumb,
            similar_offers,
            alert_count,
            impressions,
            clicks,
        })
    }

    /// Fast `/ir/mercadolivre/{id}/` affiliate redirect resolution & click tracking from warm cache.
    pub fn mercadolivre_resolve_redirect(&self, product_id: &str, now_ms: i64) -> Result<String> {
        let index = self.mercadolivre_site_catalog_cached(now_ms)?;
        let idx = resolve_offer_index(&index, product_id.trim()).ok_or(Error::NotFound)?;
        let offer = &index.snapshot.offers[idx];
        if let Ok(mut guard) = self.commerce_cache.telemetry.write() {
            guard.entry(offer.product_id.clone()).or_default().clicks += 1;
        }
        Ok(offer.official_affiliate_url.clone())
    }

    /// Non-blocking `/api/exposicao/` viewport impression recorder.
    pub fn mercadolivre_record_exposure(
        &self,
        req: ExposureBeaconRequest,
    ) -> Result<ExposureBeaconResponse> {
        req.validates().map_err(Error::Invalid)?;
        let accepted = req.events.len();
        if let Ok(mut guard) = self.commerce_cache.telemetry.write() {
            for ev in &req.events {
                guard.entry(ev.product_id.clone()).or_default().impressions += 1;
            }
        }
        Ok(ExposureBeaconResponse {
            accepted,
            route: req.route,
        })
    }

    /// Cloobe `/api/notificacoes/` alert management & target-price trigger evaluation.
    pub fn mercadolivre_handle_price_alert(
        &self,
        req: PriceAlertActionRequest,
        now_ms: i64,
    ) -> Result<PriceAlertResponse> {
        if now_ms <= 0 {
            return Err(Error::Invalid("now_ms must be positive".into()));
        }
        req.validates().map_err(Error::Invalid)?;
        let index = self.mercadolivre_site_catalog_cached(now_ms)?;
        let offers = &index.snapshot.offers;

        match req.action.as_str() {
            "criar-alerta" => {
                let pid = req.product_id.as_deref().ok_or_else(|| {
                    Error::Invalid("product_id is required for criar-alerta".into())
                })?;
                if !index.by_product_id.contains_key(pid) {
                    return Err(Error::NotFound);
                }
                let target = req.target_price_cents.unwrap_or(0);
                let mut guard = self
                    .commerce_cache
                    .price_alerts
                    .write()
                    .map_err(|_| Error::Storage(zaturo_db::Error::Poisoned))?;
                if let Some(existing) = guard
                    .iter_mut()
                    .find(|a| a.subscriber_id == req.subscriber_id && a.product_id == pid)
                {
                    existing.target_price_cents = target;
                    existing.created_at_ms = now_ms;
                } else {
                    if guard.len() >= MAX_PRICE_ALERTS {
                        return Err(Error::Capacity);
                    }
                    guard.push(PriceAlertSubscription {
                        subscriber_id: req.subscriber_id.clone(),
                        product_id: pid.to_owned(),
                        target_price_cents: target,
                        created_at_ms: now_ms,
                    });
                }
                Ok(build_alert_response(
                    &req.action,
                    &req.subscriber_id,
                    &guard,
                    &index,
                    offers,
                ))
            }
            "remover-alerta" => {
                let pid = req.product_id.as_deref().ok_or_else(|| {
                    Error::Invalid("product_id is required for remover-alerta".into())
                })?;
                let mut guard = self
                    .commerce_cache
                    .price_alerts
                    .write()
                    .map_err(|_| Error::Storage(zaturo_db::Error::Poisoned))?;
                guard.retain(|a| !(a.subscriber_id == req.subscriber_id && a.product_id == pid));
                Ok(build_alert_response(
                    &req.action,
                    &req.subscriber_id,
                    &guard,
                    &index,
                    offers,
                ))
            }
            _ => {
                let guard = self
                    .commerce_cache
                    .price_alerts
                    .read()
                    .map_err(|_| Error::Storage(zaturo_db::Error::Poisoned))?;
                Ok(build_alert_response(
                    &req.action,
                    &req.subscriber_id,
                    &guard,
                    &index,
                    offers,
                ))
            }
        }
    }

    /// List bounded Telegram delivery queue items filtered by optional channel and status.
    /// Listing `queued` deliveries only returns current offers still inside their freshness
    /// window, so publishers never see a superseded or stale price.
    pub fn mercadolivre_telegram_queue(
        &self,
        channel_id: Option<&str>,
        status: Option<TelegramDeliveryStatus>,
        limit: u32,
        now_ms: i64,
    ) -> Result<Vec<TelegramOfferQueueItem>> {
        if let Some(ch) = channel_id
            && !valid_telegram_channel(ch)
        {
            return Err(Error::Invalid("invalid Telegram channel_id".into()));
        }
        self.page(limit, 0)?;
        let fresh_at = (status == Some(TelegramDeliveryStatus::Queued)).then_some(now_ms);
        self.store
            .read(|repo| Ok(repo.ml_telegram_queue(channel_id, status, fresh_at, limit)?))
    }

    /// Owner approval for publishing one product's current offer on one Telegram channel.
    /// Approval is idempotent for an already approved, claimed, or published offer and is
    /// refused for a stale offer, which must be re-observed before it can be approved.
    pub fn approve_mercadolivre_telegram_delivery(
        &self,
        product_id: &str,
        channel_id: &str,
        now_ms: i64,
    ) -> Result<TelegramOfferQueueItem> {
        if !validate_ml_product_id(product_id) {
            return Err(Error::Invalid("invalid product_id".into()));
        }
        if !valid_telegram_channel(channel_id) {
            return Err(Error::Invalid("invalid Telegram channel_id".into()));
        }
        if now_ms <= 0 {
            return Err(Error::Invalid("now_ms must be positive".into()));
        }
        self.store.transaction(|repo| {
            let item = repo
                .ml_current_telegram_delivery(product_id, channel_id)?
                .ok_or(Error::NotFound)?;
            match item.status {
                TelegramDeliveryStatus::Queued
                | TelegramDeliveryStatus::Sending
                | TelegramDeliveryStatus::Published => Ok(item),
                TelegramDeliveryStatus::Skipped | TelegramDeliveryStatus::Failed => Err(
                    Error::Conflict("telegram delivery is closed; re-ingest the offer".into()),
                ),
                TelegramDeliveryStatus::AwaitingApproval => {
                    if !repo.ml_telegram_delivery_offer_fresh(&item.delivery_id, now_ms)? {
                        return Err(Error::Conflict(
                            "offer evidence is stale; re-ingest before approving".into(),
                        ));
                    }
                    if !repo.set_ml_telegram_delivery_status(
                        &item.delivery_id,
                        TelegramDeliveryStatus::AwaitingApproval,
                        TelegramDeliveryStatus::Queued,
                        None,
                        None,
                        now_ms,
                    )? {
                        return Err(Error::Conflict("telegram delivery changed".into()));
                    }
                    repo.approve_ml_product(product_id, now_ms)?;
                    repo.ml_telegram_delivery(&item.delivery_id)?
                        .ok_or(Error::NotFound)
                }
            }
        })
    }

    /// Acknowledge a Telegram delivery transition requested by a publisher.
    ///
    /// Publishers must claim (`queued -> sending`) before any external send and may then only
    /// finish (`sending -> published|failed`). A claim on a superseded or stale offer closes the
    /// delivery as skipped and returns it, so the caller must send only when the returned status
    /// is `sending`. Repeating a terminal acknowledgement returns the stored delivery unchanged.
    pub fn ack_mercadolivre_telegram_delivery(
        &self,
        delivery_id: &str,
        ack: TelegramDeliveryAck,
        now_ms: i64,
    ) -> Result<TelegramOfferQueueItem> {
        if delivery_id.is_empty()
            || delivery_id.len() > 64
            || !delivery_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':'))
        {
            return Err(Error::Invalid("invalid delivery_id".into()));
        }
        if now_ms <= 0 {
            return Err(Error::Invalid("now_ms must be positive".into()));
        }
        ack.validates().map_err(Error::Invalid)?;

        let item = self.store.transaction(|repo| {
            let current = repo
                .ml_telegram_delivery(delivery_id)?
                .ok_or(Error::NotFound)?;
            if current.status == ack.status && current.status.is_terminal() {
                return Ok(current);
            }
            if !TelegramDeliveryStatus::ack_transition_allowed(current.status, ack.status) {
                return Err(Error::Conflict(format!(
                    "telegram delivery cannot move from {} to {}",
                    current.status.as_str(),
                    ack.status.as_str()
                )));
            }
            let (to, error_code) = if ack.status == TelegramDeliveryStatus::Sending
                && !repo.ml_telegram_delivery_offer_fresh(delivery_id, now_ms)?
            {
                (TelegramDeliveryStatus::Skipped, Some("offer_stale"))
            } else {
                (ack.status, ack.error_code.as_deref())
            };
            if !repo.set_ml_telegram_delivery_status(
                delivery_id,
                current.status,
                to,
                ack.external_message_id.as_deref(),
                error_code,
                now_ms,
            )? {
                return Err(Error::Conflict("telegram delivery changed".into()));
            }
            repo.ml_telegram_delivery(delivery_id)?
                .ok_or(Error::NotFound)
        })?;
        self.commerce_cache.invalidate();
        Ok(item)
    }
}

fn resolve_offer_index(index: &CompiledCommerceIndex, id_or_slug: &str) -> Option<usize> {
    if let Some(&idx) = index.by_product_id.get(id_or_slug) {
        return Some(idx);
    }
    let lower = id_or_slug.to_ascii_lowercase();
    if let Some(&idx) = index.by_slug.get(&lower) {
        return Some(idx);
    }
    if let Some(pos) = id_or_slug.rfind("MLB") {
        let candidate = &id_or_slug[pos..];
        if let Some(&idx) = index.by_product_id.get(candidate) {
            return Some(idx);
        }
    }
    None
}

fn build_facets_summary(matched: &[usize], offers: &[SiteCatalogOffer]) -> CatalogFacetsSummary {
    let mut cats: BTreeMap<String, usize> = BTreeMap::new();
    let mut subs: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut p_0_50 = 0usize;
    let mut p_50_100 = 0usize;
    let mut p_100_250 = 0usize;
    let mut p_250_500 = 0usize;
    let mut p_500_plus = 0usize;
    let mut d_10 = 0usize;
    let mut d_20 = 0usize;
    let mut d_30 = 0usize;
    let mut d_50 = 0usize;

    for &idx in matched {
        let o = &offers[idx];
        *cats.entry(o.category_slug.clone()).or_default() += 1;
        *subs
            .entry((o.subcategory_slug.clone(), o.subcategory_name.clone()))
            .or_default() += 1;
        match o.price_brl_cents {
            ..=5_000 => p_0_50 += 1,
            5_001..=10_000 => p_50_100 += 1,
            10_001..=25_000 => p_100_250 += 1,
            25_001..=50_000 => p_250_500 += 1,
            _ => p_500_plus += 1,
        }
        if o.discount_pct >= 10 {
            d_10 += 1;
        }
        if o.discount_pct >= 20 {
            d_20 += 1;
        }
        if o.discount_pct >= 30 {
            d_30 += 1;
        }
        if o.discount_pct >= 50 {
            d_50 += 1;
        }
    }

    CatalogFacetsSummary {
        categories: cats
            .into_iter()
            .map(|(key, count)| FacetBucketCount {
                label: key.replace('-', " "),
                key,
                count,
            })
            .collect(),
        subcategories: subs
            .into_iter()
            .map(|((key, label), count)| FacetBucketCount { key, label, count })
            .collect(),
        stores: vec![FacetBucketCount {
            key: "mercadolivre".into(),
            label: "Mercado Livre".into(),
            count: matched.len(),
        }],
        price_ranges: vec![
            FacetBucketCount {
                key: "ate-50".into(),
                label: "Até R$ 50".into(),
                count: p_0_50,
            },
            FacetBucketCount {
                key: "50-100".into(),
                label: "R$ 50 a R$ 100".into(),
                count: p_50_100,
            },
            FacetBucketCount {
                key: "100-250".into(),
                label: "R$ 100 a R$ 250".into(),
                count: p_100_250,
            },
            FacetBucketCount {
                key: "250-500".into(),
                label: "R$ 250 a R$ 500".into(),
                count: p_250_500,
            },
            FacetBucketCount {
                key: "acima-500".into(),
                label: "Acima de R$ 500".into(),
                count: p_500_plus,
            },
        ],
        discount_tiers: vec![
            FacetBucketCount {
                key: "10".into(),
                label: "10% OFF ou mais".into(),
                count: d_10,
            },
            FacetBucketCount {
                key: "20".into(),
                label: "20% OFF ou mais".into(),
                count: d_20,
            },
            FacetBucketCount {
                key: "30".into(),
                label: "30% OFF ou mais".into(),
                count: d_30,
            },
            FacetBucketCount {
                key: "50".into(),
                label: "50% OFF ou mais".into(),
                count: d_50,
            },
        ],
    }
}

fn build_alert_response(
    action: &str,
    subscriber_id: &str,
    alerts: &[PriceAlertSubscription],
    index: &CompiledCommerceIndex,
    offers: &[SiteCatalogOffer],
) -> PriceAlertResponse {
    let active_alerts: Vec<PriceAlertSubscription> = alerts
        .iter()
        .filter(|a| a.subscriber_id == subscriber_id)
        .cloned()
        .collect();
    let mut triggered = Vec::new();
    for alert in &active_alerts {
        if let Some(&idx) = index.by_product_id.get(&alert.product_id) {
            let offer = &offers[idx];
            if offer.price_brl_cents <= alert.target_price_cents {
                triggered.push(TriggeredPriceAlert {
                    subscriber_id: alert.subscriber_id.clone(),
                    product_id: alert.product_id.clone(),
                    title: offer.title.clone(),
                    target_price_cents: alert.target_price_cents,
                    current_price_cents: offer.price_brl_cents,
                    share_link: offer.official_affiliate_url.clone(),
                });
            }
        }
    }
    PriceAlertResponse {
        action: action.to_owned(),
        subscriber_id: subscriber_id.to_owned(),
        active_alerts,
        triggered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zaturo_core::{
        FeedQuery, Limits, MercadoLivreIngestBatchRequest, MercadoLivreProductState,
        MercadoLivreSpec, MercadoLivreVariant, MinioProductImage, ValidatedMercadoLivreProduct,
    };

    fn sample_product(product_id: &str, slug: &str) -> ValidatedMercadoLivreProduct {
        ValidatedMercadoLivreProduct {
            product_id: product_id.to_owned(),
            slug: slug.to_owned(),
            title: "SSD Kingston NV3 500GB M.2 2280 NVMe PCIe 4.0".into(),
            brand: "Kingston".into(),
            model: Some("SNV3S/500G".into()),
            exact_variant: "SNV3S/500G · 500 GB · M.2 2280".into(),
            category_slug: "hardware-e-perifericos".into(),
            canonical_url: format!(
                "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/{product_id}"
            ),
            seller_name: "Loja oficial FAMILAR".into(),
            share_link: "https://meli.la/1iWQZbJ".into(),
            share_id: "1PBCGF-SMN5".into(),
            share_text: "Código 1PBCGF-SMN5\nhttps://meli.la/1iWQZbJ".into(),
            commission_label: Some("5%".into()),
            commission_bp: Some(500),
            sponsored: false,
            price_brl_cents: 69_990,
            pix_price_cents: 69_990,
            reference_price_cents: Some(137_900),
            currency: "BRL".into(),
            marketplace_rating: Some("4,9 / 5".into()),
            marketplace_sales_label: Some("+10 mil vendidos no ML".into()),
            card_badge: Some("Mais vendido".into()),
            discount_label: Some("49% OFF".into()),
            installment_summary: Some("18x de R$ 46,26".into()),
            payment_conditions: "R$ 699,90 a vista ou em ate 18x de R$ 46,26".into(),
            short_condition: "18x R$ 46,26 · devolucao gratis 30d".into(),
            freight_status: "Frete calculado no CEP; devolucao gratis 30 dias.".into(),
            availability: "InStock".into(),
            price_valid_until: Some("2026-11-18".into()),
            return_days: Some(30),
            short_description: "M.2 2280 NVMe PCIe 4.0 x4 · leitura ate 5.000 MB/s.".into(),
            long_description: "SSD interno Kingston NV3 500 GB com memoria 3D NAND.".into(),
            one_line_reason: "SSD PCIe 4.0 para notebook e desktop com leitura de 5.000 MB/s."
                .into(),
            for_whom: "Quem precisa atualizar slot M.2 2280 com boa velocidade.".into(),
            supported_benefits: vec!["Leitura ate 5.000 MB/s.".into()],
            limitations: vec!["Nao acompanha dissipador de calor.".into()],
            unknowns: vec!["Serie continua de 7 dias ainda em acumulacao.".into()],
            tags: vec!["ssd".into(), "nvme".into(), "kingston".into()],
            specs: vec![
                MercadoLivreSpec {
                    name: "Marca".into(),
                    value: "Kingston".into(),
                },
                MercadoLivreSpec {
                    name: "Capacidade".into(),
                    value: "500 GB".into(),
                },
            ],
            variants: vec![MercadoLivreVariant {
                variant_product_id: product_id.to_owned(),
                sku: "MLB7607021422".into(),
                name: "SSD Kingston NV3 500GB".into(),
                color: Some("Azul-aco".into()),
                voltage: None,
                price_brl_cents: 69_800,
                currency: "BRL".into(),
                availability: "InStock".into(),
                return_days: Some(30),
                return_fees: Some("Gratis".into()),
                canonical_url: Some(format!(
                    "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/{product_id}"
                )),
                highlights: vec!["M.2 2280".into()],
            }],
            images: vec![MinioProductImage {
                position: 0,
                bucket: "procurali-media".into(),
                object_key: format!("mercadolivre/{product_id}/00-sample.jpg"),
                public_url: format!(
                    "https://media.procurali.com.br/procurali-media/mercadolivre/{product_id}/00-sample.jpg"
                ),
                source_origin_url: Some("https://http2.mlstatic.com/D_NQ_NP_sample.jpg".into()),
                content_type: "image/jpeg".into(),
                sha256_hex: "a".repeat(64),
                size_bytes: 12_345,
                alt_text: "SSD Kingston NV3 500GB".into(),
            }],
            observed_at_ms: 1_763_231_280_000,
            freshness_ttl_minutes: 30,
            evidence_sha256: "b".repeat(64),
            state: MercadoLivreProductState::Approved,
        }
    }

    #[test]
    fn ingests_with_idempotency_conflict_detection_feed_catalog_and_telegram_ack() {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(dir.path().join("hub.db"), Limits::default()).unwrap();

        let mut candidate_product =
            sample_product("MLB39410080", "ssd-kingston-nv3-500gb-mlb39410080");
        candidate_product.state = MercadoLivreProductState::Candidate;
        let req = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "ssd nvme".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: 1_763_231_280_000,
            products: vec![candidate_product],
        };
        let sha_a = "c".repeat(64);
        let sha_b = "d".repeat(64);

        let first = engine
            .ingest_mercadolivre_batch(req.clone(), "batch-idem-1", &sha_a, 1_763_231_290_000)
            .unwrap();
        assert_eq!(first.product_count, 1);
        assert_eq!(first.telegram_queued_count, 0);
        assert_eq!(first.telegram_awaiting_approval_count, 1);

        let replay = engine
            .ingest_mercadolivre_batch(req.clone(), "batch-idem-1", &sha_a, 1_763_231_295_000)
            .unwrap();
        assert_eq!(replay, first);

        let conflict =
            engine.ingest_mercadolivre_batch(req, "batch-idem-1", &sha_b, 1_763_231_299_000);
        assert!(matches!(conflict, Err(Error::Conflict(_))));

        let feed = engine
            .feed(FeedQuery {
                day: "2025-11-15".into(),
                offset_minutes: -240,
                kind: Some(FeedKind::Service),
                q: Some("Mercado Livre batch".into()),
                limit: 20,
                offset: 0,
            })
            .unwrap();
        assert_eq!(feed.items.len(), 1);
        assert!(!feed.items[0].sensitive);

        let catalog = engine.mercadolivre_site_catalog(1_763_231_300_000).unwrap();
        assert_eq!(catalog.offers.len(), 1);
        assert_eq!(
            catalog.offers[0].official_affiliate_url,
            "https://meli.la/1iWQZbJ"
        );

        let now = 1_763_231_350_000;
        let queued = |engine: &Engine| {
            engine
                .mercadolivre_telegram_queue(
                    Some("@procurali"),
                    Some(TelegramDeliveryStatus::Queued),
                    20,
                    now,
                )
                .unwrap()
        };
        assert!(
            queued(&engine).is_empty(),
            "ingest must not approve publication"
        );

        let ack = |status| TelegramDeliveryAck {
            status,
            external_message_id: None,
            error_code: None,
        };
        let awaiting = engine
            .mercadolivre_telegram_queue(
                Some("@procurali"),
                Some(TelegramDeliveryStatus::AwaitingApproval),
                20,
                now,
            )
            .unwrap();
        assert_eq!(awaiting.len(), 1);
        let id = awaiting[0].delivery_id.clone();
        for status in [
            TelegramDeliveryStatus::Queued,
            TelegramDeliveryStatus::Sending,
        ] {
            assert!(matches!(
                engine.ack_mercadolivre_telegram_delivery(&id, ack(status), now),
                Err(Error::Conflict(_))
            ));
        }

        let approved = engine
            .approve_mercadolivre_telegram_delivery("MLB39410080", "@procurali", now)
            .unwrap();
        assert_eq!(approved.status, TelegramDeliveryStatus::Queued);
        assert_eq!(queued(&engine).len(), 1);
        assert_eq!(
            engine
                .approve_mercadolivre_telegram_delivery("MLB39410080", "@procurali", now)
                .unwrap(),
            approved
        );

        assert!(matches!(
            engine.ack_mercadolivre_telegram_delivery(
                &id,
                ack(TelegramDeliveryStatus::Published),
                now
            ),
            Err(Error::Conflict(_))
        ));
        let claimed = engine
            .ack_mercadolivre_telegram_delivery(&id, ack(TelegramDeliveryStatus::Sending), now)
            .unwrap();
        assert_eq!(claimed.status, TelegramDeliveryStatus::Sending);
        assert!(queued(&engine).is_empty());
        assert!(matches!(
            engine.ack_mercadolivre_telegram_delivery(
                &id,
                ack(TelegramDeliveryStatus::Sending),
                now
            ),
            Err(Error::Conflict(_))
        ));

        let acked = engine
            .ack_mercadolivre_telegram_delivery(
                &id,
                TelegramDeliveryAck {
                    status: TelegramDeliveryStatus::Published,
                    external_message_id: Some("msg-42".into()),
                    error_code: None,
                },
                now,
            )
            .unwrap();
        assert_eq!(acked.status, TelegramDeliveryStatus::Published);
        assert_eq!(acked.external_message_id.as_deref(), Some("msg-42"));
        assert_eq!(
            engine
                .ack_mercadolivre_telegram_delivery(
                    &id,
                    ack(TelegramDeliveryStatus::Published),
                    now + 1
                )
                .unwrap(),
            acked
        );

        // Cloobe-inspired search, taxonomy, PDP, redirect click tracking, exposure & price alerts:
        let cached = engine
            .mercadolivre_site_catalog_cached(1_763_231_360_000)
            .unwrap();
        assert_eq!(cached.generation, 2);

        let search = engine
            .mercadolivre_search(
                &CatalogSearchQuery {
                    q: Some("kingston".into()),
                    subcategoria: Some("ssd-e-armazenamento".into()),
                    formato: Some("html-fragment".into()),
                    ..Default::default()
                },
                1_763_231_360_000,
            )
            .unwrap();
        assert_eq!(search.total_matches, 1);
        assert!(
            search
                .html_fragment
                .as_deref()
                .unwrap()
                .contains("<article class=\"offer-card\"")
        );

        let alert = engine
            .mercadolivre_handle_price_alert(
                PriceAlertActionRequest {
                    action: "criar-alerta".into(),
                    subscriber_id: "sub_01".into(),
                    product_id: Some("MLB39410080".into()),
                    target_price_cents: Some(70_000),
                },
                1_763_231_365_000,
            )
            .unwrap();
        assert_eq!(alert.active_alerts.len(), 1);
        assert_eq!(alert.triggered.len(), 1);

        let link = engine
            .mercadolivre_resolve_redirect("MLB39410080", 1_763_231_370_000)
            .unwrap();
        assert_eq!(link, "https://meli.la/1iWQZbJ");

        let pdp = engine
            .mercadolivre_product_detail("MLB39410080", 1_763_231_375_000)
            .unwrap();
        assert_eq!(pdp.clicks, 1);
        assert_eq!(pdp.alert_count, 1);
        assert!(pdp.offer.price_intelligence.is_some());

        // Reference catalog warm cache hit reflects Telegram ack, and invalidates + updates on price drop:
        let ref_before = engine
            .mercadolivre_reference_catalog(1_763_231_376_000)
            .unwrap();
        assert_eq!(ref_before.reference_catalog.total_products, 1);
        assert_eq!(
            ref_before.reference_catalog.items[0].telegram_published_count,
            1
        );
        assert_eq!(
            ref_before.reference_catalog.items[0]
                .telegram_last_status
                .as_deref(),
            Some("published")
        );

        let day2_ms = 1_763_231_280_000 + 86_400_000;
        let mut p_drop = sample_product("MLB39410080", "ssd-kingston-nv3-500gb-mlb39410080");
        p_drop.price_brl_cents = 59_990;
        p_drop.pix_price_cents = 59_990;
        p_drop.observed_at_ms = day2_ms;
        let drop_req = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "ssd nvme".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: day2_ms,
            products: vec![p_drop],
        };
        let drop_report = engine
            .ingest_mercadolivre_batch(drop_req, "batch-idem-drop", &"d".repeat(64), day2_ms)
            .unwrap();
        assert_eq!(drop_report.price_drop_count, 1);
        assert_eq!(drop_report.telegram_queued_count, 1);

        let ref_after = engine
            .mercadolivre_reference_catalog(day2_ms + 1000)
            .unwrap();
        assert_ne!(ref_before.reference_etag, ref_after.reference_etag);
        let item = &ref_after.reference_catalog.items[0];
        assert_eq!(item.price_trend, zaturo_core::PriceTrendKind::Down);
        assert_eq!(item.previous_price_cents, Some(69_990));
        assert_eq!(item.price_delta_cents, -10_000);
        assert!(item.short_summary.contains("Queda"));
        assert_eq!(item.telegram_last_status.as_deref(), Some("queued"));
    }

    #[test]
    fn stale_or_superseded_offers_are_never_approved_or_claimed() {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(dir.path().join("hub.db"), Limits::default()).unwrap();
        let request = |captured| {
            let mut p = sample_product("MLB39410080", "ssd-kingston-nv3-500gb-mlb39410080");
            p.state = MercadoLivreProductState::Candidate;
            MercadoLivreIngestBatchRequest {
                worker_id: "worker-ml".into(),
                search_query: "ssd nvme".into(),
                channel_id: "@procurali".into(),
                captured_at_ms: captured,
                products: vec![p],
            }
        };
        let observed = 1_763_231_280_000_i64;
        let expired = observed + 30 * 60_000 + 1;
        engine
            .ingest_mercadolivre_batch(request(observed), "a", &"c".repeat(64), observed + 1)
            .unwrap();
        assert!(matches!(
            engine.approve_mercadolivre_telegram_delivery("MLB39410080", "@procurali", expired),
            Err(Error::Conflict(_))
        ));

        let approved = engine
            .approve_mercadolivre_telegram_delivery("MLB39410080", "@procurali", observed + 2)
            .unwrap();
        let claim = TelegramDeliveryAck {
            status: TelegramDeliveryStatus::Sending,
            external_message_id: None,
            error_code: None,
        };
        let stale = engine
            .ack_mercadolivre_telegram_delivery(&approved.delivery_id, claim.clone(), expired)
            .unwrap();
        assert_eq!(stale.status, TelegramDeliveryStatus::Skipped);

        // A newer observation supersedes an approved but unsent delivery.
        engine
            .ingest_mercadolivre_batch(request(observed), "b", &"d".repeat(64), observed + 3)
            .unwrap();
        let second = engine
            .approve_mercadolivre_telegram_delivery("MLB39410080", "@procurali", observed + 4)
            .unwrap();
        assert_ne!(second.delivery_id, approved.delivery_id);
        engine
            .ingest_mercadolivre_batch(request(observed), "c", &"e".repeat(64), observed + 5)
            .unwrap();
        let superseded =
            engine.ack_mercadolivre_telegram_delivery(&second.delivery_id, claim, observed + 6);
        assert!(matches!(superseded, Err(Error::Conflict(_))));
        assert!(
            engine
                .mercadolivre_telegram_queue(
                    Some("@procurali"),
                    Some(TelegramDeliveryStatus::Queued),
                    20,
                    observed + 6,
                )
                .unwrap()
                .is_empty()
        );
    }
}
