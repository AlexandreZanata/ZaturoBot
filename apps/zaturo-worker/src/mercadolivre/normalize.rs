use super::editorial::{
    build_honest_limitations, build_long_description, build_payment_conditions,
    build_short_condition, build_short_description, build_supported_benefits, build_tags,
    build_unknowns, clean_or_fallback, clean_text, infer_for_whom, normalize_availability,
    truncate_utf8,
};
use super::media::upload_filtered_product_images;
use anyhow::{Context, Result, bail, ensure};
use serde_json::json;
use std::{collections::BTreeSet, path::Path};
use zaturo_core::{
    DEFAULT_ML_FRESHNESS_TTL_MINUTES, MAX_ML_SPECS_PER_PRODUCT, MAX_ML_VARIANTS_PER_PRODUCT,
    MercadoLivreBotProduct, MercadoLivreProductState, MercadoLivreSpec, MercadoLivreVariant,
    ValidatedMercadoLivreProduct, extract_product_id_from_url, no_local_path_leak,
    parse_brl_amount_cents, parse_card_text, parse_commission_bp, parse_jsonld_variant,
    parse_price_value_cents, slugify_pt_br, valid_slug, validate_meli_share_link,
    validate_ml_canonical_url, validate_ml_product_id, validate_ml_share_id,
};
use zaturo_transfer::minio::{MinioClient, sha256_hex};

pub(super) async fn normalize_and_upload_product(
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
        // Owner-authorized @procurali pipeline batches auto-enqueue (deduplicated / price drops
        // per ADR-0025); Candidate batches still require per-offer approval (ADR-0024).
        state: MercadoLivreProductState::Approved,
    };
    validated.validates().map_err(anyhow::Error::msg)?;
    Ok(validated)
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
