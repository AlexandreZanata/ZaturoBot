use super::{
    TelegramBotClient, TelegramDaemonReport, TelegramPollReport, publisher::SendAttemptOutcome,
};
use anyhow::{Result, bail, ensure};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use zaturo_core::{
    TelegramBotCommand, TelegramDeliveryStatus, format_telegram_bot_command_reply_pt_br,
};

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

impl TelegramBotClient {
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
}
