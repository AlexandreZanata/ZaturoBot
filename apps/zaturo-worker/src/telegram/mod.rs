//! Official Telegram Bot API client, queue publisher, interactive command poller,
//! and WhatsApp Web human-assisted draft builder (without Meta API or unofficial browser bots).

mod commands;
mod config;
mod hub_bridge;
mod publisher;

#[cfg(test)]
mod tests;

pub use config::{TelegramBotConfig, load_telegram_env_map};
pub use hub_bridge::fetch_hub_queue_with_token;

use anyhow::{Context, Result, bail, ensure};
use config::{PLACEHOLDER_BOT_TOKEN, pick_env};
use hub_bridge::{ack_hub_delivery_with_client, fetch_hub_queue_with_client};
use serde::{Deserialize, Serialize};
use std::{fmt, fs, time::Duration};
use zaturo_core::{
    TelegramDeliveryAck, TelegramDeliveryStatus, TelegramInlineKeyboardMarkup,
    TelegramOfferQueueItem, WhatsappAssistedPackage, build_whatsapp_assisted_package,
};

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
    pub(super) config: TelegramBotConfig,
    pub(super) bot_token: String,
    pub(super) http: reqwest::Client,
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
}
