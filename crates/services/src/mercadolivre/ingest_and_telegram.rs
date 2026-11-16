use crate::{Engine, Error, Result};
use uuid::Uuid;
use zaturo_core::{
    FeedItem, FeedKind, MercadoLivreIngestBatchRequest, MercadoLivreIngestReport,
    TelegramDeliveryAck, TelegramDeliveryStatus, TelegramOfferQueueItem, valid_sha256,
    valid_telegram_channel, validate_ml_product_id,
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
