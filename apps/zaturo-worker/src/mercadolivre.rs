//! Worker-side Mercado Livre batch ingestor: validates raw scraper JSON, filters mobile vertical
//! video frames in favor of product photography, uploads verified images to MinIO, strips local
//! filesystem paths at the worker boundary, and ingests idempotent batches into the Hub API.

use anyhow::{Context, Result, bail, ensure};
use serde_json::json;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::Duration,
};
use zaturo_core::{
    DEFAULT_ML_FRESHNESS_TTL_MINUTES, MAX_ML_BATCH_PRODUCTS, MAX_ML_IMAGES_PER_PRODUCT,
    MAX_ML_SPECS_PER_PRODUCT, MAX_ML_VARIANTS_PER_PRODUCT, MercadoLivreBotBatch,
    MercadoLivreBotProduct, MercadoLivreIngestBatchRequest, MercadoLivreIngestReport,
    MercadoLivreProductState, MercadoLivreSpec, MercadoLivreVariant, MinioProductImage,
    ValidatedMercadoLivreProduct, extract_product_id_from_url, format_brl_cents,
    no_local_path_leak, parse_brl_amount_cents, parse_card_text, parse_commission_bp,
    parse_iso_to_ms, parse_jsonld_variant, parse_price_value_cents, slugify_pt_br, valid_slug,
    valid_telegram_channel, validate_asset_public_url, validate_meli_share_link,
    validate_ml_canonical_url, validate_ml_product_id, validate_ml_share_id,
};
use zaturo_transfer::minio::{
    InspectedImage, MinioClient, inspect_image_bytes, read_image_file, sha256_hex,
};

use crate::config::WorkerConfig;

#[derive(Debug, Clone)]
pub struct BatchIngestOptions {
    pub worker_id: String,
    pub default_category_slug: String,
    pub channel_id: String,
    pub now_ms: i64,
}

struct PreparedImage {
    bytes: Vec<u8>,
    inspected: InspectedImage,
    origin_url: Option<String>,
}

/// Parses a bot-emitted `products.json` payload, uploads non-vertical product photos to MinIO,
/// normalizes structured specs/variants, flags editorial contradictions, and returns an
/// authenticated-ready `MercadoLivreIngestBatchRequest` with zero local filesystem path leaks.
pub async fn prepare_and_upload_batch(
    raw_json_bytes: &[u8],
    minio: &MinioClient,
    options: &BatchIngestOptions,
    source_base_dir: Option<&Path>,
) -> Result<MercadoLivreIngestBatchRequest> {
    zaturo_core::validate_identifier(&options.worker_id).map_err(anyhow::Error::msg)?;
    ensure!(
        valid_slug(&options.default_category_slug),
        "invalid default_category_slug: {}",
        options.default_category_slug
    );
    ensure!(
        valid_telegram_channel(&options.channel_id),
        "invalid Telegram channel_id: {}",
        options.channel_id
    );
    ensure!(options.now_ms > 0, "now_ms must be positive");

    let batch: MercadoLivreBotBatch =
        serde_json::from_slice(raw_json_bytes).context("invalid Mercado Livre bot batch JSON")?;
    let network_lower = batch.network.trim().to_ascii_lowercase();
    ensure!(
        network_lower.contains("mercadolivre")
            || network_lower.contains("mercado livre")
            || (network_lower.is_empty() && batch.schema == Some(1)),
        "unsupported commercial batch network: {}",
        batch.network
    );
    ensure!(
        !batch.products.is_empty() && batch.products.len() <= MAX_ML_BATCH_PRODUCTS,
        "batch product count must be between 1 and {MAX_ML_BATCH_PRODUCTS}"
    );

    let captured_at_ms = parse_iso_to_ms(&batch.captured_at)
        .unwrap_or(options.now_ms)
        .max(1);
    let now_unix_secs = (options.now_ms / 1000).max(1);

    minio.ensure_bucket(now_unix_secs).await?;

    let search_query = clean_or_fallback(&batch.query, 160, "mercado livre hardware");
    ensure!(
        no_local_path_leak(&search_query),
        "search_query contained forbidden path content"
    );

    let mut validated_products = Vec::with_capacity(batch.products.len());
    for raw in &batch.products {
        let product = normalize_and_upload_product(
            raw,
            minio,
            &options.default_category_slug,
            captured_at_ms,
            options.now_ms,
            now_unix_secs,
            source_base_dir,
        )
        .await?;
        validated_products.push(product);
    }

    let request = MercadoLivreIngestBatchRequest {
        worker_id: options.worker_id.clone(),
        search_query,
        channel_id: options.channel_id.clone(),
        captured_at_ms,
        products: validated_products,
    };
    request.validates().map_err(anyhow::Error::msg)?;
    Ok(request)
}

/// Posts a validated batch to `{hub_url}/v1/device/commerce/mercadolivre/ingest` without
/// following HTTP redirects or logging tokens.
pub async fn ingest_to_hub(
    hub_url: &str,
    worker_token: &str,
    request: &MercadoLivreIngestBatchRequest,
    idempotency_key: &str,
) -> Result<MercadoLivreIngestReport> {
    zaturo_security::validate_endpoint(hub_url)?;
    zaturo_security::Credential::new(worker_token)?;
    ensure!(
        !idempotency_key.is_empty()
            && idempotency_key.len() <= 128
            && idempotency_key.bytes().all(|b| b.is_ascii_graphic()),
        "idempotency_key must be 1..=128 ASCII graphic characters"
    );
    request.validates().map_err(anyhow::Error::msg)?;

    let body = serde_json::to_vec(request)?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let url = format!(
        "{}/v1/device/commerce/mercadolivre/ingest",
        hub_url.trim_end_matches('/')
    );
    let response = client
        .post(url)
        .bearer_auth(worker_token)
        .header("Idempotency-Key", idempotency_key)
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .context("failed sending Mercado Livre ingest request to Hub")?;

    let status = response.status();
    if !status.is_success() {
        let error_snippet = response.text().await.unwrap_or_default();
        bail!(
            "Hub rejected Mercado Livre ingest with HTTP {} ({})",
            status.as_u16(),
            truncate_utf8(&error_snippet, 200)
        );
    }

    let report: MercadoLivreIngestReport = response
        .json()
        .await
        .context("invalid JSON response from Hub ingest endpoint")?;
    Ok(report)
}

/// Reads a bot JSON file from disk, uploads its product photography to configured MinIO,
/// and ingests the validated batch into the configured Hub.
pub async fn ingest_file_to_hub(
    config: &WorkerConfig,
    worker_token: &str,
    input_path: &Path,
    options: &BatchIngestOptions,
    idempotency_key: Option<&str>,
    source_base_dir: Option<&Path>,
) -> Result<MercadoLivreIngestReport> {
    let minio_cfg = config
        .minio
        .clone()
        .context("[minio] configuration section is required for Mercado Livre ingestion")?;
    let minio = MinioClient::from_config(minio_cfg)?;
    let raw_bytes = std::fs::read(input_path)
        .with_context(|| format!("failed reading bot batch file {}", input_path.display()))?;
    let effective_base = source_base_dir.or_else(|| input_path.parent());
    let batch_req = prepare_and_upload_batch(&raw_bytes, &minio, options, effective_base).await?;

    let derived_key = match idempotency_key {
        Some(k) => k.to_owned(),
        None => {
            let digest = sha256_hex(&raw_bytes);
            format!("ml-file-{}", &digest[..24])
        }
    };
    ingest_to_hub(&config.hub_url, worker_token, &batch_req, &derived_key).await
}

/// Checks `input_path`'s SHA-256 digest against `state_file`, skipping ingestion when the payload
/// has not changed and atomically recording the new digest after a successful Hub ingest.
pub async fn ingest_file_if_changed(
    config: &WorkerConfig,
    worker_token: &str,
    input_path: &Path,
    state_file: &Path,
    options: &BatchIngestOptions,
    source_base_dir: Option<&Path>,
) -> Result<Option<MercadoLivreIngestReport>> {
    let raw_bytes = std::fs::read(input_path)
        .with_context(|| format!("failed reading bot batch file {}", input_path.display()))?;
    let digest = sha256_hex(&raw_bytes);
    if let Ok(previous) = std::fs::read_to_string(state_file)
        && previous.trim() == digest
    {
        return Ok(None);
    }

    let idempotency_key = format!("ml-watch-{}", &digest[..24]);
    let report = ingest_file_to_hub(
        config,
        worker_token,
        input_path,
        options,
        Some(&idempotency_key),
        source_base_dir,
    )
    .await?;

    if let Some(parent) = state_file.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let tmp_path = state_file.with_extension("sha256.tmp");
    std::fs::write(&tmp_path, format!("{digest}\n"))?;
    std::fs::rename(&tmp_path, state_file)?;
    Ok(Some(report))
}

async fn normalize_and_upload_product(
    raw: &MercadoLivreBotProduct,
    minio: &MinioClient,
    default_category_slug: &str,
    observed_at_ms: i64,
    _now_ms: i64,
    now_unix_secs: i64,
    source_base_dir: Option<&Path>,
) -> Result<ValidatedMercadoLivreProduct> {
    let canonical_url = resolve_canonical_url(raw)?;
    let product_id = resolve_product_id(raw, &canonical_url)?;
    let title = clean_text(&raw.title, 220);
    ensure!(!title.is_empty(), "product {product_id} has empty title");

    let share_link = raw
        .share_link
        .as_deref()
        .map(str::trim)
        .filter(|s| validate_meli_share_link(s))
        .with_context(|| {
            format!("product {product_id} missing valid official https://meli.la/ share_link")
        })?
        .to_owned();

    let share_id = resolve_share_id(raw, &product_id);
    let share_text = resolve_share_text(raw, &share_id, &share_link);

    let slug = slugify_pt_br(&title, &product_id);
    ensure!(valid_slug(&slug), "generated invalid slug for {product_id}");

    let uploaded_images = upload_filtered_product_images(
        raw,
        &product_id,
        &title,
        minio,
        now_unix_secs,
        source_base_dir,
    )
    .await?;

    let parsed_card = raw
        .card_text
        .as_deref()
        .map(parse_card_text)
        .unwrap_or_default();

    let raw_price_cents = raw
        .price
        .as_ref()
        .and_then(parse_price_value_cents)
        .or_else(|| raw.meta_price.as_deref().and_then(parse_brl_amount_cents))
        .or(parsed_card.card_price_cents)
        .filter(|c| *c > 0)
        .with_context(|| format!("product {product_id} missing valid positive BRL price"))?;

    let pix_price_cents = parsed_card
        .card_price_cents
        .filter(|c| *c > 0 && *c <= raw_price_cents)
        .unwrap_or(raw_price_cents);

    let reference_price_cents = parsed_card
        .reference_price_cents
        .filter(|r| *r > raw_price_cents);

    let discount_label = parsed_card
        .discount_label
        .map(|d| clean_text(&d, 40))
        .filter(|d| !d.is_empty())
        .or_else(|| {
            reference_price_cents.map(|ref_c| {
                let pct = ((ref_c - pix_price_cents) * 100) / ref_c;
                format!("{pct}% OFF")
            })
        });

    let specs = normalize_specs(&raw.specs);
    let variants = normalize_variants(&raw.variants);

    let brand = raw
        .brand
        .as_deref()
        .map(|b| clean_text(b, 80))
        .filter(|b| !b.is_empty())
        .or_else(|| find_spec_value(&specs, "Marca").map(|v| clean_text(v, 80)))
        .unwrap_or_else(|| "Genérico".into());

    let model = raw
        .model
        .as_deref()
        .map(|m| clean_text(m, 80))
        .filter(|m| !m.is_empty() && no_local_path_leak(m))
        .or_else(|| find_spec_value(&specs, "Modelo").map(|v| clean_text(v, 80)));

    let seller_name = raw
        .seller
        .as_deref()
        .map(|s| clean_text(s, 140))
        .filter(|s| !s.is_empty() && no_local_path_leak(s))
        .unwrap_or_else(|| "Vendedor verificado no Mercado Livre".into());

    let commission_label = raw
        .commission
        .as_deref()
        .map(|c| clean_text(c, 32))
        .filter(|c| !c.is_empty() && no_local_path_leak(c));
    let commission_bp = commission_label.as_deref().and_then(parse_commission_bp);

    let exact_variant = build_exact_variant(model.as_deref(), &specs, &variants, &product_id);
    let availability = normalize_availability(raw.availability.as_deref());
    let price_valid_until = raw
        .price_valid_until
        .as_deref()
        .map(str::trim)
        .filter(|d| d.len() == 10 && d.as_bytes()[4] == b'-' && d.as_bytes()[7] == b'-')
        .map(str::to_owned);

    let payment_conditions = build_payment_conditions(
        raw_price_cents,
        pix_price_cents,
        parsed_card.installment_summary.as_deref(),
    );
    let short_condition = build_short_condition(
        raw_price_cents,
        pix_price_cents,
        parsed_card.installment_summary.as_deref(),
        raw.return_days,
    );
    let freight_status = match raw.return_days {
        Some(days) => format!("Frete calculado no CEP; devolução grátis em até {days} dias."),
        None => "Frete e prazo de entrega calculados no CEP durante o checkout.".into(),
    };

    let short_description = build_short_description(raw, &specs);
    let long_description = build_long_description(raw, &short_description);
    let one_line_reason = clean_or_fallback(&short_description, 320, &title);
    let for_whom = infer_for_whom(&title, &specs);

    let supported_benefits = build_supported_benefits(&specs, &short_description, &title);
    let limitations = build_honest_limitations(
        &title,
        &long_description,
        &specs,
        &variants,
        raw_price_cents,
        parsed_card.installment_summary.as_deref(),
        raw.card_text.as_deref(),
    );
    let unknowns = build_unknowns(raw.return_days, &specs);
    let tags = build_tags(&brand, &title, default_category_slug);

    let evidence_sha256 = sha256_hex(&serde_json::to_vec(&json!({
        "product_id": &product_id,
        "canonical_url": &canonical_url,
        "share_link": &share_link,
        "price_brl_cents": raw_price_cents,
        "pix_price_cents": pix_price_cents,
        "reference_price_cents": reference_price_cents,
        "specs": &specs,
        "variants": variants.iter().map(|v| (&v.variant_product_id, &v.sku, v.price_brl_cents)).collect::<Vec<_>>(),
        "image_hashes": uploaded_images.iter().map(|img| &img.sha256_hex).collect::<Vec<_>>(),
    }))?);

    let validated = ValidatedMercadoLivreProduct {
        product_id,
        slug,
        title,
        brand,
        model,
        exact_variant,
        category_slug: default_category_slug.to_owned(),
        canonical_url,
        seller_name,
        share_link,
        share_id,
        share_text,
        commission_label,
        commission_bp,
        sponsored: raw.sponsored,
        price_brl_cents: raw_price_cents,
        pix_price_cents,
        reference_price_cents,
        currency: "BRL".into(),
        marketplace_rating: parsed_card.marketplace_rating,
        marketplace_sales_label: parsed_card.marketplace_sales_label,
        card_badge: parsed_card.card_badge,
        discount_label,
        installment_summary: parsed_card.installment_summary.map(|s| clean_text(&s, 80)),
        payment_conditions,
        short_condition,
        freight_status,
        availability,
        price_valid_until,
        return_days: raw.return_days,
        short_description,
        long_description,
        one_line_reason,
        for_whom,
        supported_benefits,
        limitations,
        unknowns,
        tags,
        specs,
        variants,
        images: uploaded_images,
        observed_at_ms,
        freshness_ttl_minutes: DEFAULT_ML_FRESHNESS_TTL_MINUTES,
        evidence_sha256,
        state: MercadoLivreProductState::Approved,
    };
    validated.validates().map_err(anyhow::Error::msg)?;
    Ok(validated)
}

async fn upload_filtered_product_images(
    raw: &MercadoLivreBotProduct,
    product_id: &str,
    title: &str,
    minio: &MinioClient,
    now_unix_secs: i64,
    source_base_dir: Option<&Path>,
) -> Result<Vec<MinioProductImage>> {
    ensure!(
        !raw.photos.is_empty(),
        "product {product_id} has no local photos to upload to MinIO"
    );

    let mut prepared = Vec::new();
    let mut seen_hashes = BTreeSet::new();

    for (idx, raw_path) in raw.photos.iter().enumerate() {
        let candidate_path = resolve_photo_path(raw_path, source_base_dir);
        let bytes = read_image_file(&candidate_path)
            .with_context(|| format!("reading local product photo #{idx} for {product_id}"))?;
        let inspected = inspect_image_bytes(&bytes)
            .with_context(|| format!("inspecting product photo #{idx} for {product_id}"))?;
        if !seen_hashes.insert(inspected.sha256_hex.clone()) {
            continue;
        }
        let origin_url = raw
            .gallery
            .get(idx)
            .or(raw.image_url.as_ref())
            .or(raw.image.as_ref())
            .map(|u| u.trim().to_owned())
            .filter(|u| validate_asset_public_url(u) && !u.contains("matt_tool="));

        prepared.push(PreparedImage {
            bytes,
            inspected,
            origin_url,
        });
    }

    ensure!(
        !prepared.is_empty(),
        "product {product_id} yielded no valid images"
    );

    let has_non_vertical = prepared
        .iter()
        .any(|item| !item.inspected.is_vertical_video_frame());
    let selected: Vec<PreparedImage> = if has_non_vertical {
        prepared
            .into_iter()
            .filter(|item| !item.inspected.is_vertical_video_frame())
            .take(MAX_ML_IMAGES_PER_PRODUCT)
            .collect()
    } else {
        prepared
            .into_iter()
            .take(MAX_ML_IMAGES_PER_PRODUCT)
            .collect()
    };

    let mut uploaded = Vec::with_capacity(selected.len());
    for (pos, img) in selected.into_iter().enumerate() {
        let alt = if pos == 0 {
            clean_or_fallback(title, 220, product_id)
        } else {
            clean_or_fallback(&format!("{title} — imagem {}", pos + 1), 220, product_id)
        };
        let asset = minio
            .upload_product_image(
                product_id,
                u16::try_from(pos).unwrap_or(u16::MAX),
                &img.bytes,
                alt,
                img.origin_url,
                now_unix_secs,
            )
            .await?;
        uploaded.push(asset);
    }

    Ok(uploaded)
}

fn resolve_photo_path(raw_path: &str, source_base_dir: Option<&Path>) -> PathBuf {
    let p = Path::new(raw_path.trim());
    if p.exists() {
        return p.to_path_buf();
    }
    if let Some(base) = source_base_dir {
        if !p.is_absolute() {
            let joined = base.join(p);
            if joined.exists() {
                return joined;
            }
        }
        if let Some(name) = p.file_name() {
            for candidate in [base.join(name), base.join("images").join(name)] {
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }
    p.to_path_buf()
}

fn resolve_canonical_url(raw: &MercadoLivreBotProduct) -> Result<String> {
    let candidates = [raw.canonical_url.as_deref(), raw.product_url.as_deref()];
    for candidate in candidates.into_iter().flatten() {
        let stripped = candidate
            .trim()
            .split(['?', '#'])
            .next()
            .unwrap_or("")
            .trim();
        if validate_ml_canonical_url(stripped) {
            return Ok(stripped.to_owned());
        }
    }
    bail!(
        "product '{}' missing valid Mercado Livre canonical URL",
        raw.title
    )
}

fn resolve_product_id(raw: &MercadoLivreBotProduct, canonical_url: &str) -> Result<String> {
    if let Some(id) = raw.product_id.as_deref().map(str::trim)
        && validate_ml_product_id(id)
    {
        return Ok(id.to_owned());
    }
    extract_product_id_from_url(canonical_url)
        .or_else(|| {
            raw.product_url
                .as_deref()
                .and_then(extract_product_id_from_url)
        })
        .with_context(|| format!("unable to determine MLB product_id for '{}'", raw.title))
}

fn resolve_share_id(raw: &MercadoLivreBotProduct, product_id: &str) -> String {
    if let Some(id) = raw.share_id.as_deref().map(str::trim)
        && validate_ml_share_id(id)
    {
        return id.to_owned();
    }
    if let Some(text) = raw.share_text.as_deref() {
        for word in text.split_whitespace() {
            let trimmed = word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
            if trimmed.contains('-') && validate_ml_share_id(trimmed) {
                return trimmed.to_owned();
            }
        }
    }
    let digits = product_id.trim_start_matches("MLB");
    let fallback = format!("BOT-{digits}");
    truncate_utf8(&fallback, 32)
}

fn resolve_share_text(raw: &MercadoLivreBotProduct, share_id: &str, share_link: &str) -> String {
    let candidate = raw
        .share_text
        .as_deref()
        .map(|s| clean_text(s, 540))
        .unwrap_or_default();
    if candidate.contains(share_link) && no_local_path_leak(&candidate) {
        return candidate;
    }
    clean_or_fallback(&format!("Código {share_id}\n{share_link}"), 600, share_link)
}

fn normalize_specs(raw_specs: &[MercadoLivreSpec]) -> Vec<MercadoLivreSpec> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for spec in raw_specs {
        let name = clean_text(&spec.name, 120);
        let value = clean_text(&spec.value, 240);
        if name.is_empty()
            || value.is_empty()
            || !no_local_path_leak(&name)
            || !no_local_path_leak(&value)
        {
            continue;
        }
        if seen.insert(name.to_ascii_lowercase()) {
            out.push(MercadoLivreSpec { name, value });
            if out.len() >= MAX_ML_SPECS_PER_PRODUCT {
                break;
            }
        }
    }
    out
}

fn normalize_variants(raw_variants: &[serde_json::Value]) -> Vec<MercadoLivreVariant> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for raw in raw_variants {
        let Some(mut variant) = parse_jsonld_variant(raw) else {
            continue;
        };
        variant.name = clean_text(&variant.name, 260);
        if variant.name.is_empty()
            || !no_local_path_leak(&variant.name)
            || !no_local_path_leak(&variant.sku)
        {
            continue;
        }
        if seen.insert((variant.variant_product_id.clone(), variant.sku.clone())) {
            out.push(variant);
            if out.len() >= MAX_ML_VARIANTS_PER_PRODUCT {
                break;
            }
        }
    }
    out
}

fn find_spec_value<'a>(specs: &'a [MercadoLivreSpec], needle: &str) -> Option<&'a str> {
    let target = needle.to_ascii_lowercase();
    specs
        .iter()
        .find(|s| s.name.to_ascii_lowercase() == target)
        .map(|s| s.value.as_str())
}

fn build_exact_variant(
    model: Option<&str>,
    specs: &[MercadoLivreSpec],
    variants: &[MercadoLivreVariant],
    product_id: &str,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(m) = model {
        parts.push(m.to_owned());
    }
    for key in [
        "Capacidade",
        "Tamanho da tela",
        "Tipo de switch",
        "Conectividade",
    ] {
        if let Some(val) = find_spec_value(specs, key) {
            let trimmed = clean_text(val, 60);
            if !trimmed.is_empty() && !parts.contains(&trimmed) {
                parts.push(trimmed);
            }
        }
    }
    if parts.is_empty()
        && let Some(first) = variants.first()
    {
        parts.push(clean_text(&first.name, 120));
    }
    if parts.is_empty() {
        parts.push(product_id.to_owned());
    }
    clean_or_fallback(&parts.join(" · "), 280, product_id)
}

fn normalize_availability(raw: Option<&str>) -> String {
    raw.map(|s| s.trim().rsplit('/').next().unwrap_or("InStock"))
        .filter(|s| !s.is_empty() && s.len() <= 64 && no_local_path_leak(s))
        .unwrap_or("InStock")
        .to_owned()
}

fn build_payment_conditions(
    raw_price_cents: i64,
    pix_price_cents: i64,
    installment_summary: Option<&str>,
) -> String {
    let base = if pix_price_cents < raw_price_cents {
        format!(
            "{} no Pix ou {} no anúncio",
            format_brl_cents(pix_price_cents),
            format_brl_cents(raw_price_cents)
        )
    } else {
        format!("{} à vista", format_brl_cents(raw_price_cents))
    };
    let full = match installment_summary {
        Some(inst) => format!("{base} · {}", clean_text(inst, 80)),
        None => base,
    };
    truncate_utf8(&full, 280)
}

fn build_short_condition(
    raw_price_cents: i64,
    pix_price_cents: i64,
    installment_summary: Option<&str>,
    return_days: Option<u16>,
) -> String {
    let mut parts = Vec::new();
    if pix_price_cents < raw_price_cents {
        parts.push("Desconto no Pix".to_owned());
    } else if let Some(inst) = installment_summary {
        parts.push(clean_text(inst, 64));
    } else {
        parts.push("À vista no anúncio".to_owned());
    }
    if let Some(days) = return_days {
        parts.push(format!("devolução grátis {days}d"));
    }
    truncate_utf8(&parts.join(" · "), 160)
}

fn build_short_description(raw: &MercadoLivreBotProduct, specs: &[MercadoLivreSpec]) -> String {
    if let Some(short) = raw.short_description.as_deref() {
        let cleaned = clean_text(short, 280);
        if !cleaned.is_empty() && no_local_path_leak(&cleaned) {
            return cleaned;
        }
    }
    let summary_specs: Vec<String> = specs
        .iter()
        .take(3)
        .map(|s| format!("{}: {}", s.name, s.value))
        .collect();
    if !summary_specs.is_empty() {
        return truncate_utf8(&summary_specs.join(" · "), 280);
    }
    clean_or_fallback(&raw.title, 280, "Oferta observada no Mercado Livre")
}

fn build_long_description(raw: &MercadoLivreBotProduct, fallback: &str) -> String {
    if let Some(long) = raw.long_description.as_deref() {
        let cleaned = clean_text(long, 16_384);
        if !cleaned.is_empty() && no_local_path_leak(&cleaned) {
            return cleaned;
        }
    }
    fallback.to_owned()
}

fn infer_for_whom(title: &str, specs: &[MercadoLivreSpec]) -> String {
    let lower = title.to_ascii_lowercase();
    if lower.contains("ssd") || lower.contains("nvme") {
        "Quem quer acelerar notebook ou desktop com slot M.2 compatível.".into()
    } else if lower.contains("teclado") {
        "Quem busca teclado compacto/mecânico com layout e switches compatíveis com seu uso.".into()
    } else if lower.contains("monitor") {
        "Quem precisa de monitor para trabalho ou jogos com taxa de atualização e conexões compatíveis.".into()
    } else if lower.contains("headset") || lower.contains("fone") {
        "Quem busca headset ou fone para chamadas e jogos verificando peso e conexões.".into()
    } else if !specs.is_empty() {
        "Compradores que conferiram ficha técnica, vendedor e compatibilidade para o setup.".into()
    } else {
        "Quem precisa do item descrito conferindo medidas e condições do anúncio.".into()
    }
}

fn build_supported_benefits(
    specs: &[MercadoLivreSpec],
    short_description: &str,
    title: &str,
) -> Vec<String> {
    let mut benefits = Vec::new();
    for spec in specs.iter().take(4) {
        benefits.push(truncate_utf8(
            &format!("{}: {}", spec.name, spec.value),
            300,
        ));
    }
    if benefits.is_empty() {
        benefits.push(clean_or_fallback(short_description, 300, title));
    }
    benefits
}

fn build_honest_limitations(
    title: &str,
    long_description: &str,
    specs: &[MercadoLivreSpec],
    variants: &[MercadoLivreVariant],
    raw_price_cents: i64,
    installment_summary: Option<&str>,
    card_text: Option<&str>,
) -> Vec<String> {
    let mut limitations = Vec::new();

    // 1. Detect interest-bearing installments when card_text / installment summary lacks "sem juros"
    //    or when (N * installment) meaningfully exceeds cash price.
    if let Some(inst) = installment_summary {
        let inst_lower = inst.to_ascii_lowercase();
        let lacks_sem_juros = !inst_lower.contains("sem juros");
        let total_exceeds_cash =
            parse_installment_total_cents(inst).is_some_and(|total| total > raw_price_cents + 500);
        if lacks_sem_juros || total_exceeds_cash {
            limitations.push(
                "Parcelamento no anúncio inclui acréscimo/juros em relação ao preço à vista."
                    .into(),
            );
        }
    } else if card_text
        .map(|t| t.to_ascii_lowercase())
        .is_some_and(|t| t.contains("com juros"))
    {
        limitations.push(
            "Parcelamento informado no card pode incluir acréscimo em relação ao preço à vista."
                .into(),
        );
    }

    // 2. Detect spec vs description contradictions (water resistance, wireless claims).
    let combined_text = format!("{} {}", title, long_description).to_ascii_lowercase();
    let spec_waterproof_no = specs.iter().any(|s| {
        let n = s.name.to_ascii_lowercase();
        let v = s.value.to_ascii_lowercase();
        (n.contains("resistente à água") || n.contains("resistente a agua") || n.contains("prova"))
            && (v == "não" || v == "nao")
    });
    let text_claims_waterproof = combined_text.contains("resistente a salpicos")
        || combined_text.contains("resistente à água")
        || combined_text.contains("resistente a água")
        || combined_text.contains("à prova d'água");
    if spec_waterproof_no && text_claims_waterproof {
        limitations.push(
            "Divergência no anúncio: a ficha técnica marca resistência à água como 'Não', enquanto a descrição cita proteção contra salpicos."
                .into(),
        );
    }

    let spec_wireless_no = specs.iter().any(|s| {
        s.name.to_ascii_lowercase().contains("sem fio")
            && matches!(s.value.to_ascii_lowercase().as_str(), "não" | "nao")
    });
    if spec_wireless_no
        && (combined_text.contains("bluetooth") || combined_text.contains("tri-mode"))
    {
        limitations.push(
            "Divergência no anúncio: ficha técnica indica 'sem fio: Não', apesar de menção a modo sem fio na descrição."
                .into(),
        );
    }

    // 3. Detect multi-voltage variants requiring explicit selection.
    let voltages: BTreeSet<&str> = variants
        .iter()
        .filter_map(|v| {
            v.voltage
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })
        .collect();
    if voltages.len() > 1 {
        limitations.push(
            "Variantes com voltagens distintas no anúncio; confira a tensão antes de concluir."
                .into(),
        );
    }

    if limitations.is_empty() {
        limitations.push(
            "Condição observada pontualmente; frete e prazo variam conforme CEP no Mercado Livre."
                .into(),
        );
    }
    limitations.truncate(8);
    limitations
}

fn parse_installment_total_cents(summary: &str) -> Option<i64> {
    let lower = summary.to_ascii_lowercase();
    let x_pos = lower.find('×').or_else(|| lower.find('x'))?;
    let count_str: String = lower[..x_pos]
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    let count: i64 = count_str.parse().ok()?;
    let r_pos = summary.find("R$")?;
    let per_inst = parse_brl_amount_cents(&summary[r_pos..])?;
    count.checked_mul(per_inst)
}

fn build_unknowns(return_days: Option<u16>, specs: &[MercadoLivreSpec]) -> Vec<String> {
    let mut unknowns = vec![
        "Série contínua de 7 dias ainda em acumulação para comprovação histórica neste lote."
            .into(),
    ];
    if return_days.is_none() {
        unknowns.push("Prazo explícito de devolução não retornado no JSON-LD desta coleta.".into());
    }
    if specs.len() < 3 {
        unknowns.push(
            "Ficha técnica reduzida nesta captura; valide detalhes na página oficial.".into(),
        );
    }
    unknowns
}

fn build_tags(brand: &str, title: &str, category_slug: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut tags = Vec::new();

    for raw in std::iter::once("mercado-livre")
        .chain(std::iter::once(category_slug))
        .chain(std::iter::once(brand))
        .chain(title.split_whitespace().take(6))
    {
        let mut slug = String::new();
        let mut prev_dash = true;
        for ch in raw.chars() {
            let mapped = match ch {
                'a'..='z' | '0'..='9' => Some(ch),
                'A'..='Z' => Some(ch.to_ascii_lowercase()),
                'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => Some('a'),
                'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => Some('e'),
                'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => Some('i'),
                'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => Some('o'),
                'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => Some('u'),
                'ç' | 'Ç' => Some('c'),
                _ => None,
            };
            if let Some(c) = mapped {
                if slug.len() < 48 {
                    slug.push(c);
                    prev_dash = false;
                }
            } else if !prev_dash && !slug.is_empty() && slug.len() < 48 {
                slug.push('-');
                prev_dash = true;
            }
        }
        while slug.ends_with('-') {
            slug.pop();
        }
        if valid_slug(&slug) && slug.len() >= 2 && seen.insert(slug.clone()) {
            tags.push(slug);
            if tags.len() >= 10 {
                break;
            }
        }
    }
    tags
}

fn clean_text(raw: &str, max_bytes: usize) -> String {
    let sanitized = raw
        .replace('\0', " ")
        .replace("oc_sk_", "[redacted]")
        .replace("-----BEGIN ", "[redacted]")
        .replace("AGE-SECRET-", "[redacted]");
    let collapsed = sanitized.trim();
    let truncated = truncate_utf8(collapsed, max_bytes);
    if no_local_path_leak(&truncated) {
        truncated
    } else {
        // Strip potential local segments conservatively without ever passing a local path upward.
        truncate_utf8(
            &truncated
                .split_whitespace()
                .filter(|tok| no_local_path_leak(tok))
                .collect::<Vec<_>>()
                .join(" "),
            max_bytes,
        )
    }
}

fn clean_or_fallback(raw: &str, max_bytes: usize, fallback: &str) -> String {
    let cleaned = clean_text(raw, max_bytes);
    if cleaned.is_empty() {
        truncate_utf8(fallback.trim(), max_bytes)
    } else {
        cleaned
    }
}

fn truncate_utf8(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_owned();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].trim_end().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Bytes,
        extract::{Path as AxumPath, State as AxumState},
        http::{HeaderMap, StatusCode},
        routing::put,
    };
    use sha2::{Digest, Sha256};
    use std::{
        collections::HashMap,
        fs,
        os::unix::fs::PermissionsExt,
        sync::{Arc, Mutex},
    };
    use tokio::task::JoinHandle;
    use zaturo_core::{
        SiteCatalogSnapshot, TelegramDeliveryAck, TelegramDeliveryStatus, TelegramOfferQueueItem,
    };
    use zaturo_hub::{AppState, config::HubConfig};
    use zaturo_transfer::minio::MinioConfig;

    const OWNER: &str = "synthetic-owner-e2e-ml-token-value";
    const DEVICE: &str = "synthetic-device-e2e-ml-token-value";
    const READ_APP: &str = "synthetic-read-app-e2e-ml-token-value";

    fn valid_png_bytes(width: u32, height: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        out.extend_from_slice(&13u32.to_be_bytes());
        out.extend_from_slice(b"IHDR");
        out.extend_from_slice(&width.to_be_bytes());
        out.extend_from_slice(&height.to_be_bytes());
        out.extend_from_slice(&[8, 2, 0, 0, 0]);
        out.extend_from_slice(&[0u8; 4]);
        out
    }

    #[derive(Clone, Default)]
    struct MockMinioState {
        buckets: Arc<Mutex<BTreeSet<String>>>,
        objects: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    }

    async fn mock_head_bucket(
        AxumState(state): AxumState<MockMinioState>,
        AxumPath(bucket): AxumPath<String>,
        headers: HeaderMap,
    ) -> StatusCode {
        if headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .is_none()
        {
            return StatusCode::FORBIDDEN;
        }
        if state.buckets.lock().unwrap().contains(&bucket) {
            StatusCode::OK
        } else {
            StatusCode::NOT_FOUND
        }
    }

    async fn mock_put_bucket(
        AxumState(state): AxumState<MockMinioState>,
        AxumPath(bucket): AxumPath<String>,
        headers: HeaderMap,
    ) -> StatusCode {
        let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) else {
            return StatusCode::FORBIDDEN;
        };
        if !auth.starts_with("AWS4-HMAC-SHA256 ") {
            return StatusCode::FORBIDDEN;
        }
        state.buckets.lock().unwrap().insert(bucket);
        StatusCode::OK
    }

    async fn mock_put_object(
        AxumState(state): AxumState<MockMinioState>,
        AxumPath((bucket, key)): AxumPath<(String, String)>,
        headers: HeaderMap,
        body: Bytes,
    ) -> StatusCode {
        let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) else {
            return StatusCode::FORBIDDEN;
        };
        if !auth.starts_with("AWS4-HMAC-SHA256 ") || body.is_empty() {
            return StatusCode::BAD_REQUEST;
        }
        state
            .objects
            .lock()
            .unwrap()
            .insert(format!("{bucket}/{key}"), body.to_vec());
        StatusCode::OK
    }

    struct AbortOnDrop(JoinHandle<()>);
    impl Drop for AbortOnDrop {
        fn drop(&mut self) {
            self.0.abort();
        }
    }

    #[tokio::test]
    async fn end_to_end_ingests_four_real_ml_products_to_minio_and_hub_catalog_and_telegram() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Prepare mock local photos matching the 4 real ML hardware products, including a 9:16
        // vertical reel thumbnail next to a square product shot for SSD Kingston NV3.
        let photos_dir = root.join("photos");
        fs::create_dir_all(&photos_dir).unwrap();
        let nv3_vertical = photos_dir.join("nv3_reel.png");
        let nv3_square = photos_dir.join("nv3_box.png");
        let k500_square = photos_dir.join("k500.png");
        let lg_square = photos_dir.join("lg24.png");
        let fuxi_square = photos_dir.join("fuxi.png");

        fs::write(&nv3_vertical, valid_png_bytes(720, 1280)).unwrap();
        fs::write(&nv3_square, valid_png_bytes(1080, 1080)).unwrap();
        fs::write(&k500_square, valid_png_bytes(1000, 1000)).unwrap();
        fs::write(&lg_square, valid_png_bytes(1200, 900)).unwrap();
        fs::write(&fuxi_square, valid_png_bytes(960, 960)).unwrap();

        let prod_nv3 = json!({
            "product_id": "MLB39410080",
            "title": "SSD Kingston NV3 500GB M.2 2280 NVMe PCIe 4.0",
            "brand": "Kingston",
            "model": "SNV3S/500G",
            "product_url": "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/MLB39410080?matt_tool=99999999",
            "canonical_url": "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/MLB39410080",
            "image_url": "https://http2.mlstatic.com/D_NQ_NP_nv3.jpg",
            "commission": "5%",
            "sponsored": false,
            "card_text": "MAIS VENDIDO\n4.9 | +10mil vendidos\nR$\n1.379\n,\n00\nR$\n699\n,\n90\n49% OFF\n18x\nR$\n46\n,\n26\n",
            "share_link": "https://meli.la/1iWQZbJ",
            "share_id": "1PBCGF-SMN5",
            "share_text": "Código 1PBCGF-SMN5\nhttps://meli.la/1iWQZbJ",
            "price": 699.90,
            "meta_price": "699.90",
            "currency": "BRL",
            "availability": "https://schema.org/InStock",
            "price_valid_until": "2026-11-18",
            "return_days": 30,
            "seller": "Loja Oficial Kingston",
            "short_description": "SSD M.2 2280 PCIe 4.0 x4 com leitura sequencial de até 5.000 MB/s.",
            "long_description": "Unidade NV3 M.2 2280 para desktops e notebooks modernos.",
            "specs": [
                {"name": "Marca", "value": "Kingston"},
                {"name": "Capacidade", "value": "500 GB"},
                {"name": "Interface", "value": "PCIe 4.0 NVMe"}
            ],
            "variants": [
                {
                    "name": "SSD Kingston NV3 500GB Azul-aço",
                    "sku": "MLB7607021422",
                    "productID": "MLB39410080",
                    "color": "Azul-aço",
                    "offers": {
                        "price": 698.00,
                        "priceCurrency": "BRL",
                        "availability": "https://schema.org/InStock",
                        "url": "https://www.mercadolivre.com.br/ssd-kingston-nv3-500gb/p/MLB39410080",
                        "hasMerchantReturnPolicy": {"merchantReturnDays": 30, "returnFees": "https://schema.org/FreeReturn"}
                    }
                }
            ],
            "gallery": [
                "https://http2.mlstatic.com/D_NQ_NP_nv3_reel.jpg",
                "https://http2.mlstatic.com/D_NQ_NP_nv3.jpg"
            ],
            "photos": ["photos/nv3_reel.png", "photos/nv3_box.png"]
        });
        let prod_k500 = json!({
            "product_id": "MLB53222888",
            "title": "Teclado Mecânico Gamer Machenike K500-B61 Switch Red",
            "brand": "Machenike",
            "model": "K500-B61",
            "canonical_url": "https://www.mercadolivre.com.br/teclado-machenike-k500-b61/p/MLB53222888",
            "commission": "11%",
            "card_text": "OFERTA DO DIA\n4.8 | +5mil vendidos\nR$\n199\n,\n90\nR$\n138\n,\n79\n30% OFF no Pix",
            "share_link": "https://meli.la/2kX9LmP",
            "price": 149.18,
            "return_days": 30,
            "seller": "Machenike Brasil",
            "short_description": "Teclado compacto 60% hot-swappable com switch linear vermelho.",
            "long_description": "Construção robusta resistente a salpicos acidentais na mesa.",
            "specs": [
                {"name": "Marca", "value": "Machenike"},
                {"name": "Tipo de switch", "value": "Outemu Red Linear"},
                {"name": "É resistente à água", "value": "Não"}
            ],
            "photos": ["photos/k500.png"]
        });
        let prod_lg = json!({
            "product_id": "MLB48468943",
            "title": "Monitor Gamer LG UltraGear 24 Full HD IPS 180Hz 1ms 24GS60F",
            "brand": "LG",
            "model": "24GS60F-B",
            "canonical_url": "https://www.mercadolivre.com.br/monitor-lg-ultragear-24gs60f/p/MLB48468943",
            "commission": "8%",
            "card_text": "MAIS PROCURADO\n4.9 | +2mil vendidos\nR$\n1.099\n,\n00\nR$\n789\n,\n90\n",
            "share_link": "https://meli.la/1nQ4RsT",
            "price": 789.90,
            "return_days": 30,
            "seller": "LG Loja Oficial",
            "specs": [
                {"name": "Tamanho da tela", "value": "23.8 pol"},
                {"name": "Taxa de atualização", "value": "180 Hz"}
            ],
            "variants": [
                {
                    "name": "Monitor LG 24GS60F 110V/220V",
                    "sku": "SKU-LG-110",
                    "productID": "MLB48468943",
                    "offers": {"price": 789.90, "priceCurrency": "BRL"},
                    "additionalProperty": [{"name": "Voltagem", "value": "110V"}]
                },
                {
                    "name": "Monitor LG 24GS60F 220V",
                    "sku": "SKU-LG-220",
                    "productID": "MLB48468944",
                    "offers": {"price": 789.90, "priceCurrency": "BRL"},
                    "additionalProperty": [{"name": "Voltagem", "value": "220V"}]
                }
            ],
            "photos": ["photos/lg24.png"]
        });
        let prod_fuxi = json!({
            "product_id": "MLB79071532",
            "title": "Headset Gamer Sem Fio Havit Fuxi-H3 Quad-Mode Branco",
            "brand": "Havit",
            "model": "Fuxi-H3",
            "canonical_url": "https://www.mercadolivre.com.br/headset-havit-fuxi-h3/p/MLB79071532",
            "commission": "10%",
            "card_text": "4.7 | +1mil vendidos\nR$\n249\n,\n00\nR$\n179\n,\n90\n",
            "share_link": "https://meli.la/2mP8VxY",
            "price": 179.90,
            "return_days": 30,
            "seller": "Havit Oficial",
            "specs": [
                {"name": "Conectividade", "value": "2.4GHz / Bluetooth / USB-C / 3.5mm"},
                {"name": "Peso", "value": "186 g"}
            ],
            "photos": ["photos/fuxi.png"]
        });
        let bot_batch = json!({
            "network": "mercadolivre_afiliados",
            "query": "hardware perifericos pc",
            "captured_at": "2026-11-15T20:10:00-04:00",
            "count": 4,
            "products": [prod_nv3, prod_k500, prod_lg, prod_fuxi]
        });

        // Start mock MinIO server on loopback.
        let minio_state = MockMinioState::default();
        let minio_router = Router::new()
            .route("/{bucket}", put(mock_put_bucket).head(mock_head_bucket))
            .route("/{bucket}/{*key}", put(mock_put_object))
            .with_state(minio_state.clone());
        let minio_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let minio_addr = minio_listener.local_addr().unwrap();
        let _minio_server = AbortOnDrop(tokio::spawn(async move {
            axum::serve(minio_listener, minio_router).await.unwrap();
        }));

        let minio_secret_path = root.join("minio.secret");
        fs::write(&minio_secret_path, "synthetic-test-minio-secret-key\n").unwrap();
        fs::set_permissions(&minio_secret_path, fs::Permissions::from_mode(0o600)).unwrap();

        let minio_cfg = MinioConfig {
            endpoint: format!("http://{minio_addr}"),
            public_base_url: format!("http://{minio_addr}"),
            bucket: "procurali-media".into(),
            region: "us-east-1".into(),
            access_key_id: "procurali-worker".into(),
            secret_access_key_file: minio_secret_path,
            timeout_seconds: 5,
        };
        let minio_client = MinioClient::from_config(minio_cfg).unwrap();

        // Start real zaturo-hub listener on loopback.
        let hub_dir = root.join("hub");
        fs::create_dir_all(&hub_dir).unwrap();
        let recipient =
            zaturo_transfer::crypto::generate_identity(&hub_dir.join("worker-ml.identity"))
                .unwrap();
        let devices_toml = format!(
            "[[devices]]\nid=\"worker-ml\"\ntoken_sha256=\"{:x}\"\nencryption_recipient={recipient:?}\n",
            Sha256::digest(DEVICE)
        );
        fs::write(hub_dir.join("devices.toml"), devices_toml).unwrap();
        fs::set_permissions(
            hub_dir.join("devices.toml"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        let apps_toml = format!(
            "[[clients]]\nid=\"procurali-builder\"\ntoken_sha256=\"{:x}\"\ncan_submit=false\n",
            Sha256::digest(READ_APP)
        );
        fs::write(hub_dir.join("apps.toml"), apps_toml).unwrap();
        fs::set_permissions(hub_dir.join("apps.toml"), fs::Permissions::from_mode(0o600)).unwrap();

        let hub_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hub_addr = hub_listener.local_addr().unwrap();
        let hub_url = format!("http://{hub_addr}");
        let hub_state = AppState::new(
            HubConfig {
                bind: hub_addr,
                client_url: hub_url.clone(),
                database: hub_dir.join("hub.db"),
                worker_id: "worker-ml".into(),
                max_connections: 4,
                max_queued_tasks: 16,
                max_page_size: 100,
                lease_seconds: 30,
                max_workers: 4,
                devices_file: Some(hub_dir.join("devices.toml")),
                apps_file: Some(hub_dir.join("apps.toml")),
                transfer_root: Some(hub_dir.join("spool")),
                transfer_limits: Default::default(),
            },
            OWNER,
            DEVICE,
        )
        .unwrap();
        let _hub_server = AbortOnDrop(tokio::spawn(async move {
            axum::serve(hub_listener, zaturo_hub::router(hub_state))
                .await
                .unwrap();
        }));

        let parsed_schema1: MercadoLivreBotBatch = serde_json::from_value(json!({
            "schema": 1,
            "query": "monitor",
            "captured_at": "2026-11-15T18:30:32",
            "products": [prod_nv3.clone()]
        }))
        .unwrap();
        assert_eq!(parsed_schema1.schema, Some(1));
        assert!(parsed_schema1.network.is_empty());

        let raw_bytes = serde_json::to_vec(&bot_batch).unwrap();
        let req = prepare_and_upload_batch(
            &raw_bytes,
            &minio_client,
            &BatchIngestOptions {
                worker_id: "worker-ml".into(),
                default_category_slug: "hardware-e-perifericos".into(),
                channel_id: "@procurali".into(),
                now_ms: 1_763_252_000_000,
            },
            Some(root),
        )
        .await
        .unwrap();

        assert_eq!(req.products.len(), 4);
        // NV3 had 1 vertical video frame + 1 square photo -> only the 1 square photo was uploaded.
        assert_eq!(req.products[0].images.len(), 1);
        assert_eq!(minio_state.objects.lock().unwrap().len(), 4);
        // K500 flagged waterproofing contradiction and dual Pix/anúncio price.
        assert_eq!(req.products[1].pix_price_cents, 13_879);
        assert_eq!(req.products[1].price_brl_cents, 14_918);
        assert!(
            req.products[1]
                .limitations
                .iter()
                .any(|l| l.contains("resistência à água"))
        );
        // NV3 flagged interest-bearing 18x installments (18 * 46.26 = 832.68 > 699.90).
        assert!(
            req.products[0]
                .limitations
                .iter()
                .any(|l| l.contains("acréscimo/juros"))
        );
        // Monitor flagged multi-voltage variants.
        assert!(
            req.products[2]
                .limitations
                .iter()
                .any(|l| l.contains("voltagens distintas"))
        );

        let report = ingest_to_hub(&hub_url, DEVICE, &req, "e2e-ml-4-products")
            .await
            .unwrap();
        assert_eq!(report.product_count, 4);
        assert_eq!(report.image_count, 4);
        assert_eq!(report.telegram_queued_count, 4);

        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let catalog: SiteCatalogSnapshot = http
            .get(format!("{hub_url}/v1/commerce/mercadolivre/site-catalog"))
            .bearer_auth(READ_APP)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(catalog.offers.len(), 4);
        let catalog_json = serde_json::to_string(&catalog).unwrap();
        assert!(no_local_path_leak(&catalog_json));
        assert!(catalog_json.contains("https://meli.la/1iWQZbJ"));
        assert!(catalog.offers.iter().all(|o| {
            o.image_path
                .starts_with(&format!("http://{minio_addr}/procurali-media/"))
        }));

        let queue: Vec<TelegramOfferQueueItem> = http
            .get(format!(
                "{hub_url}/v1/commerce/mercadolivre/telegram-queue?channel_id=%40procurali&status=queued&limit=10"
            ))
            .bearer_auth(DEVICE)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(queue.len(), 4);

        let acked: TelegramOfferQueueItem = http
            .post(format!(
                "{hub_url}/v1/commerce/mercadolivre/telegram-queue/{}/ack",
                queue[0].delivery_id
            ))
            .bearer_auth(DEVICE)
            .json(&TelegramDeliveryAck {
                status: TelegramDeliveryStatus::Published,
                external_message_id: Some("tg-ml-1".into()),
                error_code: None,
            })
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(acked.status, TelegramDeliveryStatus::Published);
    }

    async fn spawn_watch_test_backends(root: &Path) -> (WorkerConfig, AbortOnDrop, AbortOnDrop) {
        let minio_state = MockMinioState::default();
        let minio_router = Router::new()
            .route("/{bucket}", put(mock_put_bucket).head(mock_head_bucket))
            .route("/{bucket}/{*key}", put(mock_put_object))
            .with_state(minio_state);
        let minio_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let minio_addr = minio_listener.local_addr().unwrap();
        let minio_srv = AbortOnDrop(tokio::spawn(async move {
            axum::serve(minio_listener, minio_router).await.unwrap();
        }));

        let minio_secret_path = root.join("minio.watch.secret");
        fs::write(&minio_secret_path, "synthetic-test-minio-secret-key\n").unwrap();
        fs::set_permissions(&minio_secret_path, fs::Permissions::from_mode(0o600)).unwrap();

        let minio_cfg = MinioConfig {
            endpoint: format!("http://{minio_addr}"),
            public_base_url: format!("http://{minio_addr}"),
            bucket: "procurali-media".into(),
            region: "us-east-1".into(),
            access_key_id: "procurali-worker".into(),
            secret_access_key_file: minio_secret_path,
            timeout_seconds: 5,
        };

        let hub_dir = root.join("hub-watch");
        fs::create_dir_all(&hub_dir).unwrap();
        let recipient =
            zaturo_transfer::crypto::generate_identity(&hub_dir.join("worker-ml.identity"))
                .unwrap();
        let devices_toml = format!(
            "[[devices]]\nid=\"worker-ml\"\ntoken_sha256=\"{:x}\"\nencryption_recipient={recipient:?}\n",
            Sha256::digest(DEVICE)
        );
        fs::write(hub_dir.join("devices.toml"), devices_toml).unwrap();
        fs::set_permissions(
            hub_dir.join("devices.toml"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();

        let hub_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hub_addr = hub_listener.local_addr().unwrap();
        let hub_url = format!("http://{hub_addr}");
        let hub_state = AppState::new(
            HubConfig {
                bind: hub_addr,
                client_url: hub_url.clone(),
                database: hub_dir.join("hub.db"),
                worker_id: "worker-ml".into(),
                max_connections: 4,
                max_queued_tasks: 16,
                max_page_size: 100,
                lease_seconds: 30,
                max_workers: 4,
                devices_file: Some(hub_dir.join("devices.toml")),
                apps_file: None,
                transfer_root: Some(hub_dir.join("spool")),
                transfer_limits: Default::default(),
            },
            OWNER,
            DEVICE,
        )
        .unwrap();
        let hub_srv = AbortOnDrop(tokio::spawn(async move {
            axum::serve(hub_listener, zaturo_hub::router(hub_state))
                .await
                .unwrap();
        }));

        let worker_cfg = WorkerConfig {
            hub_url,
            worker_id: "worker-ml".into(),
            database: root.join("worker.db"),
            workspace_root: root.join("ws"),
            artifact_root: root.join("art"),
            capabilities: vec!["diagnostic.v1".into()],
            max_concurrent_tasks: 1,
            heartbeat_seconds: 5,
            reconnect_max_seconds: 30,
            models_config: None,
            assistant_models_config: None,
            computer_models_config: None,
            computer: None,
            opencode: None,
            files: None,
            youtube: None,
            android: None,
            minio: Some(minio_cfg),
            telegram: None,
        };
        (worker_cfg, minio_srv, hub_srv)
    }

    #[tokio::test]
    async fn ingest_file_if_changed_skips_unchanged_and_reingests_on_change() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let photos_dir = root.join("photos");
        fs::create_dir_all(&photos_dir).unwrap();
        fs::write(photos_dir.join("item.png"), valid_png_bytes(800, 800)).unwrap();

        let (worker_cfg, _minio_srv, _hub_srv) = spawn_watch_test_backends(root).await;
        let input_path = root.join("products.json");
        let state_file = root.join("state/last.sha256");

        let make_batch = |price: f64| {
            json!({
                "network": "mercadolivre_afiliados",
                "query": "hardware watch test",
                "captured_at": "2026-11-16T12:00:00Z",
                "products": [{
                    "product_id": "MLB39410080",
                    "title": "SSD Kingston NV3 500GB NVMe",
                    "canonical_url": "https://www.mercadolivre.com.br/ssd-kingston/p/MLB39410080",
                    "share_link": "https://meli.la/1iWQZbJ",
                    "price": price,
                    "currency": "BRL",
                    "seller": "Loja Oficial Kingston",
                    "photos": ["photos/item.png"]
                }]
            })
        };

        fs::write(
            &input_path,
            serde_json::to_vec(&make_batch(499.90)).unwrap(),
        )
        .unwrap();
        let opts = BatchIngestOptions {
            worker_id: "worker-ml".into(),
            default_category_slug: "hardware-e-perifericos".into(),
            channel_id: "@procurali".into(),
            now_ms: 1_763_300_000_000,
        };

        let first = ingest_file_if_changed(
            &worker_cfg,
            DEVICE,
            &input_path,
            &state_file,
            &opts,
            Some(root),
        )
        .await
        .unwrap()
        .expect("initial batch must ingest");
        assert_eq!(first.product_count, 1);

        let second = ingest_file_if_changed(
            &worker_cfg,
            DEVICE,
            &input_path,
            &state_file,
            &opts,
            Some(root),
        )
        .await
        .unwrap();
        assert!(second.is_none(), "unchanged digest must skip ingest");

        fs::write(
            &input_path,
            serde_json::to_vec(&make_batch(449.90)).unwrap(),
        )
        .unwrap();
        let third = ingest_file_if_changed(
            &worker_cfg,
            DEVICE,
            &input_path,
            &state_file,
            &opts,
            Some(root),
        )
        .await
        .unwrap()
        .expect("modified batch must ingest again");
        assert_eq!(third.product_count, 1);
    }
}
