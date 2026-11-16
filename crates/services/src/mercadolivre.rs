//! Transactional Mercado Livre product ingestion, static-site catalog projection,
//! and Telegram delivery queue orchestration.

use crate::{Engine, Error, Result};
use uuid::Uuid;
use zaturo_core::{
    FeedItem, FeedKind, MercadoLivreIngestBatchRequest, MercadoLivreIngestReport,
    SiteCatalogSnapshot, TelegramDeliveryAck, TelegramDeliveryStatus, TelegramOfferQueueItem,
    valid_sha256, valid_telegram_channel,
};

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

        self.store.transaction(|repo| {
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
                    "Mercado Livre batch {} ingested ({} products, {} images, {} Telegram queued)",
                    report.batch_id,
                    report.product_count,
                    report.image_count,
                    report.telegram_queued_count
                ),
                task_id: None,
                artifact_id: None,
                sensitive: false,
                capture: None,
            };
            repo.insert_feed(&feed_item, &format!("ml-ingest:{}", report.batch_id))?;
            Ok(report)
        })
    }

    /// Build the normalized Procurali static-site catalog snapshot from relational storage.
    pub fn mercadolivre_site_catalog(&self, now_ms: i64) -> Result<SiteCatalogSnapshot> {
        if now_ms <= 0 {
            return Err(Error::Invalid("now_ms must be positive".into()));
        }
        self.store.read(|repo| Ok(repo.ml_site_catalog(now_ms)?))
    }

    /// List bounded Telegram delivery queue items filtered by optional channel and status.
    pub fn mercadolivre_telegram_queue(
        &self,
        channel_id: Option<&str>,
        status: Option<TelegramDeliveryStatus>,
        limit: u32,
    ) -> Result<Vec<TelegramOfferQueueItem>> {
        if let Some(ch) = channel_id
            && !valid_telegram_channel(ch)
        {
            return Err(Error::Invalid("invalid Telegram channel_id".into()));
        }
        self.page(limit, 0)?;
        self.store
            .read(|repo| Ok(repo.ml_telegram_queue(channel_id, status, limit)?))
    }

    /// Acknowledge a Telegram delivery attempt (published, failed, skipped, or queued).
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

        self.store.transaction(|repo| {
            repo.ack_ml_telegram_delivery(delivery_id, &ack, now_ms)?
                .ok_or(Error::NotFound)
        })
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

        let req = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "ssd nvme".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: 1_763_231_280_000,
            products: vec![sample_product(
                "MLB39410080",
                "ssd-kingston-nv3-500gb-mlb39410080",
            )],
        };
        let sha_a = "c".repeat(64);
        let sha_b = "d".repeat(64);

        let first = engine
            .ingest_mercadolivre_batch(req.clone(), "batch-idem-1", &sha_a, 1_763_231_290_000)
            .unwrap();
        assert_eq!(first.product_count, 1);
        assert_eq!(first.telegram_queued_count, 1);

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

        let queue = engine
            .mercadolivre_telegram_queue(
                Some("@procurali"),
                Some(TelegramDeliveryStatus::Queued),
                20,
            )
            .unwrap();
        assert_eq!(queue.len(), 1);

        let acked = engine
            .ack_mercadolivre_telegram_delivery(
                &queue[0].delivery_id,
                TelegramDeliveryAck {
                    status: TelegramDeliveryStatus::Published,
                    external_message_id: Some("msg-42".into()),
                    error_code: None,
                },
                1_763_231_350_000,
            )
            .unwrap();
        assert_eq!(acked.status, TelegramDeliveryStatus::Published);
        assert_eq!(acked.external_message_id.as_deref(), Some("msg-42"));
    }
}
