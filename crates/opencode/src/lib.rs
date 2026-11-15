//! Bounded installed-program execution; no model-supplied executable or command string.
mod artifacts;
mod broker;
mod cancel;
mod gateway;
mod runner;
mod state;

pub use artifacts::inspect_artifacts;
pub use cancel::cancel_owned;
pub use runner::{dispatch, reconcile};
use serde::Deserialize;
use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

pub const MODEL: &str = "opencode-go/deepseek-v4.1-flash";
pub const VERSION: &str = "2.3.0";

/// Render the same restricted profile for a read-only installed-program configuration probe.
pub fn restricted_configuration(
    order: &zaturo_core::OpenCodeWorkOrder,
    origin: &str,
    nonce: &str,
) -> anyhow::Result<serde_json::Value> {
    anyhow::ensure!(
        order.validates(now_ms()),
        "valid bounded work order required"
    );
    let address: std::net::SocketAddr = origin
        .strip_prefix("http://")
        .ok_or_else(|| anyhow::anyhow!("literal loopback origin required"))?
        .parse()?;
    anyhow::ensure!(
        address.ip() == std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
            && address.port() > 0
            && nonce.len() == 64
            && nonce
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "private loopback job origin required"
    );
    Ok(state::job_configuration(order, origin, nonce))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenCodeConfig {
    pub executable: PathBuf,
    pub executable_sha256: String,
    pub state_root: PathBuf,
    pub models_config: PathBuf,
}
impl OpenCodeConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.executable.is_absolute()
                && self.state_root.is_absolute()
                && self.models_config.is_absolute(),
            "absolute reviewed OpenCode paths required"
        );
        anyhow::ensure!(
            self.executable_sha256.len() == 64
                && self
                    .executable_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "invalid installed executable hash"
        );
        Ok(())
    }
    pub fn models(&self) -> anyhow::Result<zaturo_llm::ModelsConfig> {
        let raw = private_file(&self.models_config, 65536)?;
        let models: zaturo_llm::ModelsConfig = toml::from_str(std::str::from_utf8(&raw)?)
            .map_err(|_| anyhow::anyhow!("invalid bounded Personal Go profile"))?;
        models.validate()?;
        anyhow::ensure!(
            models.enabled
                && models.provider == "opencode-go"
                && models.allow_personal_go
                && models.allow_global_processing
                && models.allowed_privacy == [zaturo_llm::PrivacyClass::Personal]
                && models.max_attempts == 1
                && models.budget.max_task_model_calls == 1,
            "one-call Personal Go profile required"
        );
        Ok(models)
    }
}

pub(crate) fn private_file(path: &Path, maximum: u64) -> anyhow::Result<Vec<u8>> {
    anyhow::ensure!(
        path.is_absolute() && !path.ancestors().any(|p| p.is_symlink()),
        "unlinked absolute private file required"
    );
    use std::io::Read;
    let before = std::fs::symlink_metadata(path)?;
    let file = std::fs::File::open(path)?;
    let info = file.metadata()?;
    anyhow::ensure!(
        info.is_file()
            && before.dev() == info.dev()
            && before.ino() == info.ino()
            && info.uid()
                == std::fs::metadata(
                    std::env::var_os("HOME")
                        .ok_or_else(|| anyhow::anyhow!("owner home required"))?
                )?
                .uid()
            && info.permissions().mode() & 0o077 == 0
            && info.len() <= maximum,
        "private bounded owned file required"
    );
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    anyhow::ensure!(
        bytes.len() as u64 <= maximum,
        "private file exceeded its limit"
    );
    Ok(bytes)
}

pub(crate) fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}
