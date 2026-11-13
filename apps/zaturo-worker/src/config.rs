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
                && self.capabilities.len() <= 2
                && self
                    .capabilities
                    .iter()
                    .all(|c| matches!(c.as_str(), "diagnostic.v1" | "agent.diagnostic.v1"))
                && self
                    .capabilities
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    == self.capabilities.len(),
            "one task and only registered diagnostic capabilities are allowed"
        );
        ensure!(
            !self.capabilities.iter().any(|c| c == "agent.diagnostic.v1")
                || self.models_config.is_some(),
            "assisted capability requires a model configuration"
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
