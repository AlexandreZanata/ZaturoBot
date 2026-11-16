CREATE TABLE ml_ingest_batches (
    batch_id TEXT PRIMARY KEY,
    idempotency_key TEXT NOT NULL UNIQUE,
    worker_id TEXT NOT NULL,
    search_query TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    captured_at_ms INTEGER NOT NULL CHECK(captured_at_ms >= 0),
    ingested_at_ms INTEGER NOT NULL CHECK(ingested_at_ms >= 0),
    product_count INTEGER NOT NULL CHECK(product_count >= 1),
    image_count INTEGER NOT NULL CHECK(image_count >= 1),
    payload_sha256 TEXT NOT NULL CHECK(length(payload_sha256) = 64),
    report_json TEXT NOT NULL
);

CREATE TABLE ml_products (
    product_id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    brand TEXT NOT NULL,
    model TEXT,
    exact_variant TEXT NOT NULL,
    category_slug TEXT NOT NULL,
    canonical_url TEXT NOT NULL,
    seller_name TEXT NOT NULL,
    marketplace_rating TEXT,
    marketplace_sales_label TEXT,
    card_badge TEXT,
    return_days INTEGER CHECK(return_days IS NULL OR return_days >= 0),
    short_description TEXT NOT NULL,
    long_description TEXT NOT NULL,
    one_line_reason TEXT NOT NULL,
    for_whom TEXT NOT NULL,
    supported_benefits_json TEXT NOT NULL,
    limitations_json TEXT NOT NULL,
    unknowns_json TEXT NOT NULL,
    tags_json TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('candidate','approved','stale','withdrawn')),
    last_batch_id TEXT NOT NULL REFERENCES ml_ingest_batches(batch_id),
    created_at_ms INTEGER NOT NULL CHECK(created_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms >= 0)
);
CREATE INDEX ml_products_category ON ml_products(category_slug, state, updated_at_ms DESC, product_id);

CREATE TABLE ml_offers (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES ml_products(product_id) ON DELETE CASCADE,
    batch_id TEXT NOT NULL REFERENCES ml_ingest_batches(batch_id),
    share_link TEXT NOT NULL,
    share_id TEXT NOT NULL,
    share_text TEXT NOT NULL,
    commission_label TEXT,
    commission_bp INTEGER CHECK(commission_bp IS NULL OR (commission_bp >= 0 AND commission_bp <= 10000)),
    sponsored INTEGER NOT NULL CHECK(sponsored IN (0, 1)),
    price_brl_cents INTEGER NOT NULL CHECK(price_brl_cents > 0),
    pix_price_cents INTEGER NOT NULL CHECK(pix_price_cents > 0),
    reference_price_cents INTEGER CHECK(reference_price_cents IS NULL OR reference_price_cents > 0),
    currency TEXT NOT NULL CHECK(currency = 'BRL'),
    discount_label TEXT,
    installment_summary TEXT,
    payment_conditions TEXT NOT NULL,
    short_condition TEXT NOT NULL,
    freight_status TEXT NOT NULL,
    availability TEXT NOT NULL,
    price_valid_until TEXT,
    observed_at_ms INTEGER NOT NULL CHECK(observed_at_ms >= 0),
    freshness_ttl_minutes INTEGER NOT NULL CHECK(freshness_ttl_minutes >= 1),
    evidence_sha256 TEXT NOT NULL CHECK(length(evidence_sha256) = 64),
    is_current INTEGER NOT NULL CHECK(is_current IN (0, 1)),
    created_at_ms INTEGER NOT NULL CHECK(created_at_ms >= 0)
);
CREATE UNIQUE INDEX ml_offers_one_current ON ml_offers(product_id) WHERE is_current = 1;
CREATE INDEX ml_offers_history ON ml_offers(product_id, observed_at_ms DESC, id);

CREATE TABLE ml_product_specs (
    product_id TEXT NOT NULL REFERENCES ml_products(product_id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK(position >= 0),
    spec_name TEXT NOT NULL,
    spec_value TEXT NOT NULL,
    PRIMARY KEY(product_id, position),
    UNIQUE(product_id, spec_name)
);

CREATE TABLE ml_product_variants (
    product_id TEXT NOT NULL REFERENCES ml_products(product_id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK(position >= 0),
    variant_product_id TEXT NOT NULL,
    sku TEXT NOT NULL,
    name TEXT NOT NULL,
    color TEXT,
    voltage TEXT,
    price_brl_cents INTEGER NOT NULL CHECK(price_brl_cents > 0),
    currency TEXT NOT NULL CHECK(currency = 'BRL'),
    availability TEXT NOT NULL,
    return_days INTEGER CHECK(return_days IS NULL OR return_days >= 0),
    return_fees TEXT,
    canonical_url TEXT,
    highlights_json TEXT NOT NULL,
    PRIMARY KEY(product_id, position),
    UNIQUE(product_id, variant_product_id, sku)
);

CREATE TABLE ml_product_images (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES ml_products(product_id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK(position >= 0),
    minio_bucket TEXT NOT NULL,
    minio_object_key TEXT NOT NULL,
    public_url TEXT NOT NULL,
    source_origin_url TEXT,
    content_type TEXT NOT NULL CHECK(content_type IN ('image/jpeg','image/png','image/webp')),
    sha256_hex TEXT NOT NULL CHECK(length(sha256_hex) = 64),
    size_bytes INTEGER NOT NULL CHECK(size_bytes > 0),
    alt_text TEXT NOT NULL,
    rights_verified INTEGER NOT NULL CHECK(rights_verified IN (0, 1)),
    attribution_text TEXT NOT NULL,
    uploaded_at_ms INTEGER NOT NULL CHECK(uploaded_at_ms >= 0),
    UNIQUE(product_id, position),
    UNIQUE(product_id, minio_object_key)
);

CREATE TABLE ml_telegram_deliveries (
    delivery_id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES ml_products(product_id) ON DELETE CASCADE,
    offer_id TEXT NOT NULL REFERENCES ml_offers(id) ON DELETE CASCADE,
    channel_id TEXT NOT NULL,
    message_text TEXT NOT NULL,
    image_public_url TEXT,
    image_object_key TEXT,
    status TEXT NOT NULL CHECK(status IN ('queued','published','skipped','failed')),
    external_message_id TEXT,
    error_code TEXT,
    created_at_ms INTEGER NOT NULL CHECK(created_at_ms >= 0),
    updated_at_ms INTEGER NOT NULL CHECK(updated_at_ms >= 0),
    published_at_ms INTEGER CHECK(published_at_ms IS NULL OR published_at_ms >= 0),
    UNIQUE(offer_id, channel_id)
);
CREATE INDEX ml_telegram_deliveries_status ON ml_telegram_deliveries(status, channel_id, updated_at_ms DESC, delivery_id);

PRAGMA user_version = 10;
