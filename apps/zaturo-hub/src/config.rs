use anyhow::{Result, ensure};
use serde::Deserialize;
use std::{net::SocketAddr, path::PathBuf};
use zaturo_core::{Limits, TransferLimits};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubConfig {
    pub bind: SocketAddr,
    pub client_url: String,
    pub database: PathBuf,
    pub worker_id: String,
    pub max_connections: usize,
    pub max_queued_tasks: u32,
    pub max_page_size: u32,
    pub lease_seconds: u64,
    pub max_workers: u32,
    #[serde(default)]
    pub devices_file: Option<PathBuf>,
    #[serde(default)]
    pub apps_file: Option<PathBuf>,
    #[serde(default)]
    pub transfer_root: Option<PathBuf>,
    #[serde(default)]
    pub transfer_limits: TransferLimits,
}
impl HubConfig {
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let value: Self = toml::from_str(&std::fs::read_to_string(path)?).map_err(|_| {
            anyhow::anyhow!("invalid hub configuration; source values are not logged")
        })?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            zaturo_security::private_bind(self.bind.ip()),
            "bind a specific loopback or private VPN address"
        );
        ensure!(
            (1..=16).contains(&self.max_connections),
            "max_connections must be 1..16"
        );
        ensure!(
            (1..=4096).contains(&self.max_queued_tasks),
            "queue limit must be 1..4096"
        );
        ensure!(
            (1..=100).contains(&self.max_page_size),
            "page limit must be 1..100"
        );
        ensure!(
            (10..=300).contains(&self.lease_seconds),
            "lease must be 10..300 seconds"
        );
        ensure!(
            (1..=8).contains(&self.max_workers),
            "max_workers must be 1..8"
        );
        zaturo_core::validate_identifier(&self.worker_id).map_err(anyhow::Error::msg)?;
        ensure!(
            !self.database.as_os_str().is_empty(),
            "database path is required"
        );
        ensure!(self.transfer_limits.validates(), "invalid transfer limits");
        ensure!(
            self.devices_file.is_some() == self.transfer_root.is_some(),
            "device registry and transfer root must be configured together"
        );
        ensure!(
            self.devices_file.is_none() || self.max_page_size >= 20,
            "device file API requires max_page_size of at least 20"
        );
        validate_url(&self.client_url)?;
        Ok(())
    }
    pub fn limits(&self) -> Limits {
        Limits {
            max_queued_tasks: self.max_queued_tasks,
            max_page_size: self.max_page_size,
            lease_seconds: self.lease_seconds,
            max_workers: self.max_workers,
        }
    }
}

pub fn validate_url(value: &str) -> Result<()> {
    zaturo_security::validate_endpoint(value)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn examples_are_strict_and_resource_limits_are_validated() {
        let example = include_str!("../../../config/hub.example.toml");
        let mut c: HubConfig = toml::from_str(example).unwrap();
        c.validate().unwrap();
        c.max_queued_tasks = 0;
        assert!(c.validate().is_err());
        assert!(toml::from_str::<HubConfig>(&format!("{example}\nunknown = true")).is_err());
        assert!(validate_url("http://example.org").is_err());
        assert!(validate_url("https://user:password@example.org").is_err());
    }
}
