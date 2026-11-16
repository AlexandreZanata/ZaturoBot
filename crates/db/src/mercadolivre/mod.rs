//! Relational persistence for ingested Mercado Livre products, specs, variants, MinIO images,
//! price observations, static site catalog snapshots, and Telegram delivery queues.

mod catalog;
mod ingest;
mod telegram_queue;

#[cfg(test)]
mod tests;

pub(super) const THIRTY_DAYS_MS: i64 = 30 * 86_400_000;

#[derive(Default)]
pub(super) struct ThirtyDayDedupState {
    cutoff_ms: Option<i64>,
    last_kept: Option<(i64, i64)>,
    kept_count: usize,
}

impl ThirtyDayDedupState {
    pub(super) fn accept(&mut self, price_cents: i64, observed_at_ms: i64) -> bool {
        if self.kept_count >= 30 {
            return false;
        }
        let cutoff = *self
            .cutoff_ms
            .get_or_insert_with(|| observed_at_ms.saturating_sub(THIRTY_DAYS_MS));
        if observed_at_ms < cutoff {
            return false;
        }
        let day_utc = observed_at_ms.div_euclid(86_400_000);
        if self.last_kept == Some((price_cents, day_utc)) {
            return false;
        }
        self.last_kept = Some((price_cents, day_utc));
        self.kept_count += 1;
        true
    }
}
