//! Relational persistence for ingested Mercado Livre products, specs, variants, MinIO images,
//! price observations, static site catalog snapshots, and Telegram delivery queues.

use crate::{Error, Repository};
use rusqlite::{OptionalExtension, params};
use std::collections::{BTreeSet, HashMap};
use zaturo_core::{
    MercadoLivreIngestBatchRequest, MercadoLivreIngestReport, MercadoLivrePriceComparison,
    MercadoLivreProductState, MercadoLivreSpec, MercadoLivreVariant, MinioProductImage,
    PriceTrendKind, SiteCatalogCategory, SiteCatalogEvidenceSource, SiteCatalogGalleryItem,
    SiteCatalogMediaRight, SiteCatalogOffer, SiteCatalogPriceObservation, SiteCatalogSnapshot,
    TelegramDeliveryStatus, TelegramOfferQueueItem, ValidatedMercadoLivreProduct,
    classify_product_subcategory, enrich_site_catalog_offer, format_cuiaba_display,
    format_cuiaba_short, format_price_comparison_summary,
    format_telegram_caption_with_price_history_pt_br, format_utc_iso_ms,
};

const THIRTY_DAYS_MS: i64 = 30 * 86_400_000;

impl Repository<'_> {
    pub fn ml_batch_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<(String, MercadoLivreIngestReport)>, Error> {
        let row: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT payload_sha256, report_json FROM ml_ingest_batches WHERE idempotency_key = ?1",
                [key],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        row.map(|(sha, report)| Ok((sha, serde_json::from_str(&report)?)))
            .transpose()
    }

    pub fn save_ml_ingest_batch(
        &self,
        batch_id: &str,
        idempotency_key: &str,
        payload_sha256: &str,
        request: &MercadoLivreIngestBatchRequest,
        now_ms: i64,
    ) -> Result<MercadoLivreIngestReport, Error> {
        let image_count: usize = request.products.iter().map(|p| p.images.len()).sum();
        let product_ids: Vec<String> = request
            .products
            .iter()
            .map(|p| p.product_id.clone())
            .collect();
        self.connection.execute(
            "INSERT INTO ml_ingest_batches(
                batch_id, idempotency_key, worker_id, search_query, channel_id,
                captured_at_ms, ingested_at_ms, product_count, image_count,
                payload_sha256, report_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, '{}')",
            params![
                batch_id,
                idempotency_key,
                request.worker_id,
                request.search_query,
                request.channel_id,
                request.captured_at_ms,
                now_ms,
                i64::try_from(request.products.len()).unwrap_or(i64::MAX),
                i64::try_from(image_count).unwrap_or(i64::MAX),
                payload_sha256
            ],
        )?;

        let mut comparisons = Vec::with_capacity(request.products.len());
        let mut telegram_queued_count = 0usize;
        let mut telegram_awaiting_approval_count = 0usize;
        let mut new_product_count = 0usize;
        let mut updated_product_count = 0usize;
        let mut price_drop_count = 0usize;
        let mut price_increase_count = 0usize;
        let mut unchanged_price_count = 0usize;

        for product in &request.products {
            let (cmp, awaiting_approval) =
                self.upsert_ml_product(batch_id, &request.channel_id, product, now_ms)?;
            if cmp.telegram_enqueued {
                telegram_queued_count += 1;
            }
            if awaiting_approval {
                telegram_awaiting_approval_count += 1;
            }
            match cmp.trend {
                PriceTrendKind::New => new_product_count += 1,
                PriceTrendKind::Down => {
                    updated_product_count += 1;
                    price_drop_count += 1;
                }
                PriceTrendKind::Up => {
                    updated_product_count += 1;
                    price_increase_count += 1;
                }
                PriceTrendKind::Stable => {
                    updated_product_count += 1;
                    unchanged_price_count += 1;
                }
            }
            comparisons.push(cmp);
        }

        let report = MercadoLivreIngestReport {
            batch_id: batch_id.to_owned(),
            worker_id: request.worker_id.clone(),
            ingested_at_ms: now_ms,
            product_count: request.products.len(),
            image_count,
            telegram_queued_count,
            telegram_awaiting_approval_count,
            product_ids,
            payload_sha256: payload_sha256.to_owned(),
            new_product_count,
            updated_product_count,
            price_drop_count,
            price_increase_count,
            unchanged_price_count,
            comparisons,
        };
        let report_json = serde_json::to_string(&report)?;
        self.connection.execute(
            "UPDATE ml_ingest_batches SET report_json = ?2 WHERE batch_id = ?1",
            params![batch_id, report_json],
        )?;

        Ok(report)
    }

    fn upsert_ml_product(
        &self,
        batch_id: &str,
        channel_id: &str,
        product: &ValidatedMercadoLivreProduct,
        now_ms: i64,
    ) -> Result<(MercadoLivrePriceComparison, bool), Error> {
        let previous_eff_cents: Option<i64> = self
            .connection
            .query_row(
                "SELECT MIN(pix_price_cents, price_brl_cents)
                 FROM ml_offers
                 WHERE product_id = ?1 AND is_current = 1",
                [&product.product_id],
                |r| r.get(0),
            )
            .optional()?;
        self.connection.execute(
            "INSERT INTO ml_products(
                product_id, slug, title, brand, model, exact_variant, category_slug,
                canonical_url, seller_name, marketplace_rating, marketplace_sales_label,
                card_badge, return_days, short_description, long_description,
                one_line_reason, for_whom, supported_benefits_json, limitations_json,
                unknowns_json, tags_json, state, last_batch_id, created_at_ms, updated_at_ms
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?24
            )
            ON CONFLICT(product_id) DO UPDATE SET
                slug = excluded.slug,
                title = excluded.title,
                brand = excluded.brand,
                model = excluded.model,
                exact_variant = excluded.exact_variant,
                category_slug = excluded.category_slug,
                canonical_url = excluded.canonical_url,
                seller_name = excluded.seller_name,
                marketplace_rating = excluded.marketplace_rating,
                marketplace_sales_label = excluded.marketplace_sales_label,
                card_badge = excluded.card_badge,
                return_days = excluded.return_days,
                short_description = excluded.short_description,
                long_description = excluded.long_description,
                one_line_reason = excluded.one_line_reason,
                for_whom = excluded.for_whom,
                supported_benefits_json = excluded.supported_benefits_json,
                limitations_json = excluded.limitations_json,
                unknowns_json = excluded.unknowns_json,
                tags_json = excluded.tags_json,
                state = excluded.state,
                last_batch_id = excluded.last_batch_id,
                updated_at_ms = excluded.updated_at_ms",
            params![
                product.product_id,
                product.slug,
                product.title,
                product.brand,
                product.model,
                product.exact_variant,
                product.category_slug,
                product.canonical_url,
                product.seller_name,
                product.marketplace_rating,
                product.marketplace_sales_label,
                product.card_badge,
                product.return_days.map(i64::from),
                product.short_description,
                product.long_description,
                product.one_line_reason,
                product.for_whom,
                serde_json::to_string(&product.supported_benefits)?,
                serde_json::to_string(&product.limitations)?,
                serde_json::to_string(&product.unknowns)?,
                serde_json::to_string(&product.tags)?,
                product.state.as_str(),
                batch_id,
                now_ms
            ],
        )?;

        self.connection.execute(
            "UPDATE ml_offers SET is_current = 0 WHERE product_id = ?1 AND is_current = 1",
            [&product.product_id],
        )?;

        let offer_id = format!("obs-{}-{}", product.product_id, batch_id);
        self.connection.execute(
            "INSERT INTO ml_offers(
                id, product_id, batch_id, share_link, share_id, share_text,
                commission_label, commission_bp, sponsored, price_brl_cents, pix_price_cents,
                reference_price_cents, currency, discount_label, installment_summary,
                payment_conditions, short_condition, freight_status, availability,
                price_valid_until, observed_at_ms, freshness_ttl_minutes,
                evidence_sha256, is_current, created_at_ms
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, 1, ?24
            )",
            params![
                offer_id,
                product.product_id,
                batch_id,
                product.share_link,
                product.share_id,
                product.share_text,
                product.commission_label,
                product.commission_bp.map(i64::from),
                i64::from(product.sponsored),
                product.price_brl_cents,
                product.pix_price_cents,
                product.reference_price_cents,
                product.currency,
                product.discount_label,
                product.installment_summary,
                product.payment_conditions,
                product.short_condition,
                product.freight_status,
                product.availability,
                product.price_valid_until,
                product.observed_at_ms,
                i64::from(product.freshness_ttl_minutes),
                product.evidence_sha256,
                now_ms
            ],
        )?;

        self.connection.execute(
            "DELETE FROM ml_product_specs WHERE product_id = ?1",
            [&product.product_id],
        )?;
        for (idx, spec) in product.specs.iter().enumerate() {
            self.connection.execute(
                "INSERT INTO ml_product_specs(product_id, position, spec_name, spec_value)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    product.product_id,
                    i64::try_from(idx).unwrap_or(i64::MAX),
                    spec.name,
                    spec.value
                ],
            )?;
        }

        self.connection.execute(
            "DELETE FROM ml_product_variants WHERE product_id = ?1",
            [&product.product_id],
        )?;
        for (idx, variant) in product.variants.iter().enumerate() {
            self.connection.execute(
                "INSERT INTO ml_product_variants(
                    product_id, position, variant_product_id, sku, name, color, voltage,
                    price_brl_cents, currency, availability, return_days, return_fees,
                    canonical_url, highlights_json
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                params![
                    product.product_id,
                    i64::try_from(idx).unwrap_or(i64::MAX),
                    variant.variant_product_id,
                    variant.sku,
                    variant.name,
                    variant.color,
                    variant.voltage,
                    variant.price_brl_cents,
                    variant.currency,
                    variant.availability,
                    variant.return_days.map(i64::from),
                    variant.return_fees,
                    variant.canonical_url,
                    serde_json::to_string(&variant.highlights)?
                ],
            )?;
        }

        self.connection.execute(
            "DELETE FROM ml_product_images WHERE product_id = ?1",
            [&product.product_id],
        )?;
        let attribution = format!(
            "Fotos oficiais do anúncio no Mercado Livre ({} · {}).",
            product.product_id, product.seller_name
        );
        for img in &product.images {
            let image_id = format!("img-{}-{}", product.product_id, img.position);
            self.connection.execute(
                "INSERT INTO ml_product_images(
                    id, product_id, position, minio_bucket, minio_object_key, public_url,
                    source_origin_url, content_type, sha256_hex, size_bytes, alt_text,
                    rights_verified, attribution_text, uploaded_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, ?12, ?13)",
                params![
                    image_id,
                    product.product_id,
                    i64::from(img.position),
                    img.bucket,
                    img.object_key,
                    img.public_url,
                    img.source_origin_url,
                    img.content_type,
                    img.sha256_hex,
                    i64::try_from(img.size_bytes).unwrap_or(i64::MAX),
                    img.alt_text,
                    attribution,
                    now_ms
                ],
            )?;
        }

        let cur_eff = product.pix_price_cents.min(product.price_brl_cents);
        let (subcategory_slug, _) =
            classify_product_subcategory(&product.title, &product.category_slug, &product.specs);
        let (lowest_30d_raw, average_price_30d_cents, highest_30d_raw, observation_count) =
            self.ml_product_30d_stats(&product.product_id, cur_eff)?;
        let lowest_price_30d_cents = lowest_30d_raw.min(cur_eff);
        let highest_price_30d_cents = product
            .reference_price_cents
            .filter(|r| *r > 0)
            .map_or(highest_30d_raw.max(cur_eff), |r| {
                highest_30d_raw.max(cur_eff).max(r)
            });
        let (trend, price_delta_cents, price_delta_pct) =
            PriceTrendKind::from_prices(cur_eff, previous_eff_cents);
        let summary = format_price_comparison_summary(
            cur_eff,
            previous_eff_cents,
            lowest_price_30d_cents,
            average_price_30d_cents,
            observation_count,
            trend,
            price_delta_pct,
        );

        let mut comparison = MercadoLivrePriceComparison {
            product_id: product.product_id.clone(),
            title: product.title.clone(),
            category_slug: product.category_slug.clone(),
            subcategory_slug,
            current_price_cents: cur_eff,
            pix_price_cents: product.pix_price_cents,
            previous_price_cents: previous_eff_cents,
            price_delta_cents,
            price_delta_pct,
            lowest_price_30d_cents,
            average_price_30d_cents,
            highest_price_30d_cents,
            observation_count,
            trend,
            telegram_enqueued: false,
            summary,
        };
        let mut awaiting_approval = false;

        match product.state {
            MercadoLivreProductState::Candidate => {
                self.connection.execute(
                    "UPDATE ml_telegram_deliveries
                     SET status = 'skipped', error_code = 'superseded', updated_at_ms = ?3
                     WHERE product_id = ?1 AND channel_id = ?2
                       AND status IN ('awaiting_approval', 'queued')",
                    params![product.product_id, channel_id, now_ms],
                )?;
                let lead = product
                    .images
                    .iter()
                    .find(|img| matches!(img.content_type.as_str(), "image/jpeg" | "image/png"))
                    .unwrap_or(&product.images[0]);
                let delivery_id = format!("tg-{}-{}", product.product_id, batch_id);
                let message_text =
                    format_telegram_caption_with_price_history_pt_br(product, Some(&comparison));
                self.connection.execute(
                    "INSERT INTO ml_telegram_deliveries(
                        delivery_id, product_id, offer_id, channel_id, message_text,
                        image_public_url, image_object_key, status, external_message_id,
                        error_code, created_at_ms, updated_at_ms, published_at_ms
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'awaiting_approval', NULL, NULL, ?8, ?8, NULL)",
                    params![
                        delivery_id,
                        product.product_id,
                        offer_id,
                        channel_id,
                        message_text,
                        lead.public_url,
                        lead.object_key,
                        now_ms
                    ],
                )?;
                awaiting_approval = true;
            }
            MercadoLivreProductState::Approved => {
                let lead = product
                    .images
                    .iter()
                    .find(|img| matches!(img.content_type.as_str(), "image/jpeg" | "image/png"))
                    .unwrap_or(&product.images[0]);
                let message_text =
                    format_telegram_caption_with_price_history_pt_br(product, Some(&comparison));

                let active_sending: bool = self
                    .connection
                    .query_row(
                        "SELECT 1 FROM ml_telegram_deliveries
                         WHERE product_id = ?1 AND channel_id = ?2 AND status = 'sending'
                         LIMIT 1",
                        params![product.product_id, channel_id],
                        |_| Ok(()),
                    )
                    .optional()?
                    .is_some();

                if !active_sending {
                    self.connection.execute(
                        "UPDATE ml_telegram_deliveries
                         SET status = 'skipped', error_code = 'superseded', updated_at_ms = ?3
                         WHERE product_id = ?1 AND channel_id = ?2
                           AND status = 'awaiting_approval'",
                        params![product.product_id, channel_id, now_ms],
                    )?;

                    let existing_queued: Option<(String, i64)> = self
                        .connection
                        .query_row(
                            "SELECT d.delivery_id, MIN(o.pix_price_cents, o.price_brl_cents)
                             FROM ml_telegram_deliveries d
                             JOIN ml_offers o ON o.id = d.offer_id
                             WHERE d.product_id = ?1 AND d.channel_id = ?2 AND d.status = 'queued'
                             ORDER BY d.updated_at_ms DESC, d.created_at_ms DESC
                             LIMIT 1",
                            params![product.product_id, channel_id],
                            |r| Ok((r.get(0)?, r.get(1)?)),
                        )
                        .optional()?;

                    if let Some((queued_id, queued_eff)) = existing_queued {
                        if cur_eff == queued_eff {
                            self.connection.execute(
                                "UPDATE ml_telegram_deliveries
                                 SET offer_id = ?2,
                                     image_public_url = ?3,
                                     image_object_key = ?4,
                                     updated_at_ms = ?5
                                 WHERE delivery_id = ?1",
                                params![
                                    queued_id,
                                    offer_id,
                                    lead.public_url,
                                    lead.object_key,
                                    now_ms
                                ],
                            )?;
                        } else {
                            self.connection.execute(
                                "UPDATE ml_telegram_deliveries
                                 SET offer_id = ?2,
                                     message_text = ?3,
                                     image_public_url = ?4,
                                     image_object_key = ?5,
                                     updated_at_ms = ?6
                                 WHERE delivery_id = ?1",
                                params![
                                    queued_id,
                                    offer_id,
                                    message_text,
                                    lead.public_url,
                                    lead.object_key,
                                    now_ms
                                ],
                            )?;
                        }
                        comparison.telegram_enqueued = cur_eff < queued_eff;
                    } else {
                        let last_published_eff: Option<i64> = self
                            .connection
                            .query_row(
                                "SELECT MIN(o.pix_price_cents, o.price_brl_cents)
                                 FROM ml_telegram_deliveries d
                                 JOIN ml_offers o ON o.id = d.offer_id
                                 WHERE d.product_id = ?1 AND d.channel_id = ?2 AND d.status = 'published'
                                 ORDER BY COALESCE(d.published_at_ms, d.updated_at_ms) DESC, d.created_at_ms DESC
                                 LIMIT 1",
                                params![product.product_id, channel_id],
                                |r| r.get(0),
                            )
                            .optional()?;

                        if last_published_eff.is_none_or(|prev_pub| cur_eff < prev_pub) {
                            let delivery_id = format!("tg-{}-{}", product.product_id, batch_id);
                            self.connection.execute(
                                "INSERT INTO ml_telegram_deliveries(
                                    delivery_id, product_id, offer_id, channel_id, message_text,
                                    image_public_url, image_object_key, status, external_message_id,
                                    error_code, created_at_ms, updated_at_ms, published_at_ms
                                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'queued', NULL, NULL, ?8, ?8, NULL)",
                                params![
                                    delivery_id,
                                    product.product_id,
                                    offer_id,
                                    channel_id,
                                    message_text,
                                    lead.public_url,
                                    lead.object_key,
                                    now_ms
                                ],
                            )?;
                            comparison.telegram_enqueued = true;
                        }
                    }
                }
            }
            MercadoLivreProductState::Stale | MercadoLivreProductState::Withdrawn => {
                self.connection.execute(
                    "UPDATE ml_telegram_deliveries
                     SET status = 'skipped', error_code = 'superseded', updated_at_ms = ?3
                     WHERE product_id = ?1 AND channel_id = ?2
                       AND status IN ('awaiting_approval', 'queued')",
                    params![product.product_id, channel_id, now_ms],
                )?;
            }
        }

        Ok((comparison, awaiting_approval))
    }

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

    fn ml_product_30d_stats(
        &self,
        product_id: &str,
        fallback_price_cents: i64,
    ) -> Result<(i64, i64, i64, usize), Error> {
        let mut stmt = self.connection.prepare(
            "SELECT MIN(pix_price_cents, price_brl_cents), observed_at_ms
             FROM ml_offers
             WHERE product_id = ?1
             ORDER BY observed_at_ms DESC, created_at_ms DESC, id DESC",
        )?;
        let rows = stmt.query_map([product_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
        })?;
        let mut dedup = ThirtyDayDedupState::default();
        let mut min_cents = i64::MAX;
        let mut max_cents = i64::MIN;
        let mut sum: i128 = 0;
        let mut count = 0usize;
        for row in rows {
            let (eff_cents, observed_at_ms) = row?;
            if eff_cents <= 0 || !dedup.accept(eff_cents, observed_at_ms) {
                continue;
            }
            min_cents = min_cents.min(eff_cents);
            max_cents = max_cents.max(eff_cents);
            sum += i128::from(eff_cents);
            count += 1;
        }
        if count == 0 {
            return Ok((
                fallback_price_cents,
                fallback_price_cents,
                fallback_price_cents,
                1,
            ));
        }
        let avg_cents =
            i64::try_from(sum / i128::try_from(count).unwrap_or(1)).unwrap_or(fallback_price_cents);
        Ok((min_cents, avg_cents, max_cents, count))
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

    /// List deliveries newest first. With `fresh_at_ms`, only deliveries whose offer is the
    /// product's current observation and is still inside its freshness window are returned.
    pub fn ml_telegram_queue(
        &self,
        channel_id: Option<&str>,
        status: Option<TelegramDeliveryStatus>,
        fresh_at_ms: Option<i64>,
        limit: u32,
    ) -> Result<Vec<TelegramOfferQueueItem>, Error> {
        let status_str = status.map(|s| s.as_str().to_owned());
        let mut stmt = self.connection.prepare(&format!(
            "{DELIVERY_SELECT}
             WHERE (?1 IS NULL OR d.channel_id = ?1)
               AND (?2 IS NULL OR d.status = ?2)
               AND (?3 IS NULL OR ({OFFER_FRESH_AT_3}))
             ORDER BY d.updated_at_ms DESC, d.delivery_id DESC
             LIMIT ?4"
        ))?;
        let rows = stmt.query_map(
            params![channel_id, status_str, fresh_at_ms, limit],
            delivery_row,
        )?;
        rows.map(|row| row?.into_item()).collect()
    }

    /// Load one delivery by its exact identifier.
    pub fn ml_telegram_delivery(
        &self,
        delivery_id: &str,
    ) -> Result<Option<TelegramOfferQueueItem>, Error> {
        self.connection
            .query_row(
                &format!("{DELIVERY_SELECT} WHERE d.delivery_id = ?1"),
                [delivery_id],
                delivery_row,
            )
            .optional()?
            .map(DeliveryRow::into_item)
            .transpose()
    }

    /// Load the delivery for the product's current offer on one channel.
    pub fn ml_current_telegram_delivery(
        &self,
        product_id: &str,
        channel_id: &str,
    ) -> Result<Option<TelegramOfferQueueItem>, Error> {
        self.connection
            .query_row(
                &format!(
                    "{DELIVERY_SELECT}
                     WHERE d.product_id = ?1 AND d.channel_id = ?2 AND o.is_current = 1"
                ),
                [product_id, channel_id],
                delivery_row,
            )
            .optional()?
            .map(DeliveryRow::into_item)
            .transpose()
    }

    /// Whether the delivery's offer is the product's current observation and still fresh.
    pub fn ml_telegram_delivery_offer_fresh(
        &self,
        delivery_id: &str,
        now_ms: i64,
    ) -> Result<bool, Error> {
        Ok(self
            .connection
            .query_row(
                &format!(
                    "SELECT 1 FROM ml_telegram_deliveries d
                     JOIN ml_offers o ON o.id = d.offer_id
                     WHERE d.delivery_id = ?1 AND {OFFER_FRESH_AT_2}"
                ),
                params![delivery_id, now_ms],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// Compare-and-set a delivery status. Returns false when the stored status was not `from`,
    /// so concurrent publishers cannot both claim or finish the same delivery.
    pub fn set_ml_telegram_delivery_status(
        &self,
        delivery_id: &str,
        from: TelegramDeliveryStatus,
        to: TelegramDeliveryStatus,
        external_message_id: Option<&str>,
        error_code: Option<&str>,
        now_ms: i64,
    ) -> Result<bool, Error> {
        let published_at_ms = (to == TelegramDeliveryStatus::Published).then_some(now_ms);
        let changed = self.connection.execute(
            "UPDATE ml_telegram_deliveries
             SET status = ?3,
                 external_message_id = COALESCE(?4, external_message_id),
                 error_code = ?5,
                 updated_at_ms = ?6,
                 published_at_ms = COALESCE(?7, published_at_ms)
             WHERE delivery_id = ?1 AND status = ?2",
            params![
                delivery_id,
                from.as_str(),
                to.as_str(),
                external_message_id,
                error_code,
                now_ms,
                published_at_ms
            ],
        )?;
        Ok(changed == 1)
    }

    pub fn ml_telegram_product_stats(
        &self,
    ) -> Result<HashMap<String, (u32, Option<String>)>, Error> {
        let mut stmt = self.connection.prepare(
            "SELECT product_id, status
             FROM ml_telegram_deliveries
             ORDER BY product_id ASC, updated_at_ms DESC, created_at_ms DESC, delivery_id DESC",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        let mut stats: HashMap<String, (u32, Option<String>)> = HashMap::new();
        for row in rows {
            let (pid, status) = row?;
            let entry = stats.entry(pid).or_insert((0, None));
            if entry.1.is_none() {
                entry.1 = Some(status.clone());
            }
            if status == "published" {
                entry.0 = entry.0.saturating_add(1);
            }
        }
        Ok(stats)
    }

    /// Record the owner's product approval without touching withdrawn products.
    pub fn approve_ml_product(&self, product_id: &str, now_ms: i64) -> Result<(), Error> {
        self.connection.execute(
            "UPDATE ml_products SET state = 'approved', updated_at_ms = ?2
             WHERE product_id = ?1 AND state <> 'withdrawn'",
            params![product_id, now_ms],
        )?;
        Ok(())
    }
}

#[derive(Default)]
struct ThirtyDayDedupState {
    cutoff_ms: Option<i64>,
    last_kept: Option<(i64, i64)>,
    kept_count: usize,
}

impl ThirtyDayDedupState {
    fn accept(&mut self, price_cents: i64, observed_at_ms: i64) -> bool {
        if self.kept_count >= 30 {
            return false;
        }
        let cutoff = *self
            .cutoff_ms
            .get_or_insert_with(|| observed_at_ms.saturating_sub(THIRTY_DAYS_MS));
        if observed_at_ms < cutoff {
            return false;
        }
        let day_utc = observed_at_ms.div_euclid(86_400_000);
        if self.last_kept == Some((price_cents, day_utc)) {
            return false;
        }
        self.last_kept = Some((price_cents, day_utc));
        self.kept_count += 1;
        true
    }
}

const DELIVERY_SELECT: &str = "SELECT
        d.delivery_id, d.product_id, d.offer_id, d.channel_id,
        p.title, p.brand, p.model,
        o.price_brl_cents, o.reference_price_cents, o.discount_label,
        o.share_link, o.share_id,
        d.image_public_url, d.image_object_key, d.message_text,
        d.status, d.external_message_id, d.created_at_ms, d.updated_at_ms, d.published_at_ms
     FROM ml_telegram_deliveries d
     JOIN ml_products p ON p.product_id = d.product_id
     JOIN ml_offers o ON o.id = d.offer_id";

const OFFER_FRESH_AT_2: &str =
    "o.is_current = 1 AND o.observed_at_ms + o.freshness_ttl_minutes * 60000 >= ?2";
const OFFER_FRESH_AT_3: &str =
    "o.is_current = 1 AND o.observed_at_ms + o.freshness_ttl_minutes * 60000 >= ?3";

struct DeliveryRow {
    delivery_id: String,
    product_id: String,
    offer_id: String,
    channel_id: String,
    title: String,
    brand: String,
    model: Option<String>,
    price_brl_cents: i64,
    reference_price_cents: Option<i64>,
    discount_label: Option<String>,
    share_link: String,
    share_id: String,
    image_public_url: Option<String>,
    image_object_key: Option<String>,
    message_text: String,
    status: String,
    external_message_id: Option<String>,
    created_at_ms: i64,
    updated_at_ms: i64,
    published_at_ms: Option<i64>,
}

fn delivery_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<DeliveryRow> {
    Ok(DeliveryRow {
        delivery_id: r.get(0)?,
        product_id: r.get(1)?,
        offer_id: r.get(2)?,
        channel_id: r.get(3)?,
        title: r.get(4)?,
        brand: r.get(5)?,
        model: r.get(6)?,
        price_brl_cents: r.get(7)?,
        reference_price_cents: r.get(8)?,
        discount_label: r.get(9)?,
        share_link: r.get(10)?,
        share_id: r.get(11)?,
        image_public_url: r.get(12)?,
        image_object_key: r.get(13)?,
        message_text: r.get(14)?,
        status: r.get(15)?,
        external_message_id: r.get(16)?,
        created_at_ms: r.get(17)?,
        updated_at_ms: r.get(18)?,
        published_at_ms: r.get(19)?,
    })
}

impl DeliveryRow {
    fn into_item(self) -> Result<TelegramOfferQueueItem, Error> {
        let status = TelegramDeliveryStatus::parse(&self.status).ok_or(Error::SchemaVersion)?;
        Ok(TelegramOfferQueueItem {
            delivery_id: self.delivery_id,
            product_id: self.product_id,
            offer_id: self.offer_id,
            channel_id: self.channel_id,
            title: self.title,
            brand: self.brand,
            model: self.model,
            price_brl_cents: self.price_brl_cents,
            reference_price_cents: self.reference_price_cents,
            discount_label: self.discount_label,
            share_link: self.share_link,
            share_id: self.share_id,
            image_public_url: self.image_public_url,
            image_object_key: self.image_object_key,
            message_text: self.message_text,
            status,
            external_message_id: self.external_message_id,
            created_at_ms: self.created_at_ms,
            updated_at_ms: self.updated_at_ms,
            published_at_ms: self.published_at_ms,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HubStore;

    fn sample_product(
        product_id: &str,
        slug: &str,
        price_cents: i64,
    ) -> ValidatedMercadoLivreProduct {
        ValidatedMercadoLivreProduct {
            product_id: product_id.into(),
            slug: slug.into(),
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
            price_brl_cents: price_cents,
            pix_price_cents: price_cents,
            reference_price_cents: Some(137_900),
            currency: "BRL".into(),
            marketplace_rating: Some("4,9 / 5".into()),
            marketplace_sales_label: Some("+10 mil vendidos no ML".into()),
            card_badge: Some("Mais vendido".into()),
            discount_label: Some("49% OFF".into()),
            installment_summary: Some("18× de R$ 46,26".into()),
            payment_conditions: "R$ 699,90 à vista ou em até 18× de R$ 46,26".into(),
            short_condition: "18× R$ 46,26 · devolução grátis 30d".into(),
            freight_status: "Frete calculado no CEP; devolução grátis 30 dias.".into(),
            availability: "InStock".into(),
            price_valid_until: Some("2026-11-18".into()),
            return_days: Some(30),
            short_description: "M.2 2280 NVMe PCIe 4.0 x4 · leitura até 5.000 MB/s.".into(),
            long_description: "SSD interno Kingston NV3 500 GB com memória 3D NAND.".into(),
            one_line_reason: "SSD PCIe 4.0 para notebook e desktop com leitura de 5.000 MB/s."
                .into(),
            for_whom: "Quem precisa atualizar slot M.2 2280 com boa velocidade.".into(),
            supported_benefits: vec!["Leitura até 5.000 MB/s.".into()],
            limitations: vec!["Não acompanha dissipador de calor.".into()],
            unknowns: vec!["Série contínua de 7 dias ainda em acumulação.".into()],
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
                variant_product_id: product_id.into(),
                sku: "MLB7607021422".into(),
                name: "SSD Kingston NV3 500GB".into(),
                color: Some("Azul-aço".into()),
                voltage: None,
                price_brl_cents: 69_800,
                currency: "BRL".into(),
                availability: "InStock".into(),
                return_days: Some(30),
                return_fees: Some("Grátis".into()),
                canonical_url: Some(format!(
                    "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/{product_id}"
                )),
                highlights: vec!["Formato M.2 2280".into()],
            }],
            images: vec![MinioProductImage {
                position: 0,
                bucket: "procurali-media".into(),
                object_key: format!("mercadolivre/{product_id}/00-aaaaaaaaaaaaaaaa.jpg"),
                public_url: format!(
                    "https://media.procurali.com.br/procurali-media/mercadolivre/{product_id}/00-aaaaaaaaaaaaaaaa.jpg"
                ),
                source_origin_url: Some("https://http2.mlstatic.com/D_NQ_NP_sample.jpg".into()),
                content_type: "image/jpeg".into(),
                sha256_hex: "a".repeat(64),
                size_bytes: 12_345,
                alt_text: "Embalagem do SSD Kingston NV3 500GB".into(),
            }],
            observed_at_ms: 1_763_231_280_000,
            freshness_ttl_minutes: 30,
            evidence_sha256: "b".repeat(64),
            state: MercadoLivreProductState::Approved,
        }
    }

    #[test]
    fn stores_products_specs_variants_minio_images_and_serves_site_and_telegram_queue() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path().join("hub.db")).unwrap();
        let mut candidate =
            sample_product("MLB39410080", "ssd-kingston-nv3-500gb-mlb39410080", 69_990);
        candidate.state = MercadoLivreProductState::Candidate;
        let req = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "ssd nvme".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: 1_763_231_280_000,
            products: vec![candidate],
        };
        req.validates().unwrap();

        let report = store
            .transaction(|repo| {
                repo.save_ml_ingest_batch(
                    "batch-1",
                    "idem-1",
                    &"c".repeat(64),
                    &req,
                    1_763_231_290_000,
                )
            })
            .unwrap();
        assert_eq!(report.product_count, 1);
        assert_eq!(report.image_count, 1);
        assert_eq!(report.telegram_queued_count, 0);
        assert_eq!(report.telegram_awaiting_approval_count, 1);

        let catalog = store
            .read(|repo| repo.ml_site_catalog(1_763_231_300_000))
            .unwrap();
        assert_eq!(catalog.offers.len(), 1);
        assert_eq!(catalog.offers[0].specs.len(), 2);
        assert_eq!(catalog.offers[0].variants.len(), 1);
        assert!(
            catalog.offers[0]
                .image_path
                .starts_with("https://media.procurali.com.br/")
        );

        let awaiting = store
            .read(|repo| {
                repo.ml_telegram_queue(
                    Some("@procurali"),
                    Some(TelegramDeliveryStatus::AwaitingApproval),
                    None,
                    20,
                )
            })
            .unwrap();
        assert_eq!(awaiting.len(), 1);
        assert!(awaiting[0].message_text.contains("https://meli.la/1iWQZbJ"));
        let id = awaiting[0].delivery_id.clone();

        // Compare-and-set refuses a transition from a status the row is not in.
        let wrong_from = store
            .transaction(|repo| {
                repo.set_ml_telegram_delivery_status(
                    &id,
                    TelegramDeliveryStatus::Queued,
                    TelegramDeliveryStatus::Sending,
                    None,
                    None,
                    1_763_231_400_000,
                )
            })
            .unwrap();
        assert!(!wrong_from);

        // Re-ingesting the same product supersedes the unsent delivery.
        store
            .transaction(|repo| {
                repo.save_ml_ingest_batch(
                    "batch-2",
                    "idem-2",
                    &"d".repeat(64),
                    &req,
                    1_763_231_410_000,
                )
            })
            .unwrap();
        let old = store
            .read(|repo| repo.ml_telegram_delivery(&id))
            .unwrap()
            .unwrap();
        assert_eq!(old.status, TelegramDeliveryStatus::Skipped);
        let current = store
            .read(|repo| repo.ml_current_telegram_delivery("MLB39410080", "@procurali"))
            .unwrap()
            .unwrap();
        assert_ne!(current.delivery_id, id);
        assert_eq!(current.status, TelegramDeliveryStatus::AwaitingApproval);

        // Freshness is bounded by observed_at_ms + freshness_ttl_minutes.
        let observed = 1_763_231_280_000_i64;
        let fresh = |now| {
            store
                .read(|repo| repo.ml_telegram_delivery_offer_fresh(&current.delivery_id, now))
                .unwrap()
        };
        assert!(fresh(observed + 30 * 60_000));
        assert!(!fresh(observed + 30 * 60_000 + 1));
        assert!(!fresh_old(&store, &id, observed));

        let updated = store
            .transaction(|repo| {
                assert!(repo.set_ml_telegram_delivery_status(
                    &current.delivery_id,
                    TelegramDeliveryStatus::AwaitingApproval,
                    TelegramDeliveryStatus::Published,
                    Some("98765"),
                    None,
                    1_763_231_420_000,
                )?);
                repo.ml_telegram_delivery(&current.delivery_id)
            })
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, TelegramDeliveryStatus::Published);
        assert_eq!(updated.published_at_ms, Some(1_763_231_420_000));
        assert_eq!(updated.external_message_id.as_deref(), Some("98765"));
    }

    fn fresh_old(store: &HubStore, id: &str, now: i64) -> bool {
        store
            .read(|repo| repo.ml_telegram_delivery_offer_fresh(id, now))
            .unwrap()
    }

    #[test]
    fn tracks_price_history_comparisons_30d_dedup_and_smart_telegram_alerts() {
        let dir = tempfile::tempdir().unwrap();
        let store = HubStore::open(dir.path().join("hub.db")).unwrap();
        let day1 = 1_763_200_000_000i64;

        let make_req = |price_cents: i64, ts: i64| -> MercadoLivreIngestBatchRequest {
            let mut p = sample_product("MLB63816534", "delux-m900pro-mlb63816534", price_cents);
            p.observed_at_ms = ts;
            let req = MercadoLivreIngestBatchRequest {
                worker_id: "worker-ml".into(),
                search_query: "mouse gamer".into(),
                channel_id: "@procurali".into(),
                captured_at_ms: ts,
                products: vec![p],
            };
            req.validates().unwrap();
            req
        };

        // 1. Initial capture -> New + queued to Telegram
        let r1 = store
            .transaction(|repo| {
                repo.save_ml_ingest_batch(
                    "b1",
                    "i1",
                    &"1".repeat(64),
                    &make_req(29_900, day1),
                    day1,
                )
            })
            .unwrap();
        assert_eq!(r1.new_product_count, 1);
        assert_eq!(r1.telegram_queued_count, 1);
        assert_eq!(r1.comparisons[0].trend, PriceTrendKind::New);
        assert!(r1.comparisons[0].telegram_enqueued);

        let q1 = store
            .read(|repo| {
                repo.ml_telegram_queue(
                    Some("@procurali"),
                    Some(TelegramDeliveryStatus::Queued),
                    None,
                    10,
                )
            })
            .unwrap();
        assert_eq!(q1.len(), 1);
        store
            .transaction(|repo| {
                assert!(repo.set_ml_telegram_delivery_status(
                    &q1[0].delivery_id,
                    TelegramDeliveryStatus::Queued,
                    TelegramDeliveryStatus::Sending,
                    None,
                    None,
                    day1 + 59_000,
                )?);
                repo.set_ml_telegram_delivery_status(
                    &q1[0].delivery_id,
                    TelegramDeliveryStatus::Sending,
                    TelegramDeliveryStatus::Published,
                    Some("101"),
                    None,
                    day1 + 60_000,
                )
            })
            .unwrap();

        // 2. Same-day identical price -> Stable, NOT re-queued, collapsed in 30d series
        let same_day = day1 + 3_600_000;
        let r2 = store
            .transaction(|repo| {
                repo.save_ml_ingest_batch(
                    "b2",
                    "i2",
                    &"2".repeat(64),
                    &make_req(29_900, same_day),
                    same_day,
                )
            })
            .unwrap();
        assert_eq!(r2.unchanged_price_count, 1);
        assert_eq!(r2.telegram_queued_count, 0);
        assert_eq!(r2.comparisons[0].trend, PriceTrendKind::Stable);
        assert!(!r2.comparisons[0].telegram_enqueued);
        assert_eq!(r2.comparisons[0].observation_count, 1);

        let cat2 = store.read(|repo| repo.ml_site_catalog(same_day)).unwrap();
        assert_eq!(cat2.offers[0].price_observations.len(), 1);

        // 3. Next-day price drop -> Down, enqueues Telegram price-drop alert, updates 30d min/avg
        let day2 = day1 + 86_400_000;
        let r3 = store
            .transaction(|repo| {
                repo.save_ml_ingest_batch(
                    "b3",
                    "i3",
                    &"3".repeat(64),
                    &make_req(24_900, day2),
                    day2,
                )
            })
            .unwrap();
        assert_eq!(r3.price_drop_count, 1);
        assert_eq!(r3.telegram_queued_count, 1);
        assert_eq!(r3.comparisons[0].trend, PriceTrendKind::Down);
        assert_eq!(r3.comparisons[0].previous_price_cents, Some(29_900));
        assert_eq!(r3.comparisons[0].price_delta_cents, -5_000);
        assert_eq!(r3.comparisons[0].lowest_price_30d_cents, 24_900);
        assert_eq!(r3.comparisons[0].average_price_30d_cents, 27_400);
        assert_eq!(r3.comparisons[0].observation_count, 2);

        // Repeating same dropped price while still queued must update in-place without stacking duplicates
        let day2b = day2 + 600_000;
        let r3b = store
            .transaction(|repo| {
                repo.save_ml_ingest_batch(
                    "b3b",
                    "i3b",
                    &"4".repeat(64),
                    &make_req(24_900, day2b),
                    day2b,
                )
            })
            .unwrap();
        assert_eq!(r3b.telegram_queued_count, 0);
        let q_drop = store
            .read(|repo| {
                repo.ml_telegram_queue(
                    Some("@procurali"),
                    Some(TelegramDeliveryStatus::Queued),
                    None,
                    10,
                )
            })
            .unwrap();
        assert_eq!(q_drop.len(), 1);
        assert!(q_drop[0].message_text.contains("📉 BAIXOU DE PREÇO"));

        store
            .transaction(|repo| {
                assert!(repo.set_ml_telegram_delivery_status(
                    &q_drop[0].delivery_id,
                    TelegramDeliveryStatus::Queued,
                    TelegramDeliveryStatus::Sending,
                    None,
                    None,
                    day2b + 59_000,
                )?);
                repo.set_ml_telegram_delivery_status(
                    &q_drop[0].delivery_id,
                    TelegramDeliveryStatus::Sending,
                    TelegramDeliveryStatus::Published,
                    Some("102"),
                    None,
                    day2b + 60_000,
                )
            })
            .unwrap();

        // 4. Later price increase -> Up, recorded in history, NOT re-queued
        let day3 = day1 + 2 * 86_400_000;
        let r4 = store
            .transaction(|repo| {
                repo.save_ml_ingest_batch(
                    "b4",
                    "i4",
                    &"5".repeat(64),
                    &make_req(27_900, day3),
                    day3,
                )
            })
            .unwrap();
        assert_eq!(r4.price_increase_count, 1);
        assert_eq!(r4.telegram_queued_count, 0);
        assert_eq!(r4.comparisons[0].trend, PriceTrendKind::Up);
        assert_eq!(r4.comparisons[0].price_delta_cents, 3_000);
        assert_eq!(r4.comparisons[0].observation_count, 3);

        let cat4 = store.read(|repo| repo.ml_site_catalog(day3)).unwrap();
        assert_eq!(cat4.offers[0].price_observations.len(), 3);
        let stats = store.read(|repo| repo.ml_telegram_product_stats()).unwrap();
        assert_eq!(
            stats.get("MLB63816534"),
            Some(&(2u32, Some("published".to_string())))
        );
    }
}
