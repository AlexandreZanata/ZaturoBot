//! Bounded, end-to-end encrypted file transport. No shell or model interaction.
pub mod blob;
pub mod client;
pub mod crypto;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Peer {
    pub id: String,
    pub encryption_recipient: String,
}
