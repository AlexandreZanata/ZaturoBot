use super::CompiledCommerceIndex;
use crate::{Engine, Error, Result};
use zaturo_core::{
    ExposureBeaconRequest, ExposureBeaconResponse, PriceAlertActionRequest, PriceAlertResponse,
    PriceAlertSubscription, SiteCatalogOffer, TriggeredPriceAlert,
};

const MAX_PRICE_ALERTS: usize = 4096;

impl Engine {
    /// Non-blocking `/api/exposicao/` viewport impression recorder.
    pub fn mercadolivre_record_exposure(
        &self,
        req: ExposureBeaconRequest,
    ) -> Result<ExposureBeaconResponse> {
        req.validates().map_err(Error::Invalid)?;
        let accepted = req.events.len();
        if let Ok(mut guard) = self.commerce_cache.telemetry.write() {
            for ev in &req.events {
                guard.entry(ev.product_id.clone()).or_default().impressions += 1;
            }
        }
        Ok(ExposureBeaconResponse {
            accepted,
            route: req.route,
        })
    }

    /// Cloobe `/api/notificacoes/` alert management & target-price trigger evaluation.
    pub fn mercadolivre_handle_price_alert(
        &self,
        req: PriceAlertActionRequest,
        now_ms: i64,
    ) -> Result<PriceAlertResponse> {
        if now_ms <= 0 {
            return Err(Error::Invalid("now_ms must be positive".into()));
        }
        req.validates().map_err(Error::Invalid)?;
        let index = self.mercadolivre_site_catalog_cached(now_ms)?;
        let offers = &index.snapshot.offers;

        match req.action.as_str() {
            "criar-alerta" => {
                let pid = req.product_id.as_deref().ok_or_else(|| {
                    Error::Invalid("product_id is required for criar-alerta".into())
                })?;
                if !index.by_product_id.contains_key(pid) {
                    return Err(Error::NotFound);
                }
                let target = req.target_price_cents.unwrap_or(0);
                let mut guard = self
                    .commerce_cache
                    .price_alerts
                    .write()
                    .map_err(|_| Error::Storage(zaturo_db::Error::Poisoned))?;
                if let Some(existing) = guard
                    .iter_mut()
                    .find(|a| a.subscriber_id == req.subscriber_id && a.product_id == pid)
                {
                    existing.target_price_cents = target;
                    existing.created_at_ms = now_ms;
                } else {
                    if guard.len() >= MAX_PRICE_ALERTS {
                        return Err(Error::Capacity);
                    }
                    guard.push(PriceAlertSubscription {
                        subscriber_id: req.subscriber_id.clone(),
                        product_id: pid.to_owned(),
                        target_price_cents: target,
                        created_at_ms: now_ms,
                    });
                }
                Ok(build_alert_response(
                    &req.action,
                    &req.subscriber_id,
                    &guard,
                    &index,
                    offers,
                ))
            }
            "remover-alerta" => {
                let pid = req.product_id.as_deref().ok_or_else(|| {
                    Error::Invalid("product_id is required for remover-alerta".into())
                })?;
                let mut guard = self
                    .commerce_cache
                    .price_alerts
                    .write()
                    .map_err(|_| Error::Storage(zaturo_db::Error::Poisoned))?;
                guard.retain(|a| !(a.subscriber_id == req.subscriber_id && a.product_id == pid));
                Ok(build_alert_response(
                    &req.action,
                    &req.subscriber_id,
                    &guard,
                    &index,
                    offers,
                ))
            }
            _ => {
                let guard = self
                    .commerce_cache
                    .price_alerts
                    .read()
                    .map_err(|_| Error::Storage(zaturo_db::Error::Poisoned))?;
                Ok(build_alert_response(
                    &req.action,
                    &req.subscriber_id,
                    &guard,
                    &index,
                    offers,
                ))
            }
        }
    }
}

fn build_alert_response(
    action: &str,
    subscriber_id: &str,
    alerts: &[PriceAlertSubscription],
    index: &CompiledCommerceIndex,
    offers: &[SiteCatalogOffer],
) -> PriceAlertResponse {
    let active_alerts: Vec<PriceAlertSubscription> = alerts
        .iter()
        .filter(|a| a.subscriber_id == subscriber_id)
        .cloned()
        .collect();
    let mut triggered = Vec::new();
    for alert in &active_alerts {
        if let Some(&idx) = index.by_product_id.get(&alert.product_id) {
            let offer = &offers[idx];
            if offer.price_brl_cents <= alert.target_price_cents {
                triggered.push(TriggeredPriceAlert {
                    subscriber_id: alert.subscriber_id.clone(),
                    product_id: alert.product_id.clone(),
                    title: offer.title.clone(),
                    target_price_cents: alert.target_price_cents,
                    current_price_cents: offer.price_brl_cents,
                    share_link: offer.official_affiliate_url.clone(),
                });
            }
        }
    }
    PriceAlertResponse {
        action: action.to_owned(),
        subscriber_id: subscriber_id.to_owned(),
        active_alerts,
        triggered,
    }
}
