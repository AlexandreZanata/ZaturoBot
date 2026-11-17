//! Transactional Mercado Livre product ingestion, static-site catalog projection,
//! Cloobe-inspired in-memory inverted-index hot caching, faceted search, telemetry, price alerts,
//! and Telegram delivery queue orchestration.

mod alerts;
mod cache;
mod ingest_and_telegram;
mod search;

#[cfg(test)]
mod tests;

pub(crate) use cache::CommerceHotCache;
pub use cache::CompiledCommerceIndex;
