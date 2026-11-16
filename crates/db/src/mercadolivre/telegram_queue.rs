use crate::{Error, Repository};
use rusqlite::{OptionalExtension, params};
use std::collections::HashMap;
use zaturo_core::{TelegramDeliveryStatus, TelegramOfferQueueItem};

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

impl Repository<'_> {
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
