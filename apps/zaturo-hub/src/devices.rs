//! Tokens bind to a fixed device identity; only public encryption keys live on the hub.
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use zaturo_security::Credential;
use zaturo_transfer::Peer;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryFile {
    devices: Vec<DeviceEntry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceEntry {
    id: String,
    token_sha256: String,
    encryption_recipient: String,
}
pub struct DeviceRegistry {
    entries: Vec<(Peer, Credential)>,
}
impl DeviceRegistry {
    pub fn load(path: &Path, max: u32) -> Result<Self> {
        let metadata = fs::symlink_metadata(path)?;
        ensure!(
            metadata.is_file()
                && !metadata.file_type().is_symlink()
                && metadata.permissions().mode() & 0o007 == 0
                && metadata.len() <= 16384,
            "device registry must be a private regular file"
        );
        let file: RegistryFile = toml::from_str(&fs::read_to_string(path)?)
            .map_err(|_| anyhow::anyhow!("invalid device registry; values are not logged"))?;
        ensure!(
            !file.devices.is_empty() && file.devices.len() <= max as usize,
            "registry exceeds device limit"
        );
        let mut entries = Vec::new();
        let mut ids = std::collections::HashSet::new();
        let mut digests = std::collections::HashSet::new();
        let mut recipients = std::collections::HashSet::new();
        for entry in file.devices {
            zaturo_core::validate_identifier(&entry.id).map_err(anyhow::Error::msg)?;
            ensure!(
                ids.insert(entry.id.clone())
                    && digests.insert(entry.token_sha256.clone())
                    && recipients.insert(entry.encryption_recipient.clone())
                    && zaturo_transfer::crypto::valid_recipient(&entry.encryption_recipient),
                "duplicate device identity or invalid encryption recipient"
            );
            entries.push((
                Peer {
                    id: entry.id,
                    encryption_recipient: entry.encryption_recipient,
                },
                Credential::from_sha256_hex(&entry.token_sha256)?,
            ));
        }
        Ok(Self { entries })
    }
    pub fn authenticate(&self, token: &str) -> Option<String> {
        // Check every digest, even when one matches; device count is bounded to eight.
        let mut identity = None;
        for (peer, key) in &self.entries {
            if key.verifies(token) {
                identity = Some(peer.id.clone());
            }
        }
        identity
    }
    pub fn peers(&self) -> Vec<Peer> {
        self.entries.iter().map(|(p, _)| p.clone()).collect()
    }
}
