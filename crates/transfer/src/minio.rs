//! S3-compatible MinIO object storage uploader (AWS SigV4) and image header validator.
//! Ensures local filesystem paths remain on the worker while publishing clean hashed image URLs.

use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
use zaturo_core::{
    MAX_ML_IMAGE_BYTES, MinioProductImage, format_utc_sigv4, valid_minio_bucket,
    valid_minio_object_key, validate_ml_product_id,
};
use zeroize::Zeroizing;

fn default_timeout_seconds() -> u64 {
    15
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinioConfig {
    pub endpoint: String,
    pub public_base_url: String,
    pub bucket: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key_file: PathBuf,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
}

impl MinioConfig {
    pub fn validate(&self) -> Result<()> {
        zaturo_security::validate_endpoint(&self.endpoint)?;
        zaturo_security::validate_endpoint(&self.public_base_url)?;
        ensure!(
            valid_minio_bucket(&self.bucket),
            "MinIO bucket name must be 3..=63 lowercase alphanumeric or hyphen chars"
        );
        ensure!(
            !self.region.is_empty()
                && self.region.len() <= 32
                && self
                    .region
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
            "invalid MinIO region"
        );
        ensure!(
            (3..=128).contains(&self.access_key_id.len())
                && self.access_key_id.bytes().all(|b| b.is_ascii_graphic()),
            "invalid MinIO access_key_id"
        );
        ensure!(
            self.secret_access_key_file.is_absolute(),
            "MinIO secret_access_key_file must be an absolute path"
        );
        ensure!(
            (1..=60).contains(&self.timeout_seconds),
            "MinIO timeout_seconds must be 1..=60"
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    Webp,
}

impl ImageFormat {
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectedImage {
    pub format: ImageFormat,
    pub dimensions: Option<(u32, u32)>,
    pub sha256_hex: String,
    pub size_bytes: u64,
}

impl InspectedImage {
    /// Flags portrait mobile-video thumbnails (e.g. 9:16 vertical frames where h/w > 1.5)
    /// so square, landscape, and standard vertical retail-box photos (e.g. 851x1200) take precedence.
    pub fn is_vertical_video_frame(&self) -> bool {
        self.dimensions
            .is_some_and(|(w, h)| w > 0 && h.saturating_mul(2) > w.saturating_mul(3))
    }

    pub fn object_key(&self, product_id: &str, position: u16) -> Result<String> {
        ensure!(
            validate_ml_product_id(product_id),
            "invalid Mercado Livre product_id for object key"
        );
        let prefix: String = self.sha256_hex.chars().take(16).collect();
        let key = format!(
            "mercadolivre/{product_id}/{position:02}-{prefix}.{}",
            self.format.extension()
        );
        ensure!(
            valid_minio_object_key(&key),
            "constructed invalid MinIO object key"
        );
        Ok(key)
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex_lower(&digest)
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Validates image magic bytes, size ceiling, and header dimensions without invoking external
/// decoders on untrusted image streams beyond header parsing.
pub fn inspect_image_bytes(bytes: &[u8]) -> Result<InspectedImage> {
    let size_bytes = u64::try_from(bytes.len())?;
    ensure!(
        size_bytes > 0 && size_bytes <= MAX_ML_IMAGE_BYTES,
        "image size must be between 1 byte and {} bytes",
        MAX_ML_IMAGE_BYTES
    );

    let (format, dimensions) = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        ensure!(
            bytes.len() >= 24 && &bytes[12..16] == b"IHDR",
            "corrupt PNG header"
        );
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        ensure!(w > 0 && h > 0, "PNG dimensions must be positive");
        (ImageFormat::Png, Some((w, h)))
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        (ImageFormat::Jpeg, jpeg_dimensions(bytes))
    } else if bytes.len() >= 30 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        (ImageFormat::Webp, webp_dimensions(bytes))
    } else {
        anyhow::bail!("unsupported or invalid image format (expected JPEG, PNG, or WebP)");
    };

    Ok(InspectedImage {
        format,
        dimensions,
        sha256_hex: sha256_hex(bytes),
        size_bytes,
    })
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut idx = 2;
    while idx + 4 < bytes.len() {
        while idx < bytes.len() && bytes[idx] == 0xFF {
            idx += 1;
        }
        if idx >= bytes.len() {
            break;
        }
        let marker = bytes[idx];
        idx += 1;
        if matches!(marker, 0x00 | 0x01 | 0xD0..=0xD9) {
            if marker == 0xD9 {
                break;
            }
            continue;
        }
        if idx + 2 > bytes.len() {
            break;
        }
        let seg_len = usize::from(u16::from_be_bytes([bytes[idx], bytes[idx + 1]]));
        if seg_len < 2 || idx + seg_len > bytes.len() {
            break;
        }
        let is_sof = matches!(
            marker,
            0xC0 | 0xC1
                | 0xC2
                | 0xC3
                | 0xC5
                | 0xC6
                | 0xC7
                | 0xC9
                | 0xCA
                | 0xCB
                | 0xCD
                | 0xCE
                | 0xCF
        );
        if is_sof && seg_len >= 7 {
            let h = u32::from(u16::from_be_bytes([bytes[idx + 3], bytes[idx + 4]]));
            let w = u32::from(u16::from_be_bytes([bytes[idx + 5], bytes[idx + 6]]));
            if w > 0 && h > 0 {
                return Some((w, h));
            }
        }
        idx += seg_len;
    }
    None
}

fn webp_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 30 {
        return None;
    }
    match &bytes[12..16] {
        b"VP8X" if bytes.len() >= 30 => {
            let w = 1 + u32::from_le_bytes([bytes[24], bytes[25], bytes[26], 0]);
            let h = 1 + u32::from_le_bytes([bytes[27], bytes[28], bytes[29], 0]);
            Some((w, h))
        }
        b"VP8 " if bytes.len() >= 30 && bytes[23..26] == [0x9d, 0x01, 0x2a] => {
            let w = u32::from(u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3FFF);
            let h = u32::from(u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3FFF);
            (w > 0 && h > 0).then_some((w, h))
        }
        b"VP8L" if bytes.len() >= 25 && bytes[20] == 0x2f => {
            let bits = u32::from_le_bytes([bytes[21], bytes[22], bytes[23], bytes[24]]);
            let w = (bits & 0x3FFF) + 1;
            let h = ((bits >> 14) & 0x3FFF) + 1;
            Some((w, h))
        }
        _ => None,
    }
}

pub fn read_image_file(path: &Path) -> Result<Vec<u8>> {
    let mut file = crate::blob::read_file(path)
        .with_context(|| format!("could not open product image {}", path.display()))?;
    let meta = file.metadata()?;
    ensure!(
        meta.len() > 0 && meta.len() <= MAX_ML_IMAGE_BYTES,
        "product image exceeds {} byte limit",
        MAX_ML_IMAGE_BYTES
    );
    let mut buf = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    (&mut file)
        .take(MAX_ML_IMAGE_BYTES + 1)
        .read_to_end(&mut buf)?;
    ensure!(
        u64::try_from(buf.len())? <= MAX_ML_IMAGE_BYTES,
        "product image grew past size limit"
    );
    Ok(buf)
}

#[derive(Clone)]
pub struct MinioClient {
    http: reqwest::Client,
    config: MinioConfig,
    secret_access_key: Zeroizing<String>,
    host_header: String,
}

impl std::fmt::Debug for MinioClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MinioClient")
            .field("endpoint", &self.config.endpoint)
            .field("bucket", &self.config.bucket)
            .field("secret_access_key", &"[REDACTED]")
            .finish()
    }
}

impl MinioClient {
    pub fn from_config(config: MinioConfig) -> Result<Self> {
        config.validate()?;
        let mut file = crate::blob::read_file(&config.secret_access_key_file)
            .context("unable to read MinIO secret access key file")?;
        let mut raw = Zeroizing::new(String::new());
        (&mut file).take(4096).read_to_string(&mut raw)?;
        let secret = Zeroizing::new(raw.trim().to_owned());
        ensure!(
            !secret.is_empty() && !secret.contains('\n'),
            "MinIO secret access key file must contain a single non-empty key"
        );
        Self::new(config, secret.to_string())
    }

    pub fn new(config: MinioConfig, secret_access_key: impl Into<String>) -> Result<Self> {
        config.validate()?;
        let secret = Zeroizing::new(secret_access_key.into().trim().to_owned());
        ensure!(!secret.is_empty(), "MinIO secret_access_key is required");
        let parsed = reqwest::Url::parse(&config.endpoint)?;
        let host = parsed.host_str().context("MinIO endpoint missing host")?;
        let host_header = match parsed.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_owned(),
        };
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            http,
            config,
            secret_access_key: secret,
            host_header,
        })
    }

    pub fn public_url_for_key(&self, object_key: &str) -> Result<String> {
        ensure!(
            valid_minio_object_key(object_key),
            "invalid MinIO object key"
        );
        Ok(format!(
            "{}/{}/{}",
            self.config.public_base_url.trim_end_matches('/'),
            self.config.bucket,
            object_key
        ))
    }

    pub async fn ensure_bucket(&self, now_unix_secs: i64) -> Result<()> {
        let canonical_uri = format!("/{}", self.config.bucket);
        let payload_sha256 = sha256_hex(b"");
        let (amz_date, date_stamp) = format_utc_sigv4(now_unix_secs).map_err(anyhow::Error::msg)?;
        let head_auth = self.sign_request(
            "HEAD",
            &canonical_uri,
            &[
                ("host", &self.host_header),
                ("x-amz-content-sha256", &payload_sha256),
                ("x-amz-date", &amz_date),
            ],
            &payload_sha256,
            &amz_date,
            &date_stamp,
        );
        let head_url = format!(
            "{}{canonical_uri}",
            self.config.endpoint.trim_end_matches('/')
        );
        let resp = self
            .http
            .head(&head_url)
            .header("host", &self.host_header)
            .header("x-amz-content-sha256", &payload_sha256)
            .header("x-amz-date", &amz_date)
            .header("authorization", head_auth)
            .send()
            .await
            .context("MinIO bucket HEAD request failed")?;
        if resp.status().is_success() {
            return Ok(());
        }
        let put_auth = self.sign_request(
            "PUT",
            &canonical_uri,
            &[
                ("host", &self.host_header),
                ("x-amz-content-sha256", &payload_sha256),
                ("x-amz-date", &amz_date),
            ],
            &payload_sha256,
            &amz_date,
            &date_stamp,
        );
        let create = self
            .http
            .put(&head_url)
            .header("host", &self.host_header)
            .header("x-amz-content-sha256", &payload_sha256)
            .header("x-amz-date", &amz_date)
            .header("authorization", put_auth)
            .send()
            .await
            .context("MinIO bucket PUT creation failed")?;
        ensure!(
            create.status().is_success() || create.status().as_u16() == 409,
            "MinIO bucket creation returned HTTP {}",
            create.status()
        );
        Ok(())
    }

    pub async fn upload_product_image(
        &self,
        product_id: &str,
        position: u16,
        bytes: &[u8],
        alt_text: String,
        source_origin_url: Option<String>,
        now_unix_secs: i64,
    ) -> Result<MinioProductImage> {
        let inspected = inspect_image_bytes(bytes)?;
        let object_key = inspected.object_key(product_id, position)?;
        let content_type = inspected.format.content_type();
        let canonical_uri = format!("/{}/{}", self.config.bucket, object_key);
        let (amz_date, date_stamp) = format_utc_sigv4(now_unix_secs).map_err(anyhow::Error::msg)?;
        let auth = self.sign_request(
            "PUT",
            &canonical_uri,
            &[
                ("content-type", content_type),
                ("host", &self.host_header),
                ("x-amz-content-sha256", &inspected.sha256_hex),
                ("x-amz-date", &amz_date),
            ],
            &inspected.sha256_hex,
            &amz_date,
            &date_stamp,
        );
        let url = format!(
            "{}{canonical_uri}",
            self.config.endpoint.trim_end_matches('/')
        );
        let resp = self
            .http
            .put(&url)
            .header("content-type", content_type)
            .header("host", &self.host_header)
            .header("x-amz-content-sha256", &inspected.sha256_hex)
            .header("x-amz-date", &amz_date)
            .header("authorization", auth)
            .body(bytes.to_vec())
            .send()
            .await
            .context("MinIO PUT object request failed")?;
        ensure!(
            resp.status().is_success(),
            "MinIO PUT object {} failed with HTTP {}",
            object_key,
            resp.status()
        );

        let public_url = self.public_url_for_key(&object_key)?;
        let asset = MinioProductImage {
            position,
            bucket: self.config.bucket.clone(),
            object_key,
            public_url,
            source_origin_url,
            content_type: content_type.to_owned(),
            sha256_hex: inspected.sha256_hex,
            size_bytes: inspected.size_bytes,
            alt_text,
        };
        asset.validates().map_err(anyhow::Error::msg)?;
        Ok(asset)
    }

    fn sign_request(
        &self,
        method: &str,
        canonical_uri: &str,
        headers: &[(&str, &str)],
        payload_sha256: &str,
        amz_date: &str,
        date_stamp: &str,
    ) -> String {
        let mut canonical_headers = String::new();
        let mut signed_headers = Vec::with_capacity(headers.len());
        for (name, val) in headers {
            canonical_headers.push_str(name);
            canonical_headers.push(':');
            canonical_headers.push_str(val.trim());
            canonical_headers.push('\n');
            signed_headers.push(*name);
        }
        let signed_headers_str = signed_headers.join(";");
        let canonical_request = format!(
            "{method}\n{canonical_uri}\n\n{canonical_headers}\n{signed_headers_str}\n{payload_sha256}"
        );
        let scope = format!("{date_stamp}/{}/s3/aws4_request", self.config.region);
        let request_hash = sha256_hex(canonical_request.as_bytes());
        let string_to_sign = format!("AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{request_hash}");

        let k_secret = format!("AWS4{}", self.secret_access_key.as_str());
        let k_date = hmac_sha256(k_secret.as_bytes(), date_stamp.as_bytes());
        let k_region = hmac_sha256(&k_date, self.config.region.as_bytes());
        let k_service = hmac_sha256(&k_region, b"s3");
        let k_signing = hmac_sha256(&k_service, b"aws4_request");
        let signature = hex_lower(&hmac_sha256(&k_signing, string_to_sign.as_bytes()));

        format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers_str}, Signature={signature}",
            self.config.access_key_id
        )
    }
}

/// Pure safe RFC 2104 HMAC-SHA256 using `sha2::Sha256`.
fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_LEN: usize = 64;
    let mut normalized_key = [0u8; BLOCK_LEN];
    if key.len() > BLOCK_LEN {
        let hashed: [u8; 32] = Sha256::digest(key).into();
        normalized_key[..32].copy_from_slice(&hashed);
    } else {
        normalized_key[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK_LEN];
    let mut opad = [0x5cu8; BLOCK_LEN];
    for idx in 0..BLOCK_LEN {
        ipad[idx] ^= normalized_key[idx];
        opad[idx] ^= normalized_key[idx];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner_digest: [u8; 32] = inner.finalize().into();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    outer.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    fn minimal_png(width: u32, height: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        out.extend_from_slice(&13u32.to_be_bytes());
        out.extend_from_slice(b"IHDR");
        out.extend_from_slice(&width.to_be_bytes());
        out.extend_from_slice(&height.to_be_bytes());
        out.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        out
    }

    #[test]
    fn validates_images_and_detects_vertical_video_frames() {
        let square = inspect_image_bytes(&minimal_png(1200, 1200)).unwrap();
        assert_eq!(square.format, ImageFormat::Png);
        assert_eq!(square.dimensions, Some((1200, 1200)));
        assert!(!square.is_vertical_video_frame());
        assert_eq!(
            square.object_key("MLB39410080", 0).unwrap(),
            format!(
                "mercadolivre/MLB39410080/00-{}.png",
                &square.sha256_hex[..16]
            )
        );

        let reel = inspect_image_bytes(&minimal_png(720, 1280)).unwrap();
        assert!(reel.is_vertical_video_frame());

        let retail_box = inspect_image_bytes(&minimal_png(851, 1200)).unwrap();
        assert!(!retail_box.is_vertical_video_frame());

        assert!(inspect_image_bytes(b"<svg></svg>").is_err());
        assert!(inspect_image_bytes(b"").is_err());
    }

    #[test]
    fn hmac_sha256_matches_rfc_4231_vector_1() {
        let key = [0x0bu8; 20];
        let digest = hex_lower(&hmac_sha256(&key, b"Hi There"));
        assert_eq!(
            digest,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[tokio::test]
    async fn uploads_signed_put_object_to_minio_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut raw = vec![0u8; 4096];
            let n = socket.read(&mut raw).await.unwrap();
            let text = String::from_utf8_lossy(&raw[..n]).to_string();
            assert!(text.starts_with("PUT /procurali-media/mercadolivre/MLB39410080/00-"));
            assert!(text.contains("AWS4-HMAC-SHA256 Credential=testkey/"));
            assert!(text.to_ascii_lowercase().contains("x-amz-content-sha256:"));
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
        });

        let dir = tempfile::tempdir().unwrap();
        let secret_path = dir.path().join("minio-secret");
        std::fs::write(&secret_path, "synthetic-only-minio-secret-key").unwrap();
        let client = MinioClient::from_config(MinioConfig {
            endpoint: format!("http://{addr}"),
            public_base_url: "https://media.procurali.com.br".into(),
            bucket: "procurali-media".into(),
            region: "us-east-1".into(),
            access_key_id: "testkey".into(),
            secret_access_key_file: secret_path,
            timeout_seconds: 5,
        })
        .unwrap();

        let uploaded = client
            .upload_product_image(
                "MLB39410080",
                0,
                &minimal_png(800, 800),
                "SSD Kingston NV3 500GB".into(),
                Some("https://http2.mlstatic.com/sample.png".into()),
                1_763_231_280,
            )
            .await
            .unwrap();
        server.await.unwrap();

        assert!(uploaded.public_url.starts_with(
            "https://media.procurali.com.br/procurali-media/mercadolivre/MLB39410080/00-"
        ));
        assert_eq!(uploaded.content_type, "image/png");
    }
}
