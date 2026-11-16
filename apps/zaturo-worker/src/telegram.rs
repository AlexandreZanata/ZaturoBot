//! Official Telegram Bot API client, queue publisher, interactive command poller,
//! and WhatsApp Web human-assisted draft builder (without Meta API or unofficial browser bots).

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fmt, fs,
    path::{Path, PathBuf},
    time::Duration,
};
use zaturo_core::{
    TelegramBotCommand, TelegramDeliveryAck, TelegramDeliveryStatus, TelegramInlineKeyboardMarkup,
    TelegramOfferQueueItem, WhatsappAssistedPackage, build_telegram_offer_keyboard,
    build_whatsapp_assisted_package, format_telegram_bot_command_reply_pt_br,
    format_telegram_html_caption_pt_br, valid_minio_object_key, valid_telegram_channel,
};
use zaturo_transfer::minio::{ImageFormat, inspect_image_bytes};

const PLACEHOLDER_BOT_TOKEN: &str = "COLE_SEU_TOKEN_DO_BOT_AQUI";
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

fn has_configured_token_env(env_file: Option<&Path>) -> bool {
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

fn pick_env(map: &BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramPublishItemOutcome {
    pub delivery_id: String,
    pub product_id: String,
    pub channel_id: String,
    pub method: String,
    pub status: TelegramDeliveryStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    pub caption_html_preview: String,
    pub reply_markup: TelegramInlineKeyboardMarkup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramPublishReport {
    pub dry_run: bool,
    pub channel_id: String,
    pub fetched_count: usize,
    pub published_count: usize,
    pub failed_count: usize,
    pub items: Vec<TelegramPublishItemOutcome>,
    pub whatsapp_package: WhatsappAssistedPackage,
    /// Set when the run stopped because the Hub could not record a claim or result. A
    /// delivery whose result was not recorded stays `sending` and is never sent again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hub_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramPollReport {
    pub updates_processed: usize,
    pub commands_answered: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramDaemonReport {
    pub cycles_completed: usize,
    pub published_count: usize,
    pub failed_count: usize,
    pub commands_answered: usize,
}

#[derive(Clone)]
pub struct TelegramBotClient {
    config: TelegramBotConfig,
    bot_token: String,
    http: reqwest::Client,
}

impl fmt::Debug for TelegramBotClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TelegramBotClient")
            .field("api_base_url", &self.config.api_base_url)
            .field("default_channel_id", &self.config.default_channel_id)
            .field("site_base_url", &self.config.site_base_url)
            .field("min_send_interval_ms", &self.config.min_send_interval_ms)
            .field("max_send_interval_ms", &self.config.max_send_interval_ms)
            .field("allow_live_publish", &self.config.allow_live_publish)
            .field(
                "auto_publish_on_ingest",
                &self.config.auto_publish_on_ingest,
            )
            .field("bot_token", &"<redacted>")
            .finish()
    }
}

#[derive(Debug, Deserialize)]
struct TelegramApiErrorBody {
    #[serde(default)]
    parameters: Option<TelegramRetryParameters>,
}

#[derive(Debug, Deserialize)]
struct TelegramRetryParameters {
    #[serde(default)]
    retry_after: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TelegramSendSuccess {
    ok: bool,
    #[serde(default)]
    result: Option<TelegramSentMessage>,
}

#[derive(Debug, Deserialize)]
struct TelegramSentMessage {
    message_id: i64,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdatesResponse {
    ok: bool,
    #[serde(default)]
    result: Vec<TelegramUpdate>,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    #[serde(default)]
    message: Option<TelegramIncomingMessage>,
}

#[derive(Debug, Deserialize)]
struct TelegramIncomingMessage {
    message_id: i64,
    chat: TelegramChat,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
}

enum SendAttemptOutcome {
    Delivered {
        method: &'static str,
        external_message_id: String,
    },
    DeterministicFailure {
        status: u16,
        error_code: String,
        retry_after_secs: Option<u64>,
    },
    UncertainTransportFailure,
}

impl TelegramBotClient {
    pub fn from_config(config: TelegramBotConfig) -> Result<Self> {
        let resolved_config = config.with_env_overrides()?;
        let fallback_to_default = !cfg!(test)
            && (resolved_config.bot_token_file.as_os_str().is_empty()
                || !resolved_config.bot_token_file.exists());
        let env_map =
            load_telegram_env_map(resolved_config.env_file.as_deref(), fallback_to_default)?;
        let env_token = pick_env(
            &env_map,
            &["ZATURO_TELEGRAM_BOT_TOKEN", "TELEGRAM_BOT_TOKEN"],
        );
        let bot_token = match env_token {
            Some(tok) if tok == PLACEHOLDER_BOT_TOKEN => {
                if !resolved_config.bot_token_file.as_os_str().is_empty()
                    && resolved_config.bot_token_file.exists()
                {
                    fs::read_to_string(&resolved_config.bot_token_file)
                        .with_context(|| {
                            format!(
                                "failed reading Telegram bot token file {}",
                                resolved_config.bot_token_file.display()
                            )
                        })?
                        .trim()
                        .to_owned()
                } else {
                    bail!(
                        "TELEGRAM_BOT_TOKEN still contains placeholder '{PLACEHOLDER_BOT_TOKEN}' — paste your BotFather token in the .env file"
                    );
                }
            }
            Some(tok) => tok,
            None => fs::read_to_string(&resolved_config.bot_token_file)
                .with_context(|| {
                    format!(
                        "failed reading Telegram bot token file {}",
                        resolved_config.bot_token_file.display()
                    )
                })?
                .trim()
                .to_owned(),
        };
        ensure!(
            bot_token != PLACEHOLDER_BOT_TOKEN
                && !bot_token.is_empty()
                && bot_token.bytes().all(|b| b.is_ascii_graphic()),
            "Telegram bot token is empty, placeholder, or contains invalid whitespace"
        );
        zaturo_security::Credential::new(&bot_token)?;
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            config: resolved_config,
            bot_token,
            http,
        })
    }

    pub fn config(&self) -> &TelegramBotConfig {
        &self.config
    }

    pub fn effective_send_interval_range_ms(&self) -> (u64, u64) {
        let min_ms = self.config.min_send_interval_ms;
        let max_ms = self
            .config
            .max_send_interval_ms
            .unwrap_or(min_ms)
            .max(min_ms);
        (min_ms, max_ms)
    }

    /// Computes a natural inter-post delay bounded by `[min_send_interval_ms, max_send_interval_ms]`.
    pub fn compute_inter_post_delay(&self, item_index: usize, delivery_id: &str) -> Duration {
        let (min_ms, max_ms) = self.effective_send_interval_range_ms();
        if max_ms == 0 {
            return Duration::ZERO;
        }
        if max_ms <= min_ms {
            return Duration::from_millis(min_ms);
        }
        let span = max_ms - min_ms;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| u64::from(d.subsec_nanos()) ^ d.as_secs())
            .unwrap_or(0);
        let mut acc = nanos ^ (item_index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        for byte in delivery_id.bytes() {
            acc = acc
                .wrapping_mul(1_099_511_628_211)
                .wrapping_add(u64::from(byte));
        }
        Duration::from_millis(min_ms + (acc % (span + 1)))
    }

    pub fn build_whatsapp_package(
        &self,
        items: &[TelegramOfferQueueItem],
        channel_id: &str,
        now_ms: i64,
    ) -> WhatsappAssistedPackage {
        build_whatsapp_assisted_package(
            items,
            channel_id,
            self.config.site_base_url.as_deref(),
            now_ms,
        )
    }

    pub async fn fetch_hub_queue(
        &self,
        hub_url: &str,
        hub_token: &str,
        channel_id: Option<&str>,
        status: Option<TelegramDeliveryStatus>,
        limit: u32,
    ) -> Result<Vec<TelegramOfferQueueItem>> {
        fetch_hub_queue_with_client(&self.http, hub_url, hub_token, channel_id, status, limit).await
    }

    pub async fn ack_hub_delivery(
        &self,
        hub_url: &str,
        hub_token: &str,
        delivery_id: &str,
        ack: &TelegramDeliveryAck,
    ) -> Result<TelegramOfferQueueItem> {
        ack_hub_delivery_with_client(&self.http, hub_url, hub_token, delivery_id, ack).await
    }

    pub async fn publish_queued_from_hub(
        &self,
        hub_url: &str,
        hub_token: &str,
        channel_id: Option<&str>,
        limit: u32,
        dry_run: bool,
        now_ms: i64,
    ) -> Result<TelegramPublishReport> {
        let target_channel = channel_id
            .unwrap_or(&self.config.default_channel_id)
            .to_owned();
        ensure!(
            valid_telegram_channel(&target_channel),
            "invalid Telegram channel_id"
        );

        let mut queued = self
            .fetch_hub_queue(
                hub_url,
                hub_token,
                Some(&target_channel),
                Some(TelegramDeliveryStatus::Queued),
                limit,
            )
            .await?;
        if queued.is_empty() && channel_id.is_none() {
            // If using a custom .env target channel (e.g. a group ID), also claim queued offers
            // inserted under the default catalog channel.
            queued = self
                .fetch_hub_queue(
                    hub_url,
                    hub_token,
                    None,
                    Some(TelegramDeliveryStatus::Queued),
                    limit,
                )
                .await?;
        }

        let whatsapp_package = build_whatsapp_assisted_package(
            &queued,
            &target_channel,
            self.config.site_base_url.as_deref(),
            now_ms,
        );

        if dry_run {
            let items = queued
                .iter()
                .map(|item| {
                    let caption = format_telegram_html_caption_pt_br(
                        item,
                        self.config.site_base_url.as_deref(),
                    );
                    let keyboard =
                        build_telegram_offer_keyboard(item, self.config.site_base_url.as_deref());
                    let method = if item.image_public_url.is_some() {
                        "sendPhoto"
                    } else {
                        "sendMessage"
                    };
                    TelegramPublishItemOutcome {
                        delivery_id: item.delivery_id.clone(),
                        product_id: item.product_id.clone(),
                        channel_id: target_channel.clone(),
                        method: method.to_owned(),
                        status: TelegramDeliveryStatus::Queued,
                        external_message_id: None,
                        error_code: None,
                        caption_html_preview: caption,
                        reply_markup: keyboard,
                    }
                })
                .collect();
            return Ok(TelegramPublishReport {
                dry_run: true,
                channel_id: target_channel,
                fetched_count: queued.len(),
                published_count: 0,
                failed_count: 0,
                items,
                whatsapp_package,
                hub_error: None,
            });
        }

        ensure!(
            self.config.allow_live_publish,
            "telegram live publishing requires [telegram] allow_live_publish = true (or pass --dry-run)"
        );

        let mut outcomes = Vec::with_capacity(queued.len());
        let mut published_count = 0usize;
        let mut failed_count = 0usize;
        let mut hub_error = None;
        let method_for = |item: &TelegramOfferQueueItem| {
            if item.image_public_url.is_some() {
                "sendPhoto".to_owned()
            } else {
                "sendMessage".to_owned()
            }
        };

        for (idx, item) in queued.iter().enumerate() {
            if idx > 0 {
                let delay = self.compute_inter_post_delay(idx, &item.delivery_id);
                if !delay.is_zero() {
                    tracing::info!(
                        delivery_id = %item.delivery_id,
                        delay_secs = delay.as_secs(),
                        "waiting natural interval before posting next Telegram offer"
                    );
                    tokio::time::sleep(delay).await;
                }
            }
            let caption =
                format_telegram_html_caption_pt_br(item, self.config.site_base_url.as_deref());
            let keyboard =
                build_telegram_offer_keyboard(item, self.config.site_base_url.as_deref());

            // Claim the delivery before the external send. Without a recorded claim nothing
            // is sent; a claim that the Hub resolved differently (stale, superseded, or taken
            // by another publisher) is skipped.
            let claim = self
                .ack_hub_delivery(
                    hub_url,
                    hub_token,
                    &item.delivery_id,
                    &TelegramDeliveryAck {
                        status: TelegramDeliveryStatus::Sending,
                        external_message_id: None,
                        error_code: None,
                    },
                )
                .await;
            let claimed_status = match claim {
                Ok(claimed) => claimed.status,
                Err(_) => {
                    hub_error = Some("hub_claim_failed".to_owned());
                    break;
                }
            };
            if claimed_status != TelegramDeliveryStatus::Sending {
                outcomes.push(TelegramPublishItemOutcome {
                    delivery_id: item.delivery_id.clone(),
                    product_id: item.product_id.clone(),
                    channel_id: target_channel.clone(),
                    method: method_for(item),
                    status: claimed_status,
                    external_message_id: None,
                    error_code: Some("not_claimable".to_owned()),
                    caption_html_preview: caption,
                    reply_markup: keyboard,
                });
                continue;
            }

            let outcome = self
                .deliver_single_item(&target_channel, item, &caption, &keyboard)
                .await;
            let (ack_status, ext_id, err_code, method_used) = match outcome {
                SendAttemptOutcome::Delivered {
                    method,
                    external_message_id,
                } => {
                    published_count += 1;
                    (
                        TelegramDeliveryStatus::Published,
                        Some(external_message_id),
                        None,
                        method.to_owned(),
                    )
                }
                SendAttemptOutcome::DeterministicFailure { error_code, .. } => {
                    failed_count += 1;
                    (
                        TelegramDeliveryStatus::Failed,
                        None,
                        Some(error_code),
                        method_for(item),
                    )
                }
                SendAttemptOutcome::UncertainTransportFailure => {
                    // Never retry an uncertain side effect; mark failed for operator review.
                    failed_count += 1;
                    (
                        TelegramDeliveryStatus::Failed,
                        None,
                        Some("telegram_uncertain_network".to_owned()),
                        method_for(item),
                    )
                }
            };

            let recorded = self
                .ack_hub_delivery(
                    hub_url,
                    hub_token,
                    &item.delivery_id,
                    &TelegramDeliveryAck {
                        status: ack_status,
                        external_message_id: ext_id.clone(),
                        error_code: err_code.clone(),
                    },
                )
                .await;

            outcomes.push(TelegramPublishItemOutcome {
                delivery_id: item.delivery_id.clone(),
                product_id: item.product_id.clone(),
                channel_id: target_channel.clone(),
                method: method_used,
                status: ack_status,
                external_message_id: ext_id,
                error_code: err_code,
                caption_html_preview: caption,
                reply_markup: keyboard,
            });
            if recorded.is_err() {
                // The send already happened; the delivery stays `sending` in the Hub and is
                // never sent again. Stop instead of sending more while results cannot be kept.
                hub_error = Some("hub_ack_failed_delivery_left_sending".to_owned());
                break;
            }
        }

        Ok(TelegramPublishReport {
            dry_run: false,
            channel_id: target_channel,
            fetched_count: queued.len(),
            published_count,
            failed_count,
            items: outcomes,
            whatsapp_package,
            hub_error,
        })
    }

    async fn fetch_local_minio_image(
        &self,
        item: &TelegramOfferQueueItem,
    ) -> Option<(Vec<u8>, &'static str, &'static str)> {
        let endpoint = self.config.local_minio_endpoint.as_deref()?;
        let object_key = item.image_object_key.as_deref()?;
        if !valid_minio_object_key(object_key) {
            return None;
        }
        let bucket = self
            .config
            .minio_bucket
            .as_deref()
            .or_else(|| {
                item.image_public_url
                    .as_deref()
                    .and_then(|url| extract_bucket_from_public_url(url, object_key))
            })
            .unwrap_or("procurali-media");
        let url = format!(
            "{}/{}/{}",
            endpoint.trim_end_matches('/'),
            bucket.trim_matches('/'),
            object_key.trim_start_matches('/')
        );
        let resp = self.http.get(url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let bytes = resp.bytes().await.ok()?;
        if bytes.is_empty() || bytes.len() > 10 * 1024 * 1024 {
            return None;
        }
        let inspected = inspect_image_bytes(&bytes).ok()?;
        let (filename, mime) = match inspected.format {
            ImageFormat::Jpeg => ("offer.jpg", "image/jpeg"),
            ImageFormat::Png => ("offer.png", "image/png"),
            ImageFormat::Webp => return None,
        };
        Some((bytes.to_vec(), filename, mime))
    }

    async fn attempt_photo_or_message(
        &self,
        target_channel: &str,
        item: &TelegramOfferQueueItem,
        caption_html: &str,
        keyboard: &TelegramInlineKeyboardMarkup,
        cached_photo: Option<&(Vec<u8>, &'static str, &'static str)>,
    ) -> SendAttemptOutcome {
        if let Some((bytes, filename, mime)) = cached_photo {
            let multipart_outcome = self
                .call_send_photo_multipart(
                    target_channel,
                    bytes,
                    filename,
                    mime,
                    caption_html,
                    keyboard,
                )
                .await;
            if !matches!(
                multipart_outcome,
                SendAttemptOutcome::DeterministicFailure { status: 400, .. }
            ) {
                return multipart_outcome;
            }
        }
        if let Some(photo_url) = &item.image_public_url {
            let url_outcome = self
                .call_send_photo(target_channel, photo_url, caption_html, keyboard)
                .await;
            if !matches!(
                url_outcome,
                SendAttemptOutcome::DeterministicFailure { status: 400, .. }
            ) {
                return url_outcome;
            }
        }
        self.call_send_message(
            target_channel,
            caption_html,
            Some(&item.share_link),
            false,
            Some(keyboard),
            None,
        )
        .await
    }

    async fn deliver_single_item(
        &self,
        target_channel: &str,
        item: &TelegramOfferQueueItem,
        caption_html: &str,
        keyboard: &TelegramInlineKeyboardMarkup,
    ) -> SendAttemptOutcome {
        let cached_photo = self.fetch_local_minio_image(item).await;
        let first_attempt = self
            .attempt_photo_or_message(
                target_channel,
                item,
                caption_html,
                keyboard,
                cached_photo.as_ref(),
            )
            .await;

        match first_attempt {
            SendAttemptOutcome::DeterministicFailure {
                status: 429,
                retry_after_secs,
                ..
            } => {
                let wait_secs = retry_after_secs.unwrap_or(1).clamp(1, 30);
                tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                let retry_attempt = self
                    .attempt_photo_or_message(
                        target_channel,
                        item,
                        caption_html,
                        keyboard,
                        cached_photo.as_ref(),
                    )
                    .await;
                match retry_attempt {
                    SendAttemptOutcome::DeterministicFailure { status: 429, .. } => {
                        SendAttemptOutcome::DeterministicFailure {
                            status: 429,
                            error_code: "telegram_rate_limited_429".to_owned(),
                            retry_after_secs: None,
                        }
                    }
                    other => other,
                }
            }
            other => other,
        }
    }

    pub async fn run_telegram_daemon_cycles(
        &self,
        hub_url: &str,
        hub_token: &str,
        poll_timeout_secs: u64,
        tick_interval: Duration,
        max_cycles: Option<usize>,
    ) -> Result<TelegramDaemonReport> {
        let mut poll_offset: Option<i64> = None;
        let mut next_post_allowed_at: Option<tokio::time::Instant> = None;
        let mut cycles_completed = 0usize;
        let mut published_count = 0usize;
        let mut failed_count = 0usize;
        let mut commands_answered = 0usize;

        loop {
            let effective_timeout = match next_post_allowed_at {
                Some(deadline) => {
                    let remaining = deadline
                        .saturating_duration_since(tokio::time::Instant::now())
                        .as_secs();
                    if remaining == 0 {
                        0
                    } else {
                        poll_timeout_secs.min(remaining)
                    }
                }
                None => poll_timeout_secs,
            };
            match self
                .poll_commands_once(hub_url, hub_token, poll_offset, effective_timeout)
                .await
            {
                Ok(poll_rep) => {
                    poll_offset = poll_rep.next_offset;
                    commands_answered += poll_rep.commands_answered;
                }
                Err(err) => {
                    tracing::warn!(%err, "telegram command poll failed; retrying next tick");
                }
            }

            if self.config.allow_live_publish
                && next_post_allowed_at
                    .is_none_or(|deadline| tokio::time::Instant::now() >= deadline)
            {
                let now_ms = crate::now_ms();
                match self
                    .publish_queued_from_hub(hub_url, hub_token, None, 1, false, now_ms)
                    .await
                {
                    Ok(rep) => {
                        published_count += rep.published_count;
                        failed_count += rep.failed_count;
                        if let Some(first) = rep.items.first() {
                            let delay = self.compute_inter_post_delay(
                                published_count + failed_count,
                                &first.delivery_id,
                            );
                            tracing::info!(
                                delivery_id = %first.delivery_id,
                                product_id = %first.product_id,
                                channel_id = %first.channel_id,
                                status = ?first.status,
                                next_interval_secs = delay.as_secs(),
                                "telegram daemon processed queued product"
                            );
                            next_post_allowed_at = Some(tokio::time::Instant::now() + delay);
                        }
                    }
                    Err(err) => {
                        tracing::warn!(%err, "telegram queue publish step failed; retrying next tick");
                    }
                }
            }

            cycles_completed += 1;
            if max_cycles.is_some_and(|max| cycles_completed >= max) {
                break;
            }
            if !tick_interval.is_zero() {
                tokio::time::sleep(tick_interval).await;
            }
        }

        Ok(TelegramDaemonReport {
            cycles_completed,
            published_count,
            failed_count,
            commands_answered,
        })
    }

    pub async fn poll_commands_once(
        &self,
        hub_url: &str,
        hub_token: &str,
        offset: Option<i64>,
        poll_timeout_secs: u64,
    ) -> Result<TelegramPollReport> {
        let mut req_body = json!({
            "limit": 50,
            "timeout": poll_timeout_secs.min(50),
            "allowed_updates": ["message"],
        });
        if let Some(off) = offset {
            req_body["offset"] = json!(off);
        }

        let url = self.bot_method_url("getUpdates");
        let resp = self
            .http
            .post(url)
            .json(&req_body)
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("telegram getUpdates transport failure"))?;
        let status = resp.status();
        if !status.is_success() {
            bail!("telegram getUpdates returned HTTP {}", status.as_u16());
        }
        let updates: TelegramUpdatesResponse = resp
            .json()
            .await
            .map_err(|_| anyhow::anyhow!("invalid JSON response from telegram getUpdates"))?;
        ensure!(updates.ok, "telegram getUpdates returned ok=false");

        let mut parsed_commands = Vec::new();
        let mut max_update_id: Option<i64> = None;
        for upd in &updates.result {
            max_update_id = Some(max_update_id.map_or(upd.update_id, |m| m.max(upd.update_id)));
            if let Some(msg) = &upd.message
                && let Some(text) = &msg.text
                && let Some(cmd) = TelegramBotCommand::parse(text)
            {
                parsed_commands.push((msg.chat.id, msg.message_id, cmd));
            }
        }

        let mut commands_answered = 0usize;
        if !parsed_commands.is_empty() {
            let mut catalog_items = self
                .fetch_hub_queue(
                    hub_url,
                    hub_token,
                    Some(&self.config.default_channel_id),
                    Some(TelegramDeliveryStatus::Published),
                    20,
                )
                .await?;
            if catalog_items.is_empty() {
                catalog_items = self
                    .fetch_hub_queue(hub_url, hub_token, None, None, 20)
                    .await?;
            }
            for (chat_id, reply_to_message_id, cmd) in parsed_commands {
                let (reply_html, markup) = format_telegram_bot_command_reply_pt_br(
                    &cmd,
                    &catalog_items,
                    self.config.site_base_url.as_deref(),
                );
                let chat_str = chat_id.to_string();
                let outcome = self
                    .call_send_message(
                        &chat_str,
                        &reply_html,
                        None,
                        true,
                        markup.as_ref(),
                        Some(reply_to_message_id),
                    )
                    .await;
                if matches!(outcome, SendAttemptOutcome::Delivered { .. }) {
                    commands_answered += 1;
                }
            }
        }

        Ok(TelegramPollReport {
            updates_processed: updates.result.len(),
            commands_answered,
            next_offset: max_update_id.map(|id| id + 1).or(offset),
        })
    }

    fn bot_method_url(&self, method: &str) -> String {
        format!(
            "{}/bot{}/{}",
            self.config.api_base_url.trim_end_matches('/'),
            self.bot_token,
            method
        )
    }

    async fn call_send_photo_multipart(
        &self,
        chat_id: &str,
        photo_bytes: &[u8],
        filename: &str,
        content_type: &str,
        caption_html: &str,
        keyboard: &TelegramInlineKeyboardMarkup,
    ) -> SendAttemptOutcome {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let boundary = format!("ZaturoTgBoundary{nanos}");
        let reply_markup_json = serde_json::to_string(keyboard).unwrap_or_else(|_| "{}".to_owned());

        let mut payload = Vec::with_capacity(photo_bytes.len() + caption_html.len() + 1024);
        for (name, val) in [
            ("chat_id", chat_id),
            ("caption", caption_html),
            ("parse_mode", "HTML"),
            ("show_caption_above_media", "false"),
            ("reply_markup", reply_markup_json.as_str()),
        ] {
            payload.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            payload.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
            );
            payload.extend_from_slice(val.as_bytes());
            payload.extend_from_slice(b"\r\n");
        }
        payload.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        payload.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"photo\"; filename=\"{filename}\"\r\n")
                .as_bytes(),
        );
        payload.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
        payload.extend_from_slice(photo_bytes);
        payload.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let url = self.bot_method_url("sendPhoto");
        let Ok(resp) = self
            .http
            .post(url)
            .header(
                reqwest::header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(payload)
            .send()
            .await
        else {
            return SendAttemptOutcome::UncertainTransportFailure;
        };
        let status = resp.status().as_u16();
        let Ok(text) = resp.text().await else {
            return SendAttemptOutcome::UncertainTransportFailure;
        };
        parse_send_response("sendPhoto", status, &text)
    }

    async fn call_send_photo(
        &self,
        chat_id: &str,
        photo_url: &str,
        caption_html: &str,
        keyboard: &TelegramInlineKeyboardMarkup,
    ) -> SendAttemptOutcome {
        let body = json!({
            "chat_id": chat_id,
            "photo": photo_url,
            "caption": caption_html,
            "parse_mode": "HTML",
            "show_caption_above_media": false,
            "reply_markup": keyboard,
        });
        self.execute_send_call("sendPhoto", &body).await
    }

    async fn call_send_message(
        &self,
        chat_id: &str,
        text_html: &str,
        preview_url: Option<&str>,
        disable_preview: bool,
        keyboard: Option<&TelegramInlineKeyboardMarkup>,
        reply_to_message_id: Option<i64>,
    ) -> SendAttemptOutcome {
        let link_preview_options = if disable_preview {
            json!({ "is_disabled": true })
        } else {
            json!({
                "is_disabled": false,
                "url": preview_url,
                "prefer_large_media": true,
            })
        };
        let mut body = json!({
            "chat_id": chat_id,
            "text": text_html,
            "parse_mode": "HTML",
            "link_preview_options": link_preview_options,
        });
        if let Some(kb) = keyboard {
            body["reply_markup"] = serde_json::to_value(kb).unwrap_or(Value::Null);
        }
        if let Some(reply_id) = reply_to_message_id {
            body["reply_parameters"] = json!({ "message_id": reply_id });
        }
        self.execute_send_call("sendMessage", &body).await
    }

    async fn execute_send_call(&self, method: &'static str, body: &Value) -> SendAttemptOutcome {
        let url = self.bot_method_url(method);
        let Ok(resp) = self.http.post(url).json(body).send().await else {
            return SendAttemptOutcome::UncertainTransportFailure;
        };
        let status = resp.status().as_u16();
        let Ok(text) = resp.text().await else {
            return SendAttemptOutcome::UncertainTransportFailure;
        };
        parse_send_response(method, status, &text)
    }
}

fn parse_send_response(method: &'static str, status: u16, text: &str) -> SendAttemptOutcome {
    if (200..300).contains(&status) {
        if let Ok(parsed) = serde_json::from_str::<TelegramSendSuccess>(text)
            && parsed.ok
            && let Some(msg) = parsed.result
        {
            return SendAttemptOutcome::Delivered {
                method,
                external_message_id: msg.message_id.to_string(),
            };
        }
        return SendAttemptOutcome::DeterministicFailure {
            status,
            error_code: format!("telegram_{method}_unexpected_ok"),
            retry_after_secs: None,
        };
    }

    let retry_after_secs = serde_json::from_str::<TelegramApiErrorBody>(text)
        .ok()
        .and_then(|err| err.parameters)
        .and_then(|p| p.retry_after);
    let error_code = if status == 429 {
        "telegram_rate_limited_429".to_owned()
    } else {
        format!("telegram_{method}_http_{status}")
    };
    SendAttemptOutcome::DeterministicFailure {
        status,
        error_code,
        retry_after_secs,
    }
}

fn extract_bucket_from_public_url<'a>(public_url: &'a str, object_key: &str) -> Option<&'a str> {
    let (_, after_scheme) = public_url.split_once("://")?;
    let (_, path) = after_scheme.split_once('/')?;
    let trimmed = path.trim_matches('/');
    let prefix = trimmed.strip_suffix(object_key.trim_start_matches('/'))?;
    let bucket = prefix.trim_matches('/').rsplit('/').next()?;
    (!bucket.is_empty()).then_some(bucket)
}

pub async fn fetch_hub_queue_with_token(
    hub_url: &str,
    hub_token: &str,
    channel_id: Option<&str>,
    status: Option<TelegramDeliveryStatus>,
    limit: u32,
) -> Result<Vec<TelegramOfferQueueItem>> {
    let http = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    fetch_hub_queue_with_client(&http, hub_url, hub_token, channel_id, status, limit).await
}

async fn fetch_hub_queue_with_client(
    http: &reqwest::Client,
    hub_url: &str,
    hub_token: &str,
    channel_id: Option<&str>,
    status: Option<TelegramDeliveryStatus>,
    limit: u32,
) -> Result<Vec<TelegramOfferQueueItem>> {
    zaturo_security::validate_endpoint(hub_url)?;
    zaturo_security::Credential::new(hub_token)?;
    let mut query: Vec<(&str, String)> = vec![("limit", limit.clamp(1, 100).to_string())];
    if let Some(ch) = channel_id {
        query.push(("channel_id", ch.to_owned()));
    }
    if let Some(st) = status {
        query.push(("status", st.as_str().to_owned()));
    }
    let url = format!(
        "{}/v1/commerce/mercadolivre/telegram-queue",
        hub_url.trim_end_matches('/')
    );
    let resp = http
        .get(url)
        .bearer_auth(hub_token)
        .query(&query)
        .send()
        .await
        .context("failed querying Hub telegram queue")?;
    let http_status = resp.status();
    if !http_status.is_success() {
        bail!("Hub telegram-queue returned HTTP {}", http_status.as_u16());
    }
    let items: Vec<TelegramOfferQueueItem> = resp
        .json()
        .await
        .context("invalid JSON response from Hub telegram-queue")?;
    Ok(items)
}

async fn ack_hub_delivery_with_client(
    http: &reqwest::Client,
    hub_url: &str,
    hub_token: &str,
    delivery_id: &str,
    ack: &TelegramDeliveryAck,
) -> Result<TelegramOfferQueueItem> {
    zaturo_security::validate_endpoint(hub_url)?;
    zaturo_security::Credential::new(hub_token)?;
    zaturo_core::validate_identifier(delivery_id).map_err(anyhow::Error::msg)?;
    ack.validates().map_err(anyhow::Error::msg)?;
    let url = format!(
        "{}/v1/commerce/mercadolivre/telegram-queue/{}/ack",
        hub_url.trim_end_matches('/'),
        delivery_id
    );
    let resp = http
        .post(url)
        .bearer_auth(hub_token)
        .json(ack)
        .send()
        .await
        .context("failed posting telegram delivery ack to Hub")?;
    let http_status = resp.status();
    if !http_status.is_success() {
        bail!(
            "Hub telegram-queue ack returned HTTP {}",
            http_status.as_u16()
        );
    }
    let item: TelegramOfferQueueItem = resp
        .json()
        .await
        .context("invalid JSON response from Hub telegram-queue ack")?;
    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        body::Bytes,
        extract::{Path as AxumPath, State},
        http::{HeaderMap, StatusCode},
        routing::{get, post},
    };
    use sha2::{Digest, Sha256};
    use std::{
        collections::VecDeque,
        os::unix::fs::PermissionsExt,
        sync::{Arc, Mutex},
    };
    use tokio::task::JoinHandle;
    use zaturo_core::{
        MercadoLivreIngestBatchRequest, MercadoLivreProductState, MercadoLivreSpec,
        MercadoLivreVariant, MinioProductImage, ValidatedMercadoLivreProduct,
    };
    use zaturo_hub::{AppState, config::HubConfig};

    const OWNER: &str = "synthetic-owner-telegram-test-token-value";
    const DEVICE: &str = "synthetic-device-telegram-test-token-value";
    const BOT_TOKEN: &str = "987654:synthetic_telegram_bot_token_value";

    struct AbortOnDrop(JoinHandle<()>);
    impl Drop for AbortOnDrop {
        fn drop(&mut self) {
            self.0.abort();
        }
    }

    fn test_tg_config(
        api_base_url: String,
        bot_token_file: PathBuf,
        min_send_interval_ms: u64,
        allow_live_publish: bool,
    ) -> TelegramBotConfig {
        TelegramBotConfig {
            api_base_url,
            bot_token_file,
            default_channel_id: "@procurali".into(),
            site_base_url: Some("https://procurali.com.br".into()),
            min_send_interval_ms,
            max_send_interval_ms: None,
            allow_live_publish,
            auto_publish_on_ingest: false,
            local_minio_endpoint: None,
            minio_bucket: None,
            env_file: None,
        }
    }

    fn minimal_png_bytes(width: u32, height: u32) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        out.extend_from_slice(&13u32.to_be_bytes());
        out.extend_from_slice(b"IHDR");
        out.extend_from_slice(&width.to_be_bytes());
        out.extend_from_slice(&height.to_be_bytes());
        out.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        out
    }

    struct MultipartPhotoRecord {
        content_type: String,
        body: Vec<u8>,
    }

    #[derive(Clone, Default)]
    struct MockTelegramState {
        send_photo_plans: Arc<Mutex<VecDeque<(u16, Value)>>>,
        send_message_calls: Arc<Mutex<Vec<Value>>>,
        send_photo_calls: Arc<Mutex<Vec<Value>>>,
        send_photo_multipart_bodies: Arc<Mutex<Vec<MultipartPhotoRecord>>>,
        updates_queue: Arc<Mutex<Vec<Value>>>,
    }

    async fn mock_tg_method(
        State(state): State<MockTelegramState>,
        AxumPath((_bot_path, method)): AxumPath<(String, String)>,
        headers: HeaderMap,
        raw_body: Bytes,
    ) -> (StatusCode, Json<Value>) {
        let ctype = headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_owned();
        if method == "sendPhoto" && ctype.starts_with("multipart/form-data") {
            state
                .send_photo_multipart_bodies
                .lock()
                .unwrap()
                .push(MultipartPhotoRecord {
                    content_type: ctype,
                    body: raw_body.to_vec(),
                });
            return (
                StatusCode::OK,
                Json(json!({ "ok": true, "result": { "message_id": 5555 } })),
            );
        }
        let body: Value = serde_json::from_slice(&raw_body).unwrap_or(Value::Null);
        match method.as_str() {
            "sendPhoto" => {
                state.send_photo_calls.lock().unwrap().push(body);
                if let Some((status, payload)) = state.send_photo_plans.lock().unwrap().pop_front()
                {
                    let code = StatusCode::from_u16(status).unwrap_or(StatusCode::BAD_REQUEST);
                    return (code, Json(payload));
                }
                (
                    StatusCode::OK,
                    Json(json!({ "ok": true, "result": { "message_id": 5001 } })),
                )
            }
            "sendMessage" => {
                state.send_message_calls.lock().unwrap().push(body);
                (
                    StatusCode::OK,
                    Json(json!({ "ok": true, "result": { "message_id": 6001 } })),
                )
            }
            "getUpdates" => {
                let updates = std::mem::take(&mut *state.updates_queue.lock().unwrap());
                (
                    StatusCode::OK,
                    Json(json!({ "ok": true, "result": updates })),
                )
            }
            _ => (
                StatusCode::NOT_FOUND,
                Json(json!({ "ok": false, "description": "unknown method" })),
            ),
        }
    }

    fn sample_validated_product(
        product_id: &str,
        title: &str,
        price_cents: i64,
    ) -> ValidatedMercadoLivreProduct {
        let mut p = ValidatedMercadoLivreProduct {
            product_id: product_id.to_owned(),
            slug: zaturo_core::slugify_pt_br(title, product_id),
            brand: "Kingston".into(),
            model: Some("SNV3S".into()),
            title: title.to_owned(),
            exact_variant: "PCIe 4.0 x4".into(),
            category_slug: "hardware-e-perifericos".into(),
            seller_name: "Loja oficial FAMILAR".into(),
            canonical_url: format!("https://www.mercadolivre.com.br/p/{product_id}"),
            share_link: "https://meli.la/1iWQZbJ".into(),
            share_id: format!("1PBCGF-{product_id}"),
            share_text: "https://meli.la/1iWQZbJ".into(),
            sponsored: false,
            commission_bp: Some(500),
            commission_label: Some("5%".into()),
            marketplace_rating: Some("4,9 / 5".into()),
            marketplace_sales_label: Some("+10 mil vendidos".into()),
            card_badge: Some("Mais vendido".into()),
            discount_label: Some("25% OFF".into()),
            installment_summary: Some("10x sem juros".into()),
            return_days: Some(30),
            availability: "InStock".into(),
            price_valid_until: None,
            price_brl_cents: price_cents,
            pix_price_cents: price_cents,
            reference_price_cents: Some(price_cents + 10_000),
            currency: "BRL".into(),
            payment_conditions: "À vista no Pix ou cartão".into(),
            short_condition: "10x sem juros".into(),
            freight_status: "Frete grátis".into(),
            observed_at_ms: crate::now_ms(),
            freshness_ttl_minutes: 30,
            state: MercadoLivreProductState::Approved,
            one_line_reason: "SSD NVMe PCIe 4.0.".into(),
            for_whom: "Upgrade de PC ou notebook.".into(),
            short_description: "Leitura até 6000 MB/s.".into(),
            long_description: "Descrição completa validada.".into(),
            supported_benefits: vec!["PCIe 4.0".into()],
            limitations: vec!["Sem dissipador incluído.".into()],
            unknowns: vec![],
            tags: vec!["ssd".into(), "hardware".into()],
            specs: vec![MercadoLivreSpec {
                name: "Capacidade".into(),
                value: "1 TB".into(),
            }],
            variants: vec![MercadoLivreVariant {
                variant_product_id: product_id.to_owned(),
                sku: "SNV3S".into(),
                name: title.to_owned(),
                color: None,
                voltage: None,
                price_brl_cents: price_cents,
                currency: "BRL".into(),
                availability: "https://schema.org/InStock".into(),
                canonical_url: None,
                return_days: Some(30),
                return_fees: Some("Grátis".into()),
                highlights: vec![],
            }],
            images: vec![MinioProductImage {
                position: 0,
                bucket: "procurali-media".into(),
                object_key: format!("mercadolivre/{product_id}/hero.png"),
                public_url: format!(
                    "https://media.procurali.com.br/procurali-media/mercadolivre/{product_id}/hero.png"
                ),
                source_origin_url: None,
                content_type: "image/png".into(),
                size_bytes: 4096,
                sha256_hex: "a".repeat(64),
                alt_text: title.to_owned(),
            }],
            evidence_sha256: "b".repeat(64),
        };
        if product_id == "MLB53222888" {
            p.share_link = "https://meli.la/2xYvZ9W".into();
            p.share_text = "https://meli.la/2xYvZ9W".into();
        }
        p
    }

    #[test]
    fn telegram_config_rejects_empty_token_and_invalid_endpoints() {
        let bad = test_tg_config(
            "ftp://api.telegram.org".into(),
            PathBuf::from("/tmp/tg.token"),
            0,
            false,
        );
        assert!(bad.validate().is_err());

        let mut bad_max = test_tg_config(
            "https://api.telegram.org".into(),
            PathBuf::from("/tmp/tg.token"),
            60_000,
            false,
        );
        bad_max.max_send_interval_ms = Some(30_000);
        assert!(bad_max.validate().is_err());

        let good = test_tg_config(
            "https://api.telegram.org".into(),
            PathBuf::from("/tmp/tg.token"),
            1100,
            false,
        );
        good.validate().unwrap();
    }

    #[test]
    fn env_file_overrides_and_natural_one_to_three_minute_delay_work() {
        let dir = tempfile::tempdir().unwrap();
        let env_path = dir.path().join("telegram.env");
        fs::write(
            &env_path,
            [
                "# Procurali Telegram bot test env",
                "TELEGRAM_BOT_TOKEN=\"synthetic-123456:env_file_bot_token\"",
                "TELEGRAM_CHANNEL_ID=-1001987654321",
                "TELEGRAM_MIN_INTERVAL_SECS=60",
                "TELEGRAM_MAX_INTERVAL_SECS=180",
                "TELEGRAM_ALLOW_LIVE_PUBLISH=true",
                "TELEGRAM_AUTO_PUBLISH_ON_INGEST=true",
            ]
            .join("\n"),
        )
        .unwrap();

        let mut cfg = test_tg_config(
            "https://api.telegram.org".into(),
            PathBuf::new(),
            1100,
            false,
        );
        cfg.env_file = Some(env_path);

        let client = TelegramBotClient::from_config(cfg).unwrap();
        assert_eq!(client.config().default_channel_id, "-1001987654321");
        assert!(client.config().allow_live_publish);
        assert!(client.config().auto_publish_on_ingest);
        assert_eq!(client.effective_send_interval_range_ms(), (60_000, 180_000));

        for idx in 0..10 {
            let delay = client.compute_inter_post_delay(idx, &format!("tg-delivery-{idx}"));
            assert!(
                delay >= Duration::from_secs(60) && delay <= Duration::from_secs(180),
                "expected 60..=180s delay, got {:?}",
                delay
            );
        }
    }

    #[tokio::test]
    async fn end_to_end_hub_telegram_publish_retry_fallback_polling_and_whatsapp_assisted() {
        let dir = tempfile::tempdir().unwrap();
        let hub_dir = dir.path().join("hub");
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
        let _hub_server = AbortOnDrop(tokio::spawn(async move {
            axum::serve(hub_listener, zaturo_hub::router(hub_state))
                .await
                .unwrap();
        }));

        // Ingest 2 validated products into Hub queue.
        let mut c1 = sample_validated_product("MLB39410080", "SSD Kingston NV3 1TB NVMe", 44_990);
        c1.state = MercadoLivreProductState::Candidate;
        let mut c2 =
            sample_validated_product("MLB53222888", "Teclado Mecanico Redragon K500", 13_879);
        c2.state = MercadoLivreProductState::Candidate;
        let batch = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "hardware".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: 1_763_252_000_000,
            products: vec![c1, c2],
        };
        crate::mercadolivre::ingest_to_hub(&hub_url, DEVICE, &batch, "tg-e2e-batch-1")
            .await
            .unwrap();
        let empty = TelegramBotClient::from_config(test_tg_config(
            "http://127.0.0.1:9".into(),
            write_token(dir.path(), "unused.token"),
            0,
            true,
        ))
        .unwrap()
        .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
        .await
        .unwrap();
        assert_eq!(
            empty.fetched_count, 0,
            "ingest must not approve publication"
        );
        for product in ["MLB39410080", "MLB53222888"] {
            owner_approve(&hub_url, product).await;
        }

        // Start mock Telegram Bot API server.
        let tg_state = MockTelegramState::default();
        {
            let mut plans = tg_state.send_photo_plans.lock().unwrap();
            // Item 1: 429 retry_after=1 then 200 OK
            plans.push_back((
                429,
                json!({
                    "ok": false,
                    "error_code": 429,
                    "description": "Too Many Requests: retry after 1",
                    "parameters": { "retry_after": 1 }
                }),
            ));
            plans.push_back((200, json!({ "ok": true, "result": { "message_id": 7001 } })));
            // Item 2: 400 Bad Request -> falls back cleanly to sendMessage
            plans.push_back((
                400,
                json!({ "ok": false, "error_code": 400, "description": "Bad Request: failed to get HTTP URL content" }),
            ));
        }

        let tg_router = Router::new()
            .route("/{bot_token}/{method}", post(mock_tg_method))
            .with_state(tg_state.clone());
        let tg_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tg_addr = tg_listener.local_addr().unwrap();
        let _tg_server = AbortOnDrop(tokio::spawn(async move {
            axum::serve(tg_listener, tg_router).await.unwrap();
        }));

        let token_file = dir.path().join("telegram.token");
        fs::write(&token_file, format!("{BOT_TOKEN}\n")).unwrap();
        fs::set_permissions(&token_file, fs::Permissions::from_mode(0o600)).unwrap();

        let cfg = test_tg_config(format!("http://{tg_addr}"), token_file, 0, true);
        let tg = TelegramBotClient::from_config(cfg).unwrap();

        // 1) Dry run produces previews & WhatsApp Web drafts without sending or acking.
        let preview = tg
            .publish_queued_from_hub(&hub_url, DEVICE, None, 10, true, crate::now_ms())
            .await
            .unwrap();
        assert!(preview.dry_run);
        assert_eq!(preview.fetched_count, 2);
        assert_eq!(preview.whatsapp_package.draft_count, 2);
        assert!(
            preview.items[0]
                .caption_html_preview
                .contains("OFERTA RELÂMPAGO")
        );
        assert!(tg_state.send_photo_calls.lock().unwrap().is_empty());

        // 2) Live publish exercises 429 retry and 400 -> sendMessage fallback, acking Hub.
        let report = tg
            .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
            .await
            .unwrap();
        assert!(!report.dry_run);
        assert_eq!(report.fetched_count, 2);
        assert_eq!(report.published_count, 2);
        assert_eq!(report.failed_count, 0);
        assert_eq!(tg_state.send_photo_calls.lock().unwrap().len(), 3);
        assert_eq!(tg_state.send_message_calls.lock().unwrap().len(), 1);
        assert_eq!(
            report.whatsapp_package.mode,
            "whatsapp_web_human_assisted_no_meta_api"
        );
        assert!(
            report
                .whatsapp_package
                .combined_digest_text
                .contains("https://meli.la/1iWQZbJ")
        );

        let published_queue = tg
            .fetch_hub_queue(
                &hub_url,
                DEVICE,
                Some("@procurali"),
                Some(TelegramDeliveryStatus::Published),
                10,
            )
            .await
            .unwrap();
        assert_eq!(published_queue.len(), 2);

        // 3) Interactive command polling (/start, /ofertas, /buscar, /whatsapp)
        {
            let mut updates = tg_state.updates_queue.lock().unwrap();
            updates.push(json!({
                "update_id": 101,
                "message": { "message_id": 11, "chat": { "id": 55001 }, "text": "/start" }
            }));
            updates.push(json!({
                "update_id": 102,
                "message": { "message_id": 12, "chat": { "id": 55001 }, "text": "/buscar kingston" }
            }));
            updates.push(json!({
                "update_id": 103,
                "message": { "message_id": 13, "chat": { "id": 55001 }, "text": "/whatsapp" }
            }));
        }
        let poll = tg
            .poll_commands_once(&hub_url, DEVICE, None, 0)
            .await
            .unwrap();
        assert_eq!(poll.updates_processed, 3);
        assert_eq!(poll.commands_answered, 3);
        assert_eq!(poll.next_offset, Some(104));

        {
            let messages = tg_state.send_message_calls.lock().unwrap();
            // 1 fallback sendMessage + 3 interactive command replies = 4
            assert_eq!(messages.len(), 4);
            assert!(
                messages[2]["text"]
                    .as_str()
                    .unwrap()
                    .contains("https://meli.la/1iWQZbJ")
            );
            assert!(
                messages[3]["text"]
                    .as_str()
                    .unwrap()
                    .contains("WhatsApp Web")
            );
        }

        // 4) Uncertain transport failure (closed loopback port) marks Failed without retrying.
        let dead_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let dead_addr = dead_listener.local_addr().unwrap();
        drop(dead_listener);

        let mut c3 =
            sample_validated_product("MLB48468943", "Headset Gamer Redragon Zeus Lite", 19_990);
        c3.state = MercadoLivreProductState::Candidate;
        let batch2 = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "headset".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: 1_763_252_300_000,
            products: vec![c3],
        };
        crate::mercadolivre::ingest_to_hub(&hub_url, DEVICE, &batch2, "tg-e2e-batch-2")
            .await
            .unwrap();
        owner_approve(&hub_url, "MLB48468943").await;
        let dead_token_file = dir.path().join("telegram-dead.token");
        fs::write(&dead_token_file, format!("{BOT_TOKEN}\n")).unwrap();
        fs::set_permissions(&dead_token_file, fs::Permissions::from_mode(0o600)).unwrap();
        let dead_tg = TelegramBotClient::from_config(test_tg_config(
            format!("http://{dead_addr}"),
            dead_token_file,
            0,
            true,
        ))
        .unwrap();
        let dead_report = dead_tg
            .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
            .await
            .unwrap();
        assert_eq!(dead_report.fetched_count, 1);
        assert_eq!(dead_report.published_count, 0);
        assert_eq!(dead_report.failed_count, 1);
        assert_eq!(
            dead_report.items[0].error_code.as_deref(),
            Some("telegram_uncertain_network")
        );

        let failed_queue = dead_tg
            .fetch_hub_queue(
                &hub_url,
                DEVICE,
                Some("@procurali"),
                Some(TelegramDeliveryStatus::Failed),
                10,
            )
            .await
            .unwrap();
        assert_eq!(failed_queue.len(), 1);
    }

    #[tokio::test]
    async fn multipart_photo_upload_from_local_minio_and_daemon_cycle_work() {
        let dir = tempfile::tempdir().unwrap();
        let hub_dir = dir.path().join("hub");
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
        let _hub_server = AbortOnDrop(tokio::spawn(async move {
            axum::serve(hub_listener, zaturo_hub::router(hub_state))
                .await
                .unwrap();
        }));

        let png_bytes = minimal_png_bytes(600, 600);
        let served_png = png_bytes.clone();
        let minio_router = Router::new().route(
            "/{bucket}/{*key}",
            get(move || {
                let bytes = served_png.clone();
                async move { (StatusCode::OK, bytes) }
            }),
        );
        let minio_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let minio_addr = minio_listener.local_addr().unwrap();
        let _minio_server = AbortOnDrop(tokio::spawn(async move {
            axum::serve(minio_listener, minio_router).await.unwrap();
        }));

        // Ingest offer queued under @procurali, then have a custom group ID claim & publish it.
        let batch = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "ssd".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: 1_763_252_400_000,
            products: vec![sample_validated_product(
                "MLB39410080",
                "SSD Kingston NV3 1TB NVMe",
                44_990,
            )],
        };
        crate::mercadolivre::ingest_to_hub(&hub_url, DEVICE, &batch, "tg-mp-batch-1")
            .await
            .unwrap();

        let tg_state = MockTelegramState::default();
        tg_state.updates_queue.lock().unwrap().push(json!({
            "update_id": 201,
            "message": { "message_id": 21, "chat": { "id": -1001555666777i64 }, "text": "/ofertas" }
        }));
        let tg_router = Router::new()
            .route("/{bot_token}/{method}", post(mock_tg_method))
            .with_state(tg_state.clone());
        let tg_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tg_addr = tg_listener.local_addr().unwrap();
        let _tg_server = AbortOnDrop(tokio::spawn(async move {
            axum::serve(tg_listener, tg_router).await.unwrap();
        }));

        let env_path = dir.path().join("telegram.env");
        fs::write(
            &env_path,
            [
                "TELEGRAM_BOT_TOKEN=synthetic-987654:telegram_bot_token_value",
                "TELEGRAM_CHANNEL_ID=-1001555666777",
                "TELEGRAM_MIN_INTERVAL_SECS=1",
                "TELEGRAM_MAX_INTERVAL_SECS=2",
                "TELEGRAM_ALLOW_LIVE_PUBLISH=true",
            ]
            .join("\n"),
        )
        .unwrap();
        let mut cfg = test_tg_config(format!("http://{tg_addr}"), PathBuf::new(), 0, false);
        cfg.env_file = Some(env_path);
        cfg.local_minio_endpoint = Some(format!("http://{minio_addr}"));
        cfg.minio_bucket = Some("procurali-media".into());

        let tg = TelegramBotClient::from_config(cfg).unwrap();
        let daemon_rep = tg
            .run_telegram_daemon_cycles(&hub_url, DEVICE, 0, Duration::ZERO, Some(1))
            .await
            .unwrap();
        assert_eq!(daemon_rep.cycles_completed, 1);
        assert_eq!(daemon_rep.published_count, 1);
        assert_eq!(daemon_rep.failed_count, 0);
        assert_eq!(daemon_rep.commands_answered, 1);

        let multipart_calls = tg_state.send_photo_multipart_bodies.lock().unwrap();
        assert_eq!(multipart_calls.len(), 1);
        assert!(
            multipart_calls[0]
                .content_type
                .starts_with("multipart/form-data; boundary=")
        );
        let body_lossy = String::from_utf8_lossy(&multipart_calls[0].body);
        assert!(body_lossy.contains("-1001555666777"));
        assert!(body_lossy.contains("OFERTA RELÂMPAGO"));
        assert!(body_lossy.contains("filename=\"offer.png\""));
        assert!(
            multipart_calls[0]
                .body
                .windows(png_bytes.len())
                .any(|w| w == png_bytes.as_slice())
        );
    }

    fn write_token(dir: &std::path::Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, format!("{BOT_TOKEN}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        path
    }

    async fn owner_approve(hub_url: &str, product_id: &str) {
        let approved: TelegramOfferQueueItem = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .post(format!(
                "{hub_url}/v1/commerce/mercadolivre/telegram-queue/approve"
            ))
            .bearer_auth(OWNER)
            .json(&json!({"product_id": product_id, "channel_id": "@procurali"}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(approved.status, TelegramDeliveryStatus::Queued);
    }

    #[derive(Clone, Default)]
    struct FakeHub {
        deliveries: Arc<Mutex<Vec<TelegramOfferQueueItem>>>,
        acks: Arc<Mutex<Vec<(String, TelegramDeliveryStatus)>>>,
        fail_claims: Arc<std::sync::atomic::AtomicBool>,
        fail_results: Arc<std::sync::atomic::AtomicBool>,
    }

    async fn fake_queue(State(hub): State<FakeHub>) -> Json<Vec<TelegramOfferQueueItem>> {
        Json(
            hub.deliveries
                .lock()
                .unwrap()
                .iter()
                .filter(|item| item.status == TelegramDeliveryStatus::Queued)
                .cloned()
                .collect(),
        )
    }

    async fn fake_ack(
        State(hub): State<FakeHub>,
        AxumPath(id): AxumPath<String>,
        Json(ack): Json<TelegramDeliveryAck>,
    ) -> Result<Json<TelegramOfferQueueItem>, StatusCode> {
        use std::sync::atomic::Ordering;
        hub.acks.lock().unwrap().push((id.clone(), ack.status));
        let failing = if ack.status == TelegramDeliveryStatus::Sending {
            &hub.fail_claims
        } else {
            &hub.fail_results
        };
        if failing.load(Ordering::SeqCst) {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        let mut deliveries = hub.deliveries.lock().unwrap();
        let item = deliveries
            .iter_mut()
            .find(|item| item.delivery_id == id)
            .ok_or(StatusCode::NOT_FOUND)?;
        item.status = ack.status;
        Ok(Json(item.clone()))
    }

    fn queued_item(id: &str) -> TelegramOfferQueueItem {
        TelegramOfferQueueItem {
            delivery_id: id.into(),
            product_id: "MLB39410080".into(),
            offer_id: format!("obs-{id}"),
            channel_id: "@procurali".into(),
            title: "SSD Kingston NV3 1TB".into(),
            brand: "Kingston".into(),
            model: None,
            price_brl_cents: 44_990,
            reference_price_cents: None,
            discount_label: None,
            share_link: "https://meli.la/1iWQZbJ".into(),
            share_id: "1PBCGF-SMN5".into(),
            image_public_url: Some(
                "https://media.procurali.com.br/procurali-media/ml/MLB39410080/hero.jpg".into(),
            ),
            image_object_key: Some("ml/MLB39410080/hero.jpg".into()),
            message_text: "SSD".into(),
            status: TelegramDeliveryStatus::Queued,
            external_message_id: None,
            created_at_ms: 1,
            updated_at_ms: 1,
            published_at_ms: None,
        }
    }

    #[tokio::test]
    async fn publisher_claims_before_sending_and_never_resends_after_a_lost_ack() {
        use std::sync::atomic::Ordering;
        let dir = tempfile::tempdir().unwrap();
        let hub = FakeHub::default();
        hub.deliveries
            .lock()
            .unwrap()
            .extend([queued_item("tg-one"), queued_item("tg-two")]);
        let hub_router = Router::new()
            .route(
                "/v1/commerce/mercadolivre/telegram-queue",
                axum::routing::get(fake_queue),
            )
            .route(
                "/v1/commerce/mercadolivre/telegram-queue/{id}/ack",
                post(fake_ack),
            )
            .with_state(hub.clone());
        let hub_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let hub_url = format!("http://{}", hub_listener.local_addr().unwrap());
        let _hub = AbortOnDrop(tokio::spawn(async move {
            axum::serve(hub_listener, hub_router).await.unwrap();
        }));

        let tg_state = MockTelegramState::default();
        let tg_router = Router::new()
            .route("/{bot_token}/{method}", post(mock_tg_method))
            .with_state(tg_state.clone());
        let tg_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let tg_addr = tg_listener.local_addr().unwrap();
        let _tg = AbortOnDrop(tokio::spawn(async move {
            axum::serve(tg_listener, tg_router).await.unwrap();
        }));
        let tg = TelegramBotClient::from_config(test_tg_config(
            format!("http://{tg_addr}"),
            write_token(dir.path(), "telegram.token"),
            0,
            true,
        ))
        .unwrap();
        let sends = || tg_state.send_photo_calls.lock().unwrap().len();
        let status_of = |id: &str| {
            hub.deliveries
                .lock()
                .unwrap()
                .iter()
                .find(|item| item.delivery_id == id)
                .unwrap()
                .status
        };

        // The send succeeds but the Hub loses the result: the run stops and the delivery
        // stays claimed (`sending`) instead of returning to the queue.
        hub.fail_results.store(true, Ordering::SeqCst);
        let lost = tg
            .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
            .await
            .unwrap();
        assert_eq!(sends(), 1);
        assert_eq!(lost.published_count, 1);
        assert_eq!(lost.items.len(), 1);
        assert_eq!(
            lost.hub_error.as_deref(),
            Some("hub_ack_failed_delivery_left_sending")
        );
        assert_eq!(status_of("tg-one"), TelegramDeliveryStatus::Sending);
        assert_eq!(status_of("tg-two"), TelegramDeliveryStatus::Queued);
        assert_eq!(
            hub.acks.lock().unwrap()[0],
            ("tg-one".into(), TelegramDeliveryStatus::Sending),
            "the claim must precede the external send"
        );

        // A later run never sends the claimed delivery again.
        hub.fail_results.store(false, Ordering::SeqCst);
        let next = tg
            .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
            .await
            .unwrap();
        assert_eq!(sends(), 2);
        assert_eq!(next.hub_error, None);
        assert_eq!(next.items[0].delivery_id, "tg-two");
        assert_eq!(status_of("tg-one"), TelegramDeliveryStatus::Sending);
        assert_eq!(status_of("tg-two"), TelegramDeliveryStatus::Published);

        // Without a recorded claim nothing is sent.
        hub.deliveries.lock().unwrap().push(queued_item("tg-three"));
        hub.fail_claims.store(true, Ordering::SeqCst);
        let unclaimed = tg
            .publish_queued_from_hub(&hub_url, DEVICE, None, 10, false, crate::now_ms())
            .await
            .unwrap();
        assert_eq!(sends(), 2);
        assert_eq!(unclaimed.hub_error.as_deref(), Some("hub_claim_failed"));
        assert!(unclaimed.items.is_empty());
    }
}
