use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use zaturo_core::valid_telegram_channel;

pub(super) const PLACEHOLDER_BOT_TOKEN: &str = "COLE_SEU_TOKEN_DO_BOT_AQUI";
const MAX_TELEGRAM_SEND_INTERVAL_MS: u64 = 600_000;
const DEFAULT_LOCAL_ENV_FILE: &str = ".secrets/telegram.env";
const DEFAULT_ROOT_ENV_FILE: &str = ".env";
const TELEGRAM_ENV_KEYS: &[&str] = &[
    "ZATURO_TELEGRAM_BOT_TOKEN",
    "TELEGRAM_BOT_TOKEN",
    "ZATURO_TELEGRAM_CHANNEL_ID",
    "TELEGRAM_CHANNEL_ID",
    "ZATURO_TELEGRAM_ALLOW_LIVE_PUBLISH",
    "TELEGRAM_ALLOW_LIVE_PUBLISH",
    "ZATURO_TELEGRAM_AUTO_PUBLISH_ON_INGEST",
    "TELEGRAM_AUTO_PUBLISH_ON_INGEST",
    "ZATURO_TELEGRAM_MIN_INTERVAL_SECS",
    "TELEGRAM_MIN_INTERVAL_SECS",
    "ZATURO_TELEGRAM_MAX_INTERVAL_SECS",
    "TELEGRAM_MAX_INTERVAL_SECS",
    "ZATURO_TELEGRAM_LOCAL_MINIO_ENDPOINT",
    "TELEGRAM_LOCAL_MINIO_ENDPOINT",
    "ZATURO_TELEGRAM_MINIO_BUCKET",
    "TELEGRAM_MINIO_BUCKET",
    "ZATURO_TELEGRAM_SITE_BASE_URL",
    "TELEGRAM_SITE_BASE_URL",
];

fn default_telegram_api_base_url() -> String {
    "https://api.telegram.org".to_owned()
}

fn default_channel_id() -> String {
    "@procurali".to_owned()
}

fn default_send_interval_ms() -> u64 {
    1100
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelegramBotConfig {
    #[serde(default = "default_telegram_api_base_url")]
    pub api_base_url: String,
    #[serde(default)]
    pub bot_token_file: PathBuf,
    #[serde(default = "default_channel_id")]
    pub default_channel_id: String,
    #[serde(default)]
    pub site_base_url: Option<String>,
    #[serde(default = "default_send_interval_ms")]
    pub min_send_interval_ms: u64,
    #[serde(default)]
    pub max_send_interval_ms: Option<u64>,
    #[serde(default)]
    pub allow_live_publish: bool,
    #[serde(default)]
    pub auto_publish_on_ingest: bool,
    #[serde(default)]
    pub local_minio_endpoint: Option<String>,
    #[serde(default)]
    pub minio_bucket: Option<String>,
    #[serde(default)]
    pub env_file: Option<PathBuf>,
}

impl TelegramBotConfig {
    pub fn validate(&self) -> Result<()> {
        zaturo_security::validate_endpoint(&self.api_base_url)?;
        let has_env_token = has_configured_token_env(self.env_file.as_deref());
        ensure!(
            !self.bot_token_file.as_os_str().is_empty() || self.env_file.is_some() || has_env_token,
            "telegram bot_token_file or env_file/TELEGRAM_BOT_TOKEN must be set"
        );
        ensure!(
            valid_telegram_channel(&self.default_channel_id),
            "invalid telegram default_channel_id"
        );
        ensure!(
            self.min_send_interval_ms <= MAX_TELEGRAM_SEND_INTERVAL_MS,
            "telegram min_send_interval_ms exceeds maximum 600000 ms"
        );
        if let Some(max_ms) = self.max_send_interval_ms {
            ensure!(
                max_ms >= self.min_send_interval_ms && max_ms <= MAX_TELEGRAM_SEND_INTERVAL_MS,
                "telegram max_send_interval_ms must be between min_send_interval_ms and 600000 ms"
            );
        }
        if let Some(site_url) = &self.site_base_url {
            zaturo_security::validate_endpoint(site_url)?;
        }
        if let Some(minio_url) = &self.local_minio_endpoint {
            zaturo_security::validate_endpoint(minio_url)?;
        }
        if let Some(bucket) = &self.minio_bucket {
            ensure!(
                !bucket.trim().is_empty()
                    && bucket
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "invalid telegram minio_bucket"
            );
        }
        Ok(())
    }

    /// Resolves overrides from configured `env_file` (or `.secrets/telegram.env` when present
    /// and `bot_token_file` is absent) plus process environment variables.
    pub fn with_env_overrides(&self) -> Result<Self> {
        let fallback_to_default = !cfg!(test)
            && (self.bot_token_file.as_os_str().is_empty() || !self.bot_token_file.exists());
        let env_map = load_telegram_env_map(self.env_file.as_deref(), fallback_to_default)?;
        let mut resolved = self.clone();

        if let Some(channel) = pick_env(
            &env_map,
            &["ZATURO_TELEGRAM_CHANNEL_ID", "TELEGRAM_CHANNEL_ID"],
        ) {
            resolved.default_channel_id = channel;
        }
        if let Some(allow) = pick_env_bool(
            &env_map,
            &[
                "ZATURO_TELEGRAM_ALLOW_LIVE_PUBLISH",
                "TELEGRAM_ALLOW_LIVE_PUBLISH",
            ],
        )? {
            resolved.allow_live_publish = allow;
        }
        if let Some(auto_pub) = pick_env_bool(
            &env_map,
            &[
                "ZATURO_TELEGRAM_AUTO_PUBLISH_ON_INGEST",
                "TELEGRAM_AUTO_PUBLISH_ON_INGEST",
            ],
        )? {
            resolved.auto_publish_on_ingest = auto_pub;
        }
        if let Some(min_secs) = pick_env_u64(
            &env_map,
            &[
                "ZATURO_TELEGRAM_MIN_INTERVAL_SECS",
                "TELEGRAM_MIN_INTERVAL_SECS",
            ],
        )? {
            resolved.min_send_interval_ms = min_secs.saturating_mul(1000);
        }
        if let Some(max_secs) = pick_env_u64(
            &env_map,
            &[
                "ZATURO_TELEGRAM_MAX_INTERVAL_SECS",
                "TELEGRAM_MAX_INTERVAL_SECS",
            ],
        )? {
            resolved.max_send_interval_ms = Some(max_secs.saturating_mul(1000));
        } else if let Some(max_ms) = resolved.max_send_interval_ms
            && max_ms < resolved.min_send_interval_ms
        {
            resolved.max_send_interval_ms = Some(resolved.min_send_interval_ms);
        }
        if let Some(endpoint) = pick_env(
            &env_map,
            &[
                "ZATURO_TELEGRAM_LOCAL_MINIO_ENDPOINT",
                "TELEGRAM_LOCAL_MINIO_ENDPOINT",
            ],
        ) {
            resolved.local_minio_endpoint = Some(endpoint);
        }
        if let Some(bucket) = pick_env(
            &env_map,
            &["ZATURO_TELEGRAM_MINIO_BUCKET", "TELEGRAM_MINIO_BUCKET"],
        ) {
            resolved.minio_bucket = Some(bucket);
        }
        if let Some(site_url) = pick_env(
            &env_map,
            &["ZATURO_TELEGRAM_SITE_BASE_URL", "TELEGRAM_SITE_BASE_URL"],
        ) {
            resolved.site_base_url = Some(site_url);
        }
        resolved.validate()?;
        Ok(resolved)
    }
}

pub(super) fn has_configured_token_env(env_file: Option<&Path>) -> bool {
    if ["ZATURO_TELEGRAM_BOT_TOKEN", "TELEGRAM_BOT_TOKEN"]
        .iter()
        .any(|k| {
            std::env::var(k)
                .ok()
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        })
    {
        return true;
    }
    env_file.is_some()
        || (!cfg!(test)
            && (Path::new(DEFAULT_LOCAL_ENV_FILE).exists()
                || Path::new(DEFAULT_ROOT_ENV_FILE).exists()))
}

pub fn load_telegram_env_map(
    env_file: Option<&Path>,
    fallback_to_default_secrets: bool,
) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    let candidate_files: Vec<PathBuf> = match env_file {
        Some(path) => vec![path.to_path_buf()],
        None if fallback_to_default_secrets => [DEFAULT_LOCAL_ENV_FILE, DEFAULT_ROOT_ENV_FILE]
            .into_iter()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .collect(),
        None => Vec::new(),
    };
    for path in candidate_files {
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(&path)
            .with_context(|| format!("failed reading Telegram env file {}", path.display()))?;
        for raw_line in content.lines() {
            let trimmed = raw_line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let Some((raw_key, raw_val)) = trimmed.split_once('=') else {
                continue;
            };
            let key = raw_key
                .trim()
                .strip_prefix("export ")
                .unwrap_or(raw_key)
                .trim();
            if !TELEGRAM_ENV_KEYS.contains(&key) {
                continue;
            }
            let mut val = raw_val.trim();
            if ((val.starts_with('"') && val.ends_with('"'))
                || (val.starts_with('\'') && val.ends_with('\'')))
                && val.len() >= 2
            {
                val = &val[1..val.len() - 1];
            }
            if !val.trim().is_empty() {
                map.insert(key.to_owned(), val.trim().to_owned());
            }
        }
    }

    for key in TELEGRAM_ENV_KEYS {
        if let Ok(val) = std::env::var(key)
            && !val.trim().is_empty()
        {
            map.insert((*key).to_owned(), val.trim().to_owned());
        }
    }
    Ok(map)
}

pub(super) fn pick_env(map: &BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| map.get(*k))
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

fn pick_env_bool(map: &BTreeMap<String, String>, keys: &[&str]) -> Result<Option<bool>> {
    let Some(raw) = pick_env(map, keys) else {
        return Ok(None);
    };
    match raw.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "sim" => Ok(Some(true)),
        "false" | "0" | "no" | "nao" => Ok(Some(false)),
        _ => bail!("invalid boolean value for Telegram env setting: {raw}"),
    }
}

fn pick_env_u64(map: &BTreeMap<String, String>, keys: &[&str]) -> Result<Option<u64>> {
    let Some(raw) = pick_env(map, keys) else {
        return Ok(None);
    };
    let parsed: u64 = raw
        .parse()
        .with_context(|| format!("invalid integer seconds value for Telegram env: {raw}"))?;
    Ok(Some(parsed))
}
