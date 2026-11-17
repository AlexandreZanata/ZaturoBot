use super::CompiledCommerceIndex;
use crate::{Engine, Error, Result};
use std::{collections::BTreeMap, sync::Arc};
use zaturo_core::{
    BreadcrumbItem, CatalogFacetsSummary, CatalogSearchQuery, CatalogSearchResponse,
    FacetBucketCount, PriceLevelKind, ProductDetailResponse, SiteCatalogOffer, SiteCatalogSnapshot,
    render_offer_cards_html_fragment,
};

impl Engine {
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
