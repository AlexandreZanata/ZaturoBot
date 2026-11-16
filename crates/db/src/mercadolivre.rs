//! Relational persistence for ingested Mercado Livre products, specs, variants, MinIO images,
//! price observations, static site catalog snapshots, and Telegram delivery queues.

use crate::{Error, Repository};
use rusqlite::{OptionalExtension, params};
use std::collections::BTreeSet;
use zaturo_core::{
    MercadoLivreIngestBatchRequest, MercadoLivreIngestReport, MercadoLivreProductState,
    MercadoLivreSpec, MercadoLivreVariant, MinioProductImage, SiteCatalogCategory,
    SiteCatalogEvidenceSource, SiteCatalogGalleryItem, SiteCatalogMediaRight, SiteCatalogOffer,
    SiteCatalogPriceObservation, SiteCatalogSnapshot, TelegramDeliveryAck, TelegramDeliveryStatus,
    TelegramOfferQueueItem, ValidatedMercadoLivreProduct, format_cuiaba_display,
    format_cuiaba_short, format_telegram_caption_pt_br, format_utc_iso_ms,
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
        let telegram_queued_count = request
            .products
            .iter()
            .filter(|p| p.state == MercadoLivreProductState::Approved)
            .count();

        let report = MercadoLivreIngestReport {
            batch_id: batch_id.to_owned(),
            worker_id: request.worker_id.clone(),
            ingested_at_ms: now_ms,
            product_count: request.products.len(),
            image_count,
            telegram_queued_count,
            product_ids,
            payload_sha256: payload_sha256.to_owned(),
        };
        let report_json = serde_json::to_string(&report)?;

        self.connection.execute(
            "INSERT INTO ml_ingest_batches(
                batch_id, idempotency_key, worker_id, search_query, channel_id,
                captured_at_ms, ingested_at_ms, product_count, image_count,
                payload_sha256, report_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
                payload_sha256,
                report_json
            ],
        )?;

        for product in &request.products {
            self.upsert_ml_product(batch_id, &request.channel_id, product, now_ms)?;
        }

        Ok(report)
    }

    fn upsert_ml_product(
        &self,
        batch_id: &str,
        channel_id: &str,
        product: &ValidatedMercadoLivreProduct,
        now_ms: i64,
    ) -> Result<(), Error> {
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

        if product.state == MercadoLivreProductState::Approved {
            let lead = product
                .images
                .iter()
                .find(|img| matches!(img.content_type.as_str(), "image/jpeg" | "image/png"))
                .unwrap_or(&product.images[0]);
            let delivery_id = format!("tg-{}-{}", product.product_id, batch_id);
            let message_text = format_telegram_caption_pt_br(product);
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
        }

        Ok(())
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

            let images = self.ml_product_images(&product_id)?;
            if images.is_empty() {
                continue;
            }
            let specs = self.ml_product_specs(&product_id)?;
            let variants = self.ml_product_variants(&product_id)?;
            let price_observations = self.ml_price_observations(&product_id, &seller_name)?;
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

            let lead = &images[0];
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

            offers.push(SiteCatalogOffer {
                id: format!("offer-ml-{product_id}"),
                slug,
                is_demo_fixture: false,
                product_id,
                brand,
                model,
                title,
                exact_variant,
                category_slug,
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
                marketplace_sales_label,
                card_badge,
                discount_percent_label: discount_label,
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
            });
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

    fn ml_product_specs(&self, product_id: &str) -> Result<Vec<MercadoLivreSpec>, Error> {
        let mut stmt = self.connection.prepare(
            "SELECT spec_name, spec_value FROM ml_product_specs WHERE product_id = ?1 ORDER BY position ASC",
        )?;
        let rows = stmt.query_map([product_id], |r| {
            Ok(MercadoLivreSpec {
                name: r.get(0)?,
                value: r.get(1)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }

    fn ml_product_variants(&self, product_id: &str) -> Result<Vec<MercadoLivreVariant>, Error> {
        let mut stmt = self.connection.prepare(
            "SELECT variant_product_id, sku, name, color, voltage, price_brl_cents, currency,
                    availability, return_days, return_fees, canonical_url, highlights_json
             FROM ml_product_variants WHERE product_id = ?1 ORDER BY position ASC",
        )?;
        let rows = stmt.query_map([product_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, Option<i64>>(8)?,
                r.get::<_, Option<String>>(9)?,
                r.get::<_, Option<String>>(10)?,
                r.get::<_, String>(11)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (
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
            out.push(MercadoLivreVariant {
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
        Ok(out)
    }

    fn ml_product_images(&self, product_id: &str) -> Result<Vec<MinioProductImage>, Error> {
        let mut stmt = self.connection.prepare(
            "SELECT position, minio_bucket, minio_object_key, public_url, source_origin_url,
                    content_type, sha256_hex, size_bytes, alt_text
             FROM ml_product_images WHERE product_id = ?1 ORDER BY position ASC",
        )?;
        let rows = stmt.query_map([product_id], |r| {
            Ok(MinioProductImage {
                position: u16::try_from(r.get::<_, i64>(0)?).unwrap_or(0),
                bucket: r.get(1)?,
                object_key: r.get(2)?,
                public_url: r.get(3)?,
                source_origin_url: r.get(4)?,
                content_type: r.get(5)?,
                sha256_hex: r.get(6)?,
                size_bytes: u64::try_from(r.get::<_, i64>(7)?).unwrap_or(0),
                alt_text: r.get(8)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }

    fn ml_price_observations(
        &self,
        product_id: &str,
        seller_name: &str,
    ) -> Result<Vec<SiteCatalogPriceObservation>, Error> {
        let mut stmt = self.connection.prepare(
            "SELECT id, payment_conditions, short_condition, price_brl_cents, observed_at_ms
             FROM ml_offers WHERE product_id = ?1 ORDER BY observed_at_ms DESC, id DESC LIMIT 30",
        )?;
        let offer_id = format!("offer-ml-{product_id}");
        let evidence_id = format!("ev-ml-{product_id}");
        let rows = stmt.query_map([product_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })?;
        let mut obs = Vec::new();
        for row in rows {
            let (id, payment_basis, freight_note, price_brl_cents, observed_at_ms) = row?;
            obs.push(SiteCatalogPriceObservation {
                id,
                offer_id: offer_id.clone(),
                variant_id: product_id.to_owned(),
                seller_name: seller_name.to_owned(),
                payment_basis,
                freight_note,
                price_brl_cents,
                observed_at: format_utc_iso_ms(observed_at_ms),
                evidence_id: evidence_id.clone(),
            });
        }
        Ok(obs)
    }

    pub fn ml_telegram_queue(
        &self,
        channel_id: Option<&str>,
        status: Option<TelegramDeliveryStatus>,
        limit: u32,
    ) -> Result<Vec<TelegramOfferQueueItem>, Error> {
        let status_str = status.map(|s| s.as_str().to_owned());
        let mut stmt = self.connection.prepare(
            "SELECT
                d.delivery_id, d.product_id, d.offer_id, d.channel_id,
                p.title, p.brand, p.model,
                o.price_brl_cents, o.reference_price_cents, o.discount_label,
                o.share_link, o.share_id,
                d.image_public_url, d.image_object_key, d.message_text,
                d.status, d.external_message_id, d.created_at_ms, d.updated_at_ms, d.published_at_ms
             FROM ml_telegram_deliveries d
             JOIN ml_products p ON p.product_id = d.product_id
             JOIN ml_offers o ON o.id = d.offer_id
             WHERE (?1 IS NULL OR d.channel_id = ?1)
               AND (?2 IS NULL OR d.status = ?2)
             ORDER BY d.updated_at_ms DESC, d.delivery_id DESC
             LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![channel_id, status_str, limit], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, Option<String>>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, Option<i64>>(8)?,
                r.get::<_, Option<String>>(9)?,
                r.get::<_, String>(10)?,
                r.get::<_, String>(11)?,
                r.get::<_, Option<String>>(12)?,
                r.get::<_, Option<String>>(13)?,
                r.get::<_, String>(14)?,
                r.get::<_, String>(15)?,
                r.get::<_, Option<String>>(16)?,
                r.get::<_, i64>(17)?,
                r.get::<_, i64>(18)?,
                r.get::<_, Option<i64>>(19)?,
            ))
        })?;
        let mut items = Vec::new();
        for row in rows {
            let (
                delivery_id,
                product_id,
                offer_id,
                channel_id,
                title,
                brand,
                model,
                price_brl_cents,
                reference_price_cents,
                discount_label,
                share_link,
                share_id,
                image_public_url,
                image_object_key,
                message_text,
                status_raw,
                external_message_id,
                created_at_ms,
                updated_at_ms,
                published_at_ms,
            ) = row?;
            let status = TelegramDeliveryStatus::parse(&status_raw).ok_or(Error::SchemaVersion)?;
            items.push(TelegramOfferQueueItem {
                delivery_id,
                product_id,
                offer_id,
                channel_id,
                title,
                brand,
                model,
                price_brl_cents,
                reference_price_cents,
                discount_label,
                share_link,
                share_id,
                image_public_url,
                image_object_key,
                message_text,
                status,
                external_message_id,
                created_at_ms,
                updated_at_ms,
                published_at_ms,
            });
        }
        Ok(items)
    }

    pub fn ack_ml_telegram_delivery(
        &self,
        delivery_id: &str,
        ack: &TelegramDeliveryAck,
        now_ms: i64,
    ) -> Result<Option<TelegramOfferQueueItem>, Error> {
        let published_at_ms = if ack.status == TelegramDeliveryStatus::Published {
            Some(now_ms)
        } else {
            None
        };
        let changed = self.connection.execute(
            "UPDATE ml_telegram_deliveries
             SET status = ?2,
                 external_message_id = COALESCE(?3, external_message_id),
                 error_code = ?4,
                 updated_at_ms = ?5,
                 published_at_ms = COALESCE(?6, published_at_ms)
             WHERE delivery_id = ?1",
            params![
                delivery_id,
                ack.status.as_str(),
                ack.external_message_id,
                ack.error_code,
                now_ms,
                published_at_ms
            ],
        )?;
        if changed == 0 {
            return Ok(None);
        }
        let items = self.ml_telegram_queue(None, None, 50)?;
        Ok(items
            .into_iter()
            .find(|item| item.delivery_id == delivery_id))
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
        let req = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "ssd nvme".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: 1_763_231_280_000,
            products: vec![sample_product(
                "MLB39410080",
                "ssd-kingston-nv3-500gb-mlb39410080",
                69_990,
            )],
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
        assert_eq!(report.telegram_queued_count, 1);

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

        let queued = store
            .read(|repo| {
                repo.ml_telegram_queue(Some("@procurali"), Some(TelegramDeliveryStatus::Queued), 20)
            })
            .unwrap();
        assert_eq!(queued.len(), 1);
        assert!(queued[0].message_text.contains("https://meli.la/1iWQZbJ"));

        let updated = store
            .transaction(|repo| {
                repo.ack_ml_telegram_delivery(
                    &queued[0].delivery_id,
                    &TelegramDeliveryAck {
                        status: TelegramDeliveryStatus::Published,
                        external_message_id: Some("98765".into()),
                        error_code: None,
                    },
                    1_763_231_400_000,
                )
            })
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, TelegramDeliveryStatus::Published);
        assert_eq!(updated.external_message_id.as_deref(), Some("98765"));
    }
}
