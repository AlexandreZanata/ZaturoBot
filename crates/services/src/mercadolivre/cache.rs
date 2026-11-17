use crate::Result;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU64, Ordering},
    },
};
use zaturo_core::{
    CatalogTaxonomyResponse, CategoryTaxonomyNode, CommerceReferenceCatalog, CommerceReferenceItem,
    FacetBucketCount, KeywordClusterNode, PriceAlertSubscription, SiteCatalogSnapshot,
    build_commerce_reference_item,
};

#[derive(Debug, Default)]
pub(super) struct ProductTelemetryStats {
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
    pub(super) generation: AtomicU64,
    pub(super) index: RwLock<Option<Arc<CompiledCommerceIndex>>>,
    pub(super) telemetry: RwLock<BTreeMap<String, ProductTelemetryStats>>,
    pub(super) price_alerts: RwLock<Vec<PriceAlertSubscription>>,
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
