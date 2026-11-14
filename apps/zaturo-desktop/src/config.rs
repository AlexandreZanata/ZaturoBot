use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopConfig {
    pub hub_url: String,
    pub credentials_file: PathBuf,
    pub target_worker: String,
    pub inbox_root: PathBuf,
    #[serde(default)]
    pub transfer_identity: Option<PathBuf>,
}
impl DesktopConfig {
    pub fn defaults(home: &Path) -> Self {
        Self {
            transfer_identity: Some(home.join(".config/zaturobot/transfer.identity")),
            hub_url: "http://127.0.0.1:17878".into(),
            credentials_file: home.join(".config/zaturobot/app.env"),
            target_worker: "local-worker".into(),
            inbox_root: home.join(".local/share/zaturobot/inbox"),
        }
    }
    pub fn validate(&self) -> Result<()> {
        zaturo_security::validate_endpoint(&self.hub_url)?;
        zaturo_core::validate_identifier(&self.target_worker).map_err(anyhow::Error::msg)?;
        ensure!(
            self.credentials_file.is_absolute() && self.inbox_root.is_absolute(),
            "private client paths must be absolute"
        );
        if let Some(identity) = &self.transfer_identity {
            ensure!(
                identity.is_absolute(),
                "transfer identity path must be absolute"
            );
        }
        Ok(())
    }
    pub fn load(path: &Path, home: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::defaults(home));
        }
        let data = read_private(path, 16384)?;
        let mut config: Self = toml::from_str(&data)
            .map_err(|_| anyhow::anyhow!("invalid private desktop configuration"))?;
        if config.transfer_identity.is_none() {
            config.transfer_identity = Some(home.join(".config/zaturobot/transfer.identity"));
        }
        config.validate()?;
        Ok(config)
    }
    pub fn token(&self) -> Result<String> {
        if let Ok(token) =
            std::env::var("ZATURO_APP_TOKEN").or_else(|_| std::env::var("ZATURO_OWNER_TOKEN"))
        {
            zaturo_security::Credential::new(&token)?;
            return Ok(token);
        }
        let content = read_private(&self.credentials_file, 16384)?;
        let token = content
            .lines()
            .find_map(|line| {
                line.strip_prefix("ZATURO_APP_TOKEN=")
                    .or_else(|| line.strip_prefix("ZATURO_OWNER_TOKEN="))
            })
            .ok_or_else(|| anyhow::anyhow!("owner credential is not configured"))?
            .to_owned();
        zaturo_security::Credential::new(&token)?;
        Ok(token)
    }
    pub fn save(&mut self, path: &Path, new_token: &str) -> Result<()> {
        self.validate()?;
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("configuration directory required"))?;
        private_dir(parent)?;
        if !new_token.is_empty() {
            zaturo_security::Credential::new(new_token)?;
            self.credentials_file = parent.join("desktop.env");
            private_write(
                &self.credentials_file,
                format!("ZATURO_APP_TOKEN={new_token}\n").as_bytes(),
            )?;
        }
        private_write(path, toml::to_string(self)?.as_bytes())
    }
}
fn read_private(path: &Path, limit: u64) -> Result<String> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    ensure!(
        meta.is_file() && meta.permissions().mode() & 0o077 == 0 && meta.len() <= limit,
        "private regular file mode 0600 is required"
    );
    let mut value = String::new();
    Read::by_ref(&mut file)
        .take(limit + 1)
        .read_to_string(&mut value)?;
    ensure!(value.len() as u64 <= limit, "private file exceeds limit");
    Ok(value)
}
fn private_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)?;
    }
    let info = fs::symlink_metadata(path)?;
    ensure!(
        info.is_dir() && !info.file_type().is_symlink() && info.permissions().mode() & 0o077 == 0,
        "private directory mode 0700 is required"
    );
    Ok(())
}
fn private_write(path: &Path, data: &[u8]) -> Result<()> {
    if path.symlink_metadata().is_ok() {
        ensure!(
            path.symlink_metadata()?.is_file()
                && !path.symlink_metadata()?.file_type().is_symlink(),
            "private destination must be a regular file"
        );
    }
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&temporary)?;
    file.write_all(data)?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    fs::File::open(path.parent().expect("private configuration parent"))?.sync_all()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_preserve_owner_credentials_and_reject_unsafe_origins() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = DesktopConfig::defaults(dir.path());
        let path = dir.path().join("private/desktop.toml");
        config
            .save(&path, "synthetic-desktop-owner-token-for-tests")
            .unwrap();
        assert_eq!(
            config.token().unwrap(),
            "synthetic-desktop-owner-token-for-tests"
        );
        let loaded = DesktopConfig::load(&path, dir.path()).unwrap();
        assert_eq!(loaded.credentials_file, config.credentials_file);
        assert_eq!(
            fs::metadata(&loaded.credentials_file)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        config.hub_url = "http://public.example".into();
        assert!(config.validate().is_err());
    }
}
