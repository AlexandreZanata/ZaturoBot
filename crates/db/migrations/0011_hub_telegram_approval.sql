-- Per-offer owner approval and a pre-send claim for Telegram deliveries.
-- `awaiting_approval` replaces implicit batch approval; `sending` is set before the external
-- send so an uncertain send is never repeated automatically.
CREATE TABLE ml_telegram_deliveries_v11 (
    delivery_id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES ml_products(product_id) ON DELETE CASCADE,
    offer_id TEXT NOT NULL REFERENCES ml_offers(id) ON DELETE CASCADE,
    channel_id TEXT NOT NULL,
    message_text TEXT NOT NULL,
    image_public_url TEXT,
    image_object_key TEXT,
    status TEXT NOT NULL CHECK(status IN (
        'awaiting_approval','queued','sending','published','skipped','failed'
    )),
    external_message_id TEXT,
    error_code TEXT,
    created_at_ms INTEGER NOT NULL CHECK(created_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms >= 0),
    published_at_ms INTEGER CHECK(published_at_ms IS NULL OR published_at_ms >= 0),
    UNIQUE(offer_id, channel_id)
);

-- Version 10 queued every ingested offer without owner review. Those rows return to
-- awaiting approval; rows whose offer is no longer current are superseded.
INSERT INTO ml_telegram_deliveries_v11(
    delivery_id, product_id, offer_id, channel_id, message_text, image_public_url,
    image_object_key, status, external_message_id, error_code, created_at_ms,
    updated_at_ms, published_at_ms
)
SELECT
    d.delivery_id, d.product_id, d.offer_id, d.channel_id, d.message_text,
    d.image_public_url, d.image_object_key,
    CASE
        WHEN d.status <> 'queued' THEN d.status
        WHEN EXISTS (SELECT 1 FROM ml_offers o WHERE o.id = d.offer_id AND o.is_current = 1)
            THEN 'awaiting_approval'
        ELSE 'skipped'
    END,
    d.external_message_id,
    CASE
        WHEN d.status = 'queued'
            AND NOT EXISTS (
                SELECT 1 FROM ml_offers o WHERE o.id = d.offer_id AND o.is_current = 1
            )
            THEN 'superseded'
        ELSE d.error_code
    END,
    d.created_at_ms, d.updated_at_ms, d.published_at_ms
FROM ml_telegram_deliveries d;

DROP TABLE ml_telegram_deliveries;
ALTER TABLE ml_telegram_deliveries_v11 RENAME TO ml_telegram_deliveries;
CREATE INDEX ml_telegram_deliveries_status ON ml_telegram_deliveries(status, channel_id, updated_at_ms DESC, delivery_id);
CREATE INDEX ml_telegram_deliveries_product ON ml_telegram_deliveries(product_id, channel_id, status);

PRAGMA user_version = 11;
