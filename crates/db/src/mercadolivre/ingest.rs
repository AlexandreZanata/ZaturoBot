use super::ThirtyDayDedupState;
use crate::{Error, Repository};
use rusqlite::{OptionalExtension, params};
use zaturo_core::{
    MercadoLivreIngestBatchRequest, MercadoLivreIngestReport, MercadoLivrePriceComparison,
    MercadoLivreProductState, PriceTrendKind, ValidatedMercadoLivreProduct,
    classify_product_subcategory, format_price_comparison_summary,
    format_telegram_caption_with_price_history_pt_br,
};

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
}
