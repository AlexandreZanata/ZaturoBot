use super::{TelegramBotClient, TelegramPublishItemOutcome, TelegramPublishReport};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;
use zaturo_core::{
    TelegramDeliveryAck, TelegramDeliveryStatus, TelegramInlineKeyboardMarkup,
    TelegramOfferQueueItem, build_telegram_offer_keyboard, build_whatsapp_assisted_package,
    format_telegram_html_caption_pt_br, valid_minio_object_key, valid_telegram_channel,
};
use zaturo_transfer::minio::{ImageFormat, inspect_image_bytes};

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

pub(super) enum SendAttemptOutcome {
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

    pub(super) fn bot_method_url(&self, method: &str) -> String {
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

    pub(super) async fn call_send_message(
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
