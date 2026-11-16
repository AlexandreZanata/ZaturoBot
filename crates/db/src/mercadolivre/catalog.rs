use super::ThirtyDayDedupState;
use crate::{Error, Repository};
use std::collections::{BTreeSet, HashMap};
use zaturo_core::{
    MercadoLivreSpec, MercadoLivreVariant, MinioProductImage, SiteCatalogCategory,
    SiteCatalogEvidenceSource, SiteCatalogGalleryItem, SiteCatalogMediaRight, SiteCatalogOffer,
    SiteCatalogPriceObservation, SiteCatalogSnapshot, enrich_site_catalog_offer,
    format_cuiaba_display, format_cuiaba_short, format_utc_iso_ms,
};

impl Repository<'_> {
    pub fn ml_site_catalog(&self, now_ms: i64) -> Result<SiteCatalogSnapshot, Error> {
        let mut stmt = self.connection.prepare(
            "SELECT
                p.product_id, p.slug, p.title, p.brand, p.model, p.exact_variant,
                p.category_slug, p.canonical_url, p.seller_name, p.marketplace_rating,
                p.marketplace_sales_label, p.card_badge, p.return_days,
                p.short_description, p.long_description, p.one_line_reason, p.for_whom,
                p.supported_benefits_json, p.limitations_json, p.unknowns_json, p.tags_json,
                p.state,
                o.id, o.share_link, o.share_id, o.share_text, o.commission_label,
                o.sponsored, o.price_brl_cents, o.pix_price_cents, o.reference_price_cents,
                o.currency, o.discount_label, o.installment_summary, o.payment_conditions,
                o.short_condition, o.freight_status, o.availability, o.price_valid_until,
                o.observed_at_ms, o.freshness_ttl_minutes, o.evidence_sha256
             FROM ml_products p
             JOIN ml_offers o ON o.product_id = p.product_id AND o.is_current = 1
             WHERE p.state != 'withdrawn'
             ORDER BY o.observed_at_ms DESC, p.product_id ASC",
        )?;

        let mut offers = Vec::new();
        let mut evidence_sources = Vec::new();
        let mut media_rights = Vec::new();
        let mut category_slugs = BTreeSet::new();

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<i64>>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
                row.get::<_, String>(15)?,
                row.get::<_, String>(16)?,
                row.get::<_, String>(17)?,
                row.get::<_, String>(18)?,
                row.get::<_, String>(19)?,
                row.get::<_, String>(20)?,
                row.get::<_, String>(21)?,
                row.get::<_, String>(22)?,
                row.get::<_, String>(23)?,
                row.get::<_, String>(24)?,
                row.get::<_, String>(25)?,
                row.get::<_, Option<String>>(26)?,
                row.get::<_, i64>(27)?,
                row.get::<_, i64>(28)?,
                row.get::<_, i64>(29)?,
                row.get::<_, Option<i64>>(30)?,
                row.get::<_, String>(31)?,
                row.get::<_, Option<String>>(32)?,
                row.get::<_, Option<String>>(33)?,
                row.get::<_, String>(34)?,
                row.get::<_, String>(35)?,
                row.get::<_, String>(36)?,
                row.get::<_, String>(37)?,
                row.get::<_, Option<String>>(38)?,
                row.get::<_, i64>(39)?,
                row.get::<_, i64>(40)?,
                row.get::<_, String>(41)?,
            ))
        })?;

        // Preload related child collections in 4 constant-count batch queries (eliminating N+1 loops).
        let mut images_by_product = self.ml_all_product_images()?;
        let mut specs_by_product = self.ml_all_product_specs()?;
        let mut variants_by_product = self.ml_all_product_variants()?;
        let mut obs_by_product = self.ml_all_recent_price_observations()?;

        for row in rows {
            let (
                product_id,
                slug,
                title,
                brand,
                model,
                exact_variant,
                category_slug,
                canonical_url,
                seller_name,
                marketplace_rating,
                marketplace_sales_label,
                card_badge,
                return_days_raw,
                short_description,
                long_description,
                one_line_reason,
                for_whom,
                benefits_json,
                limitations_json,
                unknowns_json,
                tags_json,
                state,
                _current_offer_id,
                share_link,
                share_id,
                share_text,
                commission_label,
                sponsored_int,
                price_brl_cents,
                pix_price_cents,
                reference_price_cents,
                currency,
                discount_label,
                installment_summary,
                payment_conditions,
                short_condition,
                freight_status,
                availability,
                price_valid_until,
                observed_at_ms,
                freshness_ttl_minutes_raw,
                evidence_sha256,
            ) = row?;

            category_slugs.insert(category_slug.clone());

            let images = images_by_product.remove(&product_id).unwrap_or_default();
            if images.is_empty() {
                continue;
            }
            let specs = specs_by_product.remove(&product_id).unwrap_or_default();
            let variants = variants_by_product.remove(&product_id).unwrap_or_default();
            let price_observations = obs_by_product.remove(&product_id).unwrap_or_default();
            let evidence_id = format!("ev-ml-{product_id}");
            let media_rights_id = format!("mr-ml-{product_id}");

            let has_seven_day = verify_seven_day_sample_db(&price_observations, price_brl_cents);
            let gallery_paths: Vec<SiteCatalogGalleryItem> = images
                .iter()
                .map(|img| SiteCatalogGalleryItem {
                    path: img.public_url.clone(),
                    alt: img.alt_text.clone(),
                    bucket: Some(img.bucket.clone()),
                    object_key: Some(img.object_key.clone()),
                    sha256: Some(img.sha256_hex.clone()),
                })
                .collect();

            let lead = images
                .iter()
                .find(|img| matches!(img.content_type.as_str(), "image/jpeg" | "image/png"))
                .unwrap_or(&images[0]);
            let return_days = return_days_raw.and_then(|v| u16::try_from(v).ok());
            let freshness_ttl_minutes = u32::try_from(freshness_ttl_minutes_raw).unwrap_or(30);
            let supported_benefits: Vec<String> = serde_json::from_str(&benefits_json)?;
            let limitations: Vec<String> = serde_json::from_str(&limitations_json)?;
            let unknowns: Vec<String> = serde_json::from_str(&unknowns_json)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json)?;

            let verified_facts: Vec<String> = vec![
                format!("ID do catálogo ML: {product_id} · Marca {brand} · Share ID {share_id}."),
                format!(
                    "Preço observado via Hub: {}.",
                    zaturo_core::format_brl_cents(price_brl_cents)
                ),
                one_line_reason.clone(),
            ];
            evidence_sources.push(SiteCatalogEvidenceSource {
                id: evidence_id.clone(),
                source_type: "marketplace_listing".into(),
                title: format!("Anúncio oficial Mercado Livre Afiliados — {title} ({product_id})"),
                publisher: format!("Mercado Livre · {seller_name} (ingestão validada via Hub)"),
                source_url: canonical_url.clone(),
                retrieved_at: format_utc_iso_ms(observed_at_ms),
                verified_facts,
            });
            media_rights.push(SiteCatalogMediaRight {
                id: media_rights_id.clone(),
                asset_description: format!(
                    "Fotos públicas do catálogo Mercado Livre armazenadas em MinIO — {title} ({product_id})"
                ),
                license_type: "Mercado Livre Afiliados — Imagens públicas hospedadas em MinIO".into(),
                rights_verified: true,
                owner_permitted_use: format!(
                    "Exibição vinculada ao anúncio afiliado oficial {product_id} via object storage MinIO."
                ),
                attribution_text: format!(
                    "Fotos oficiais do anúncio no Mercado Livre ({product_id} · {seller_name})."
                ),
            });

            let short_use_line = if short_description.chars().count() > 140 {
                let truncated: String = short_description.chars().take(137).collect();
                format!("{truncated}...")
            } else {
                short_description
            };

            let mut offer = SiteCatalogOffer {
                id: format!("offer-ml-{product_id}"),
                slug,
                is_demo_fixture: false,
                product_id,
                brand,
                model,
                title,
                exact_variant,
                category_slug,
                subcategory_slug: String::new(),
                subcategory_name: String::new(),
                merchant: format!("Mercado Livre · {seller_name}"),
                marketplace_label: "Mercado Livre".into(),
                seller_name: seller_name.clone(),
                store_short: seller_name,
                program: "Mercado Livre Afiliados (ingestão validada via Hub)".into(),
                official_affiliate_url: share_link,
                canonical_url,
                share_id,
                share_text,
                sponsored: sponsored_int != 0,
                commission_rate: commission_label,
                marketplace_rating,
                rating_milli: None,
                marketplace_sales_label,
                sales_volume_estimate: 0,
                card_badge,
                discount_percent_label: discount_label,
                discount_pct: 0,
                trust_badges: Vec::new(),
                installment_summary,
                return_days,
                availability,
                price_valid_until: price_valid_until.clone(),
                image_path: lead.public_url.clone(),
                image_alt: lead.alt_text.clone(),
                gallery_paths,
                price_brl_cents,
                pix_price_cents,
                reference_price_cents,
                currency,
                payment_conditions,
                short_condition,
                short_use_line,
                short_checked_at: format_cuiaba_short(observed_at_ms),
                cta_dest_label: "Ver oferta no Mercado Livre".into(),
                coupon_condition: None,
                freight_status,
                observed_at: format_utc_iso_ms(observed_at_ms),
                observed_at_display: format_cuiaba_display(observed_at_ms),
                valid_until: price_valid_until
                    .as_ref()
                    .map(|d| format!("{d}T23:59:59-03:00")),
                valid_until_display: price_valid_until
                    .as_ref()
                    .map(|d| format!("Validade registrada até {d}")),
                freshness_ttl_minutes,
                evidence_refs: vec![evidence_id],
                media_rights_ref: media_rights_id,
                state,
                approval_revision: format!(
                    "ml-ingested-{}",
                    evidence_sha256.chars().take(12).collect::<String>()
                ),
                one_line_reason,
                for_whom_it_makes_sense: for_whom,
                supported_benefits,
                limitations,
                unknowns,
                tags,
                has_verified_seven_day_sample: has_seven_day,
                seven_day_coverage_note: if has_seven_day {
                    "Série contínua de 7 dias acumulada via histórico persistido no Hub.".into()
                } else {
                    "Ingestão validada via Hub Mercado Livre; série contínua de 7 dias acumulada progressivamente.".into()
                },
                price_observations,
                specs,
                variants,
                long_description,
                price_intelligence: None,
                dynamic_faq: Vec::new(),
                similar_offer_ids: Vec::new(),
            };
            enrich_site_catalog_offer(&mut offer, observed_at_ms);
            offers.push(offer);
        }

        // Compute cross-product Cloobe "Produtos similares comparados" IDs per subcategory/category.
        let summary_meta: Vec<(String, String, String)> = offers
            .iter()
            .map(|o| {
                (
                    o.product_id.clone(),
                    o.subcategory_slug.clone(),
                    o.category_slug.clone(),
                )
            })
            .collect();
        for offer in &mut offers {
            let mut same_sub: Vec<String> = summary_meta
                .iter()
                .filter(|(pid, sub, _)| *pid != offer.product_id && *sub == offer.subcategory_slug)
                .map(|(pid, _, _)| pid.clone())
                .take(4)
                .collect();
            if same_sub.len() < 4 {
                for (pid, _, cat) in &summary_meta {
                    if same_sub.len() >= 4 {
                        break;
                    }
                    if *pid != offer.product_id
                        && *cat == offer.category_slug
                        && !same_sub.contains(pid)
                    {
                        same_sub.push(pid.clone());
                    }
                }
            }
            offer.similar_offer_ids = same_sub;
        }

        let categories = category_slugs
            .into_iter()
            .map(build_category_metadata)
            .collect();

        Ok(SiteCatalogSnapshot {
            generated_at: format_utc_iso_ms(now_ms),
            categories,
            offers,
            evidence_sources,
            media_rights,
        })
    }

    fn ml_all_product_specs(&self) -> Result<HashMap<String, Vec<MercadoLivreSpec>>, Error> {
        let mut stmt = self.connection.prepare(
            "SELECT product_id, spec_name, spec_value FROM ml_product_specs ORDER BY product_id ASC, position ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                MercadoLivreSpec {
                    name: r.get(1)?,
                    value: r.get(2)?,
                },
            ))
        })?;
        let mut map: HashMap<String, Vec<MercadoLivreSpec>> = HashMap::new();
        for row in rows {
            let (pid, spec) = row?;
            map.entry(pid).or_default().push(spec);
        }
        Ok(map)
    }

    fn ml_all_product_variants(&self) -> Result<HashMap<String, Vec<MercadoLivreVariant>>, Error> {
        let mut stmt = self.connection.prepare(
            "SELECT product_id, variant_product_id, sku, name, color, voltage, price_brl_cents, currency,
                    availability, return_days, return_fees, canonical_url, highlights_json
             FROM ml_product_variants ORDER BY product_id ASC, position ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, String>(8)?,
                r.get::<_, Option<i64>>(9)?,
                r.get::<_, Option<String>>(10)?,
                r.get::<_, Option<String>>(11)?,
                r.get::<_, String>(12)?,
            ))
        })?;
        let mut map: HashMap<String, Vec<MercadoLivreVariant>> = HashMap::new();
        for row in rows {
            let (
                product_id,
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
                highlights_json,
            ) = row?;
            map.entry(product_id)
                .or_default()
                .push(MercadoLivreVariant {
                    variant_product_id,
                    sku,
                    name,
                    color,
                    voltage,
                    price_brl_cents,
                    currency,
                    availability,
                    return_days: return_days.and_then(|d| u16::try_from(d).ok()),
                    return_fees,
                    canonical_url,
                    highlights: serde_json::from_str(&highlights_json)?,
                });
        }
        Ok(map)
    }

    fn ml_all_product_images(&self) -> Result<HashMap<String, Vec<MinioProductImage>>, Error> {
        let mut stmt = self.connection.prepare(
            "SELECT product_id, position, minio_bucket, minio_object_key, public_url, source_origin_url,
                    content_type, sha256_hex, size_bytes, alt_text
             FROM ml_product_images ORDER BY product_id ASC, position ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                MinioProductImage {
                    position: u16::try_from(r.get::<_, i64>(1)?).unwrap_or(0),
                    bucket: r.get(2)?,
                    object_key: r.get(3)?,
                    public_url: r.get(4)?,
                    source_origin_url: r.get(5)?,
                    content_type: r.get(6)?,
                    sha256_hex: r.get(7)?,
                    size_bytes: u64::try_from(r.get::<_, i64>(8)?).unwrap_or(0),
                    alt_text: r.get(9)?,
                },
            ))
        })?;
        let mut map: HashMap<String, Vec<MinioProductImage>> = HashMap::new();
        for row in rows {
            let (pid, img) = row?;
            map.entry(pid).or_default().push(img);
        }
        Ok(map)
    }

    fn ml_all_recent_price_observations(
        &self,
    ) -> Result<HashMap<String, Vec<SiteCatalogPriceObservation>>, Error> {
        let mut stmt = self.connection.prepare(
            "SELECT o.product_id, p.seller_name, o.id, o.payment_conditions, o.short_condition,
                    MIN(o.pix_price_cents, o.price_brl_cents), o.observed_at_ms
             FROM ml_offers o
             JOIN ml_products p ON p.product_id = o.product_id
             ORDER BY o.product_id ASC, o.observed_at_ms DESC, o.created_at_ms DESC, o.id DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
            ))
        })?;
        let mut map: HashMap<String, Vec<SiteCatalogPriceObservation>> = HashMap::new();
        let mut dedup_by_product: HashMap<String, ThirtyDayDedupState> = HashMap::new();
        for row in rows {
            let (
                product_id,
                seller_name,
                id,
                payment_basis,
                freight_note,
                eff_price_cents,
                observed_at_ms,
            ) = row?;
            let state = dedup_by_product.entry(product_id.clone()).or_default();
            if !state.accept(eff_price_cents, observed_at_ms) {
                continue;
            }
            let list = map.entry(product_id.clone()).or_default();
            list.push(SiteCatalogPriceObservation {
                id,
                offer_id: format!("offer-ml-{product_id}"),
                variant_id: product_id.clone(),
                seller_name,
                payment_basis,
                freight_note,
                price_brl_cents: eff_price_cents,
                observed_at: format_utc_iso_ms(observed_at_ms),
                evidence_id: format!("ev-ml-{product_id}"),
            });
        }
        Ok(map)
    }
}

fn verify_seven_day_sample_db(
    observations: &[SiteCatalogPriceObservation],
    current_price_cents: i64,
) -> bool {
    if observations.len() < 7 {
        return false;
    }
    let first = &observations[0];
    let same_scope = observations.iter().all(|obs| {
        obs.seller_name == first.seller_name && obs.payment_basis == first.payment_basis
    });
    if !same_scope {
        return false;
    }
    let unique_days: BTreeSet<&str> = observations
        .iter()
        .filter_map(|obs| obs.observed_at.get(..10))
        .collect();
    if unique_days.len() < 7 {
        return false;
    }
    observations
        .iter()
        .map(|obs| obs.price_brl_cents)
        .min()
        .is_some_and(|min_cents| current_price_cents <= min_cents)
}

fn build_category_metadata(slug: String) -> SiteCatalogCategory {
    match slug.as_str() {
        "hardware-e-perifericos" => SiteCatalogCategory {
            slug,
            name: "Hardware e periféricos".into(),
            description: "SSDs NVMe, teclados mecânicos, headsets e monitores para computador com ficha técnica conferida e links oficiais de afiliado.".into(),
            editable_by_owner: true,
            intended_budget_hint: Some("Hardware e periféricos ingeridos via Hub".into()),
        },
        "ergonomia-e-suportes" => SiteCatalogCategory {
            slug,
            name: "Ergonomia e suportes".into(),
            description: "Suportes de notebook e monitor para postura e organização da mesa.".into(),
            editable_by_owner: true,
            intended_budget_hint: None,
        },
        _ => {
            let label = slug.replace('-', " ");
            SiteCatalogCategory {
                slug,
                name: label,
                description: "Categoria gerada dinamicamente a partir das ofertas ingeridas via Hub.".into(),
                editable_by_owner: true,
                intended_budget_hint: None,
            }
        }
    }
}
