use anyhow::{Context, Result, bail};
use std::time::Duration;
use zaturo_core::{TelegramDeliveryAck, TelegramDeliveryStatus, TelegramOfferQueueItem};

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

pub(super) async fn fetch_hub_queue_with_client(
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

pub(super) async fn ack_hub_delivery_with_client(
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
