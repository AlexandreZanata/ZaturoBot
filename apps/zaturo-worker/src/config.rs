use anyhow::{Result, ensure};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfig {
    pub hub_url: String,
    pub worker_id: String,
    pub database: PathBuf,
    pub workspace_root: PathBuf,
    pub artifact_root: PathBuf,
    pub capabilities: Vec<String>,
    pub max_concurrent_tasks: usize,
    pub heartbeat_seconds: u64,
    pub reconnect_max_seconds: u64,
    #[serde(default)]
    pub models_config: Option<PathBuf>,
    #[serde(default)]
    pub files: Option<FilesConfig>,
    #[serde(default)]
    pub youtube: Option<zaturo_youtube::YoutubeConfig>,
    #[serde(default)]
    pub android: Option<AndroidConfig>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilesConfig {
    pub identity: PathBuf,
    pub cache: PathBuf,
    pub inbox: PathBuf,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AndroidConfig {
    pub executable: PathBuf,
    #[serde(default)]
    pub devices: Vec<zaturo_core::RegisteredAndroidDevice>,
    #[serde(default)]
    pub allow_sensitive_views: bool,
}
impl WorkerConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let value: Self = toml::from_str(&std::fs::read_to_string(path)?).map_err(|_| {
            anyhow::anyhow!("invalid worker configuration; source values are not logged")
        })?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        zaturo_security::validate_endpoint(&self.hub_url)?;
        zaturo_core::validate_identifier(&self.worker_id).map_err(anyhow::Error::msg)?;
        ensure!(
            self.max_concurrent_tasks == 1
                && self.capabilities.contains(&"diagnostic.v1".into())
                && self.capabilities.len() <= 6
                && self.capabilities.iter().all(|c| matches!(
                    c.as_str(),
                    "diagnostic.v1"
                        | "agent.diagnostic.v1"
                        | "youtube.read.v1"
                        | "youtube.submit.v1"
                        | "android.devices.v1"
                        | "android.private.v1"
                ))
                && self
                    .capabilities
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    == self.capabilities.len(),
            "one task and only registered diagnostic/service capabilities are allowed"
        );
        ensure!(
            !self.capabilities.iter().any(|c| c == "agent.diagnostic.v1")
                || self.models_config.is_some(),
            "assisted capability requires a model configuration"
        );
        if let Some(youtube) = &self.youtube {
            youtube.validate()?;
        }
        ensure!(
            !self.capabilities.iter().any(|c| c.starts_with("youtube.")) || self.youtube.is_some(),
            "YouTube capabilities require a configured local adapter"
        );
        ensure!(
            !self.capabilities.iter().any(|c| c == "youtube.submit.v1")
                || self.youtube.as_ref().is_some_and(|y| y.allow_schedule),
            "YouTube submission requires a local scheduling grant"
        );
        ensure!(
            !self.capabilities.iter().any(|c| c.starts_with("android.")) || self.android.is_some(),
            "Android capabilities require a configured local adapter"
        );
        if let Some(android) = &self.android {
            ensure!(
                android.executable.is_absolute() && android.devices.len() <= 8,
                "invalid Android configuration"
            );
            if !android.devices.is_empty() {
                zaturo_android::SessionManager::new(android.devices.clone())?;
            }
            for device in &android.devices {
                device.validate().map_err(anyhow::Error::msg)?;
            }
        }
        ensure!(
            !self.capabilities.iter().any(|c| c == "android.private.v1")
                || self
                    .android
                    .as_ref()
                    .is_some_and(|a| a.allow_sensitive_views && !a.devices.is_empty()),
            "sensitive Android views require explicit registered devices and a local grant"
        );
        ensure!(
            (1..=5).contains(&self.heartbeat_seconds),
            "heartbeat must be 1..5 seconds"
        );
        ensure!(
            (1..=60).contains(&self.reconnect_max_seconds),
            "reconnect maximum must be 1..60 seconds"
        );
        for path in [&self.database, &self.workspace_root, &self.artifact_root] {
            ensure!(!path.as_os_str().is_empty(), "local paths are required");
        }
        if let Some(files) = &self.files {
            for path in [&files.identity, &files.cache, &files.inbox] {
                ensure!(path.is_absolute(), "file transfer paths must be absolute");
            }
            ensure!(
                files.cache != files.inbox,
                "cache and inbox must be separate"
            );
        }
        Ok(())
    }
    pub fn websocket_url(&self) -> Result<String> {
        let mut url = reqwest::Url::parse(&self.hub_url)?;
        let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
        url.set_scheme(scheme)
            .map_err(|_| anyhow::anyhow!("invalid hub scheme"))?;
        url.set_path("/v1/worker");
        Ok(url.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn examples_parse_and_capabilities_are_narrow() {
        let example = include_str!("../../../config/worker.example.toml");
        let mut config: WorkerConfig = toml::from_str(example).unwrap();
        config.validate().unwrap();
        assert_eq!(
            config.websocket_url().unwrap(),
            "ws://127.0.0.1:7878/v1/worker"
        );
        config.capabilities.push("shell".into());
        assert!(config.validate().is_err());
        assert!(toml::from_str::<WorkerConfig>(&format!("{example}\nunknown=true")).is_err());
        config.capabilities.pop();
        config.hub_url = "http://example.com".into();
        assert!(config.validate().is_err());
    }
}
