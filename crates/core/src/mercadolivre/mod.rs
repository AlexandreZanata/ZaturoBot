//! Mercado Livre affiliate product contracts, card/JSON-LD parsers, MinIO media metadata,
//! and static site / Telegram queue projections. Pure validation only; no network or disk I/O.

pub const MAX_ML_BATCH_PRODUCTS: usize = 50;
pub const MAX_ML_SPECS_PER_PRODUCT: usize = 64;
pub const MAX_ML_VARIANTS_PER_PRODUCT: usize = 16;
pub const MAX_ML_IMAGES_PER_PRODUCT: usize = 16;
pub const MAX_ML_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
pub const DEFAULT_ML_FRESHNESS_TTL_MINUTES: u32 = 30;
pub const AFFILIATE_DISCLOSURE_PT_BR: &str =
    "Publicidade · Link de afiliado: podemos receber comissão se você comprar.";

mod catalog;
mod intelligence;
mod model;
mod parse;
mod telegram;
mod whatsapp;

#[cfg(test)]
mod tests;

pub use catalog::*;
pub use intelligence::*;
pub use model::*;
pub use parse::*;
pub use telegram::*;
pub use whatsapp::*;
