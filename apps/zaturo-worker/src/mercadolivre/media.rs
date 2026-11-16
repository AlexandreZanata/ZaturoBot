use super::editorial::clean_or_fallback;
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use zaturo_core::{
    MAX_ML_IMAGES_PER_PRODUCT, MercadoLivreBotProduct, MinioProductImage, validate_asset_public_url,
};
use zaturo_transfer::minio::{InspectedImage, MinioClient, inspect_image_bytes, read_image_file};

struct PreparedImage {
    bytes: Vec<u8>,
    inspected: InspectedImage,
    origin_url: Option<String>,
}

pub(super) async fn upload_filtered_product_images(
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
