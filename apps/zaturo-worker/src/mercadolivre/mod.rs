//! Worker-side Mercado Livre batch ingestor: validates raw scraper JSON, filters mobile vertical
//! video frames in favor of product photography, uploads verified images to MinIO, strips local
//! filesystem paths at the worker boundary, and ingests idempotent batches into the Hub API.

mod editorial;
mod media;
mod normalize;
mod reference_sync;

#[cfg(test)]
mod tests;

pub use reference_sync::{
    ReferenceSyncSummary, load_and_query_reference_catalog, sync_reference_catalog,
};

use crate::config::WorkerConfig;
use anyhow::{Context, Result, bail, ensure};
use editorial::{clean_or_fallback, truncate_utf8};
use normalize::normalize_and_upload_product;
use std::{path::Path, time::Duration};
use zaturo_core::{
    MAX_ML_BATCH_PRODUCTS, MercadoLivreBotBatch, MercadoLivreIngestBatchRequest,
    MercadoLivreIngestReport, no_local_path_leak, parse_iso_to_ms, valid_slug,
    valid_telegram_channel,
};
use zaturo_transfer::minio::{MinioClient, sha256_hex};

#[derive(Debug, Clone)]
pub struct BatchIngestOptions {
    pub worker_id: String,
    pub default_category_slug: String,
    pub channel_id: String,
    pub now_ms: i64,
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
