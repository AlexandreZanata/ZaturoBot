//! Official Telegram Bot API client, queue publisher, interactive command poller,
//! and WhatsApp Web human-assisted draft builder (without Meta API or unofficial browser bots).

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{fmt, fs, path::PathBuf, time::Duration};
use zaturo_core::{
    TelegramBotCommand, TelegramDeliveryAck, TelegramDeliveryStatus, TelegramInlineKeyboardMarkup,
    TelegramOfferQueueItem, WhatsappAssistedPackage, build_telegram_offer_keyboard,
    build_whatsapp_assisted_package, format_telegram_bot_command_reply_pt_br,
    format_telegram_html_caption_pt_br, valid_telegram_channel,
};

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
    pub bot_token_file: PathBuf,
    #[serde(default = "default_channel_id")]
    pub default_channel_id: String,
    #[serde(default)]
    pub site_base_url: Option<String>,
    #[serde(default = "default_send_interval_ms")]
    pub min_send_interval_ms: u64,
    #[serde(default)]
    pub allow_live_publish: bool,
}

impl TelegramBotConfig {
    pub fn validate(&self) -> Result<()> {
        zaturo_security::validate_endpoint(&self.api_base_url)?;
        ensure!(
            !self.bot_token_file.as_os_str().is_empty(),
            "telegram bot_token_file must be set"
        );
        ensure!(
            valid_telegram_channel(&self.default_channel_id),
            "invalid telegram default_channel_id"
        );
        if let Some(site_url) = &self.site_base_url {
            zaturo_security::validate_endpoint(site_url)?;
        }
        Ok(())
    }
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
            .field("allow_live_publish", &self.config.allow_live_publish)
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
        config.validate()?;
        let env_token = std::env::var("ZATURO_TELEGRAM_BOT_TOKEN")
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        let bot_token = match env_token {
            Some(tok) => tok,
            None => fs::read_to_string(&config.bot_token_file)
                .with_context(|| {
                    format!(
                        "failed reading Telegram bot token file {}",
                        config.bot_token_file.display()
                    )
                })?
                .trim()
                .to_owned(),
        };
        ensure!(
            !bot_token.is_empty() && bot_token.bytes().all(|b| b.is_ascii_graphic()),
            "Telegram bot token is empty or contains invalid whitespace"
        );
        zaturo_security::Credential::new(&bot_token)?;
        let http = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            config,
            bot_token,
            http,
        })
    }

    pub fn config(&self) -> &TelegramBotConfig {
        &self.config
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

        let queued = self
            .fetch_hub_queue(
                hub_url,
                hub_token,
                Some(&target_channel),
                Some(TelegramDeliveryStatus::Queued),
                limit,
            )
            .await?;

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
                        channel_id: item.channel_id.clone(),
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
            if idx > 0 && self.config.min_send_interval_ms > 0 {
                tokio::time::sleep(Duration::from_millis(self.config.min_send_interval_ms)).await;
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
                    channel_id: item.channel_id.clone(),
                    method: method_for(item),
                    status: claimed_status,
                    external_message_id: None,
                    error_code: Some("not_claimable".to_owned()),
                    caption_html_preview: caption,
                    reply_markup: keyboard,
                });
                continue;
            }

            let outcome = self.deliver_single_item(item, &caption, &keyboard).await;
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
                channel_id: item.channel_id.clone(),
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

    async fn deliver_single_item(
        &self,
        item: &TelegramOfferQueueItem,
        caption_html: &str,
        keyboard: &TelegramInlineKeyboardMarkup,
    ) -> SendAttemptOutcome {
        let first_attempt = if let Some(photo_url) = &item.image_public_url {
            self.call_send_photo(&item.channel_id, photo_url, caption_html, keyboard)
                .await
        } else {
            self.call_send_message(
                &item.channel_id,
                caption_html,
                Some(&item.share_link),
                false,
                Some(keyboard),
                None,
            )
            .await
        };

        match first_attempt {
            SendAttemptOutcome::Delivered { .. }
            | SendAttemptOutcome::UncertainTransportFailure => first_attempt,
            SendAttemptOutcome::DeterministicFailure {
                status: 429,
                retry_after_secs,
                ..
            } => {
                let wait_secs = retry_after_secs.unwrap_or(1).clamp(1, 30);
                tokio::time::sleep(Duration::from_secs(wait_secs)).await;
                let retry_attempt = if let Some(photo_url) = &item.image_public_url {
                    self.call_send_photo(&item.channel_id, photo_url, caption_html, keyboard)
                        .await
                } else {
                    self.call_send_message(
                        &item.channel_id,
                        caption_html,
                        Some(&item.share_link),
                        false,
                        Some(keyboard),
                        None,
                    )
                    .await
                };
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
            SendAttemptOutcome::DeterministicFailure { status: 400, .. }
                if item.image_public_url.is_some() =>
            {
                self.call_send_message(
                    &item.channel_id,
                    caption_html,
                    Some(&item.share_link),
                    false,
                    Some(keyboard),
                    None,
                )
                .await
            }
            other => other,
        }
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
            let catalog_items = self
                .fetch_hub_queue(
                    hub_url,
                    hub_token,
                    Some(&self.config.default_channel_id),
                    Some(TelegramDeliveryStatus::Published),
                    20,
                )
                .await?;
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
        if (200..300).contains(&status) {
            if let Ok(parsed) = serde_json::from_str::<TelegramSendSuccess>(&text)
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

        let retry_after_secs = serde_json::from_str::<TelegramApiErrorBody>(&text)
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
        extract::{Path as AxumPath, State},
        http::StatusCode,
        routing::post,
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

    #[derive(Clone, Default)]
    struct MockTelegramState {
        send_photo_plans: Arc<Mutex<VecDeque<(u16, Value)>>>,
        send_message_calls: Arc<Mutex<Vec<Value>>>,
        send_photo_calls: Arc<Mutex<Vec<Value>>>,
        updates_queue: Arc<Mutex<Vec<Value>>>,
    }

    async fn mock_tg_method(
        State(state): State<MockTelegramState>,
        AxumPath((_bot_path, method)): AxumPath<(String, String)>,
        Json(body): Json<Value>,
    ) -> (StatusCode, Json<Value>) {
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
                object_key: format!("ml/{product_id}/hero.jpg"),
                public_url: format!(
                    "https://media.procurali.com.br/procurali-media/ml/{product_id}/hero.jpg"
                ),
                source_origin_url: None,
                content_type: "image/jpeg".into(),
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
        let bad = TelegramBotConfig {
            api_base_url: "ftp://api.telegram.org".into(),
            bot_token_file: PathBuf::from("/tmp/tg.token"),
            default_channel_id: "@procurali".into(),
            site_base_url: Some("https://procurali.com.br".into()),
            min_send_interval_ms: 0,
            allow_live_publish: false,
        };
        assert!(bad.validate().is_err());

        let good = TelegramBotConfig {
            api_base_url: "https://api.telegram.org".into(),
            bot_token_file: PathBuf::from("/tmp/tg.token"),
            default_channel_id: "@procurali".into(),
            site_base_url: Some("https://procurali.com.br".into()),
            min_send_interval_ms: 1100,
            allow_live_publish: false,
        };
        good.validate().unwrap();
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
        let batch = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "hardware".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: 1_763_252_000_000,
            products: vec![
                sample_validated_product("MLB39410080", "SSD Kingston NV3 1TB NVMe", 44_990),
                sample_validated_product("MLB53222888", "Teclado Mecanico Redragon K500", 13_879),
            ],
        };
        crate::mercadolivre::ingest_to_hub(&hub_url, DEVICE, &batch, "tg-e2e-batch-1")
            .await
            .unwrap();
        let empty = TelegramBotClient::from_config(TelegramBotConfig {
            api_base_url: "http://127.0.0.1:9".into(),
            bot_token_file: write_token(dir.path(), "unused.token"),
            default_channel_id: "@procurali".into(),
            site_base_url: None,
            min_send_interval_ms: 0,
            allow_live_publish: true,
        })
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

        let cfg = TelegramBotConfig {
            api_base_url: format!("http://{tg_addr}"),
            bot_token_file: token_file,
            default_channel_id: "@procurali".into(),
            site_base_url: Some("https://procurali.com.br".into()),
            min_send_interval_ms: 0,
            allow_live_publish: true,
        };
        let tg = TelegramBotClient::from_config(cfg).unwrap();

        // 1) Dry run produces previews & WhatsApp Web drafts without sending or acking.
        let preview = tg
            .publish_queued_from_hub(&hub_url, DEVICE, None, 10, true, crate::now_ms())
            .await
            .unwrap();
        assert!(preview.dry_run);
        assert_eq!(preview.fetched_count, 2);
        assert_eq!(preview.whatsapp_package.draft_count, 2);
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

        let batch2 = MercadoLivreIngestBatchRequest {
            worker_id: "worker-ml".into(),
            search_query: "headset".into(),
            channel_id: "@procurali".into(),
            captured_at_ms: 1_763_252_300_000,
            products: vec![sample_validated_product(
                "MLB48468943",
                "Headset Gamer Redragon Zeus Lite",
                19_990,
            )],
        };
        crate::mercadolivre::ingest_to_hub(&hub_url, DEVICE, &batch2, "tg-e2e-batch-2")
            .await
            .unwrap();
        owner_approve(&hub_url, "MLB48468943").await;
        let dead_token_file = dir.path().join("telegram-dead.token");
        fs::write(&dead_token_file, format!("{BOT_TOKEN}\n")).unwrap();
        fs::set_permissions(&dead_token_file, fs::Permissions::from_mode(0o600)).unwrap();
        let dead_tg = TelegramBotClient::from_config(TelegramBotConfig {
            api_base_url: format!("http://{dead_addr}"),
            bot_token_file: dead_token_file,
            default_channel_id: "@procurali".into(),
            site_base_url: Some("https://procurali.com.br".into()),
            min_send_interval_ms: 0,
            allow_live_publish: true,
        })
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
        let tg = TelegramBotClient::from_config(TelegramBotConfig {
            api_base_url: format!("http://{tg_addr}"),
            bot_token_file: write_token(dir.path(), "telegram.token"),
            default_channel_id: "@procurali".into(),
            site_base_url: None,
            min_send_interval_ms: 0,
            allow_live_publish: true,
        })
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
