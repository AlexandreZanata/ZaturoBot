//! Independently revocable application credentials; no worker or admin key sharing.
use anyhow::{Result, ensure};
use serde::Deserialize;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use zaturo_security::Credential;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    clients: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    token_sha256: String,
    can_submit: bool,
    #[serde(default)]
    viewer_device: Option<String>,
}
pub struct AppRegistry(Vec<(String, Credential, bool, Option<String>)>);
impl AppRegistry {
    pub fn load(path: &Path) -> Result<Self> {
        let meta = fs::symlink_metadata(path)?;
        ensure!(
            meta.is_file()
                && !meta.file_type().is_symlink()
                && meta.permissions().mode() & 0o007 == 0
                && meta.len() <= 16384,
            "application registry must be private"
        );
        let file: File = toml::from_str(&fs::read_to_string(path)?)
            .map_err(|_| anyhow::anyhow!("invalid application registry; values are not logged"))?;
        ensure!(
            !file.clients.is_empty() && file.clients.len() <= 8,
            "application limit is eight"
        );
        let mut ids = std::collections::HashSet::new();
        let mut keys = std::collections::HashSet::new();
        let mut entries = Vec::new();
        for entry in file.clients {
            zaturo_core::validate_identifier(&entry.id).map_err(anyhow::Error::msg)?;
            ensure!(
                ids.insert(entry.id.clone()) && keys.insert(entry.token_sha256.clone()),
                "duplicate application identity"
            );
            entries.push((
                entry.id,
                Credential::from_sha256_hex(&entry.token_sha256)?,
                entry.can_submit,
                entry.viewer_device,
            ));
        }
        Ok(Self(entries))
    }
    pub fn permits(&self, token: &str, write: bool) -> bool {
        let mut permitted = false;
        for (_, key, submit, _) in &self.0 {
            permitted |= key.verifies(token) && (!write || *submit);
        }
        permitted
    }
}

impl AppRegistry {
    pub fn viewer(&self, token: &str) -> Option<String> {
        let mut viewer = None;
        for (_, key, _, device) in &self.0 {
            if key.verifies(token) {
                viewer = device.clone();
            }
        }
        viewer
    }
}
