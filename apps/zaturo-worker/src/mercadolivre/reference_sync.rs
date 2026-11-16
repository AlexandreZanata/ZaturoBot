use super::editorial::truncate_utf8;
use anyhow::{Context, Result, bail};
use std::{path::Path, time::Duration};
use zaturo_core::{CommerceReferenceCatalog, CommerceReferenceItem};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ReferenceSyncSummary {
    pub status: String,
    pub etag: Option<String>,
    pub cache_generation: u64,
    pub total_products: usize,
    pub output_path: String,
}

fn write_atomic_private(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Fetches the compact VPS product/price-trend reference catalog using ETag conditional GET
/// (`If-None-Match` -> `304 Not Modified`) and writes `output_path` atomically for fast local lookup.
pub async fn sync_reference_catalog(
    hub_url: &str,
    worker_token: &str,
    output_path: &Path,
) -> Result<ReferenceSyncSummary> {
    zaturo_security::validate_endpoint(hub_url)?;
    zaturo_security::Credential::new(worker_token)?;

    let etag_path = output_path.with_extension("etag");
    let cached_etag = if output_path.exists() {
        std::fs::read_to_string(&etag_path)
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    } else {
        None
    };

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let url = format!(
        "{}/v1/device/commerce/mercadolivre/reference-catalog",
        hub_url.trim_end_matches('/')
    );
    let mut req = client.get(&url).bearer_auth(worker_token);
    if let Some(ref etag) = cached_etag {
        req = req.header(reqwest::header::IF_NONE_MATCH, etag);
    }

    let response = req
        .send()
        .await
        .context("failed connecting to Hub reference-catalog endpoint")?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_MODIFIED {
        let existing_bytes = std::fs::read(output_path)
            .with_context(|| format!("missing cached reference file {}", output_path.display()))?;
        let existing: CommerceReferenceCatalog = serde_json::from_slice(&existing_bytes)
            .context("cached local reference JSON is invalid")?;
        return Ok(ReferenceSyncSummary {
            status: "not_modified".into(),
            etag: cached_etag,
            cache_generation: existing.cache_generation,
            total_products: existing.total_products,
            output_path: output_path.display().to_string(),
        });
    }
    if !status.is_success() {
        let snippet = response.text().await.unwrap_or_default();
        bail!(
            "Hub rejected reference-catalog sync with HTTP {} ({})",
            status.as_u16(),
            truncate_utf8(&snippet, 200)
        );
    }

    let new_etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);
    let body = response
        .bytes()
        .await
        .context("failed reading Hub reference-catalog response body")?;
    let catalog: CommerceReferenceCatalog = serde_json::from_slice(&body)
        .context("invalid CommerceReferenceCatalog JSON returned by Hub")?;

    write_atomic_private(output_path, &body)?;
    if let Some(ref etag) = new_etag {
        write_atomic_private(&etag_path, format!("{etag}\n").as_bytes())?;
    }

    Ok(ReferenceSyncSummary {
        status: "updated".into(),
        etag: new_etag,
        cache_generation: catalog.cache_generation,
        total_products: catalog.total_products,
        output_path: output_path.display().to_string(),
    })
}

/// Queries the locally cached `vps-reference.json` file in milliseconds with zero network round-trips.
pub fn load_and_query_reference_catalog(
    reference_path: &Path,
    query: Option<&str>,
    product_id: Option<&str>,
    category: Option<&str>,
    limit: usize,
) -> Result<Vec<CommerceReferenceItem>> {
    let raw = std::fs::read(reference_path).with_context(|| {
        format!(
            "failed reading local reference file {}",
            reference_path.display()
        )
    })?;
    let catalog: CommerceReferenceCatalog = serde_json::from_slice(&raw).with_context(|| {
        format!(
            "invalid reference catalog JSON in {}",
            reference_path.display()
        )
    })?;

    let target_pid = product_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_uppercase);
    let target_cat = category
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase);
    let tokens: Vec<String> = query
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .filter(|t| !t.is_empty())
        .collect();

    let cap = limit.max(1);
    let mut matches = Vec::new();
    for item in catalog.items {
        if let Some(ref pid) = target_pid
            && item.product_id.to_ascii_uppercase() != *pid
        {
            continue;
        }
        if let Some(ref cat) = target_cat
            && item.category_slug.to_ascii_lowercase() != *cat
            && item.subcategory_slug.to_ascii_lowercase() != *cat
        {
            continue;
        }
        if !tokens.is_empty() {
            let haystack = format!(
                "{} {} {} {} {} {}",
                item.product_id,
                item.title,
                item.brand,
                item.category_slug,
                item.subcategory_slug,
                item.short_summary
            )
            .to_ascii_lowercase();
            if !tokens.iter().all(|tok| haystack.contains(tok)) {
                continue;
            }
        }
        matches.push(item);
        if matches.len() >= cap {
            break;
        }
    }
    Ok(matches)
}
