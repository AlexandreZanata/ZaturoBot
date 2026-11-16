use crate::{
    AppState,
    http::{ApiFailure, database},
    now_ms,
};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use zaturo_core::{TelegramDeliveryAck, TelegramDeliveryStatus, TelegramOfferQueueItem};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ApproveRequest {
    product_id: String,
    channel_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct QueueQuery {
    channel_id: Option<String>,
    status: Option<TelegramDeliveryStatus>,
    limit: Option<u32>,
}

pub(super) async fn telegram_queue(
    State(state): State<AppState>,
    Query(query): Query<QueueQuery>,
) -> Result<Json<Vec<TelegramOfferQueueItem>>, ApiFailure> {
    let limit = query.limit.unwrap_or(20.min(state.config.max_page_size));
    let items = database(&state, move |e| {
        e.mercadolivre_telegram_queue(query.channel_id.as_deref(), query.status, limit, now_ms())
    })
    .await?;
    Ok(Json(items))
}

/// Owner-only per-offer publication approval; devices and apps can never approve.
pub(super) async fn telegram_approve(
    State(state): State<AppState>,
    Json(request): Json<ApproveRequest>,
) -> Result<Json<TelegramOfferQueueItem>, ApiFailure> {
    let item = database(&state, move |e| {
        e.approve_mercadolivre_telegram_delivery(&request.product_id, &request.channel_id, now_ms())
    })
    .await?;
    state.changes.notify_waiters();
    Ok(Json(item))
}

pub(super) async fn telegram_ack(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(ack): Json<TelegramDeliveryAck>,
) -> Result<Json<TelegramOfferQueueItem>, ApiFailure> {
    let item = database(&state, move |e| {
        e.ack_mercadolivre_telegram_delivery(&id, ack, now_ms())
    })
    .await?;
    Ok(Json(item))
}
