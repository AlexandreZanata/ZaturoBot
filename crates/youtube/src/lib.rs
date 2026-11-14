//! Bounded adapter for the existing YoutubeBot Python service, not a video implementation.
mod process;
mod response;

pub use response::{JobState, JobStatus};
use serde::Deserialize;
use std::path::PathBuf;
use zaturo_core::{ServiceRequest, ServiceResult};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YoutubeConfig {
    pub python: PathBuf,
    pub project_root: PathBuf,
    #[serde(default)]
    pub allow_schedule: bool,
}
impl YoutubeConfig {
    pub fn validate(&self) -> Result<(), Error> {
        if !self.python.is_absolute() || !self.project_root.is_absolute() {
            return Err(Error::InvalidConfiguration);
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("YoutubeBot requires a trusted absolute interpreter and project root")]
    InvalidConfiguration,
    #[error("invalid typed YoutubeBot request")]
    InvalidRequest,
    #[error("scheduling is not authorized by the local worker profile")]
    PolicyDenied,
    #[error("YoutubeBot service is unavailable")]
    Unavailable,
    #[error("YoutubeBot service exceeded its deadline")]
    Timeout,
    #[error("YoutubeBot service output exceeded its limit")]
    OutputLimit,
    #[error("YoutubeBot service returned an invalid bounded result")]
    InvalidOutput,
    #[error("YoutubeBot service rejected the operation; raw errors are not logged")]
    Rejected,
    #[error("YoutubeBot reports failed work; submission was not replayed")]
    JobFailed,
    #[error("YoutubeBot operation requires reconciliation; submission was not replayed")]
    ReconciliationRequired,
}
#[derive(Clone)]
pub struct YoutubeClient {
    config: YoutubeConfig,
    home: PathBuf,
    project_root: PathBuf,
    timeout: std::time::Duration,
}
impl YoutubeClient {
    pub fn new(config: YoutubeConfig) -> Result<Self, Error> {
        Self::with_timeout(config, std::time::Duration::from_secs(20))
    }
    pub fn with_timeout(
        config: YoutubeConfig,
        timeout: std::time::Duration,
    ) -> Result<Self, Error> {
        config.validate()?;
        if timeout.is_zero() || timeout > std::time::Duration::from_secs(20) {
            return Err(Error::InvalidConfiguration);
        }
        let canonical = std::fs::canonicalize(&config.python).map_err(|_| Error::Unavailable)?;
        let metadata = std::fs::metadata(canonical).map_err(|_| Error::Unavailable)?;
        use std::os::unix::fs::PermissionsExt;
        if !metadata.is_file()
            || metadata.permissions().mode() & 0o111 == 0
            || metadata.permissions().mode() & 0o022 != 0
        {
            return Err(Error::InvalidConfiguration);
        }
        let project_root =
            std::fs::canonicalize(&config.project_root).map_err(|_| Error::Unavailable)?;
        if !project_root.is_dir() {
            return Err(Error::InvalidConfiguration);
        }
        let service = std::fs::canonicalize(project_root.join("tools/ytbot/service.py"))
            .map_err(|_| Error::Unavailable)?;
        if !service.starts_with(&project_root) || !service.is_file() {
            return Err(Error::InvalidConfiguration);
        }
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .ok_or(Error::InvalidConfiguration)?;
        // Preserve the configured interpreter symlink so Python virtual environments still work.
        Ok(Self {
            config,
            home,
            project_root,
            timeout,
        })
    }
    pub fn config(&self) -> &YoutubeConfig {
        &self.config
    }
    pub async fn execute(
        &self,
        operation: &str,
        request: &ServiceRequest,
    ) -> Result<ServiceResult, Error> {
        canonical_operation(operation)?;
        if !request.validates() {
            return Err(Error::InvalidRequest);
        }
        let raw = self.invoke(&self.arguments(operation, request)?).await?;
        response::map_result(operation, request, raw)
    }
    /// Status is a read-only observation even when the underlying job has failed or is uncertain.
    pub async fn status(&self, operation: &str) -> Result<JobStatus, Error> {
        canonical_operation(operation)?;
        let raw = self
            .invoke(&["status".into(), "--operation-id".into(), operation.into()])
            .await?;
        let job = response::job_status(raw)?;
        if job.operation_id != operation {
            return Err(Error::InvalidOutput);
        }
        Ok(job)
    }
    /// An unrecorded submission can only query its existing operation, never submit a second time.
    pub async fn reconcile(
        &self,
        operation: &str,
        request: &ServiceRequest,
    ) -> Result<ServiceResult, Error> {
        if !request.validates() {
            return Err(Error::InvalidRequest);
        }
        let ServiceRequest::YoutubeSubmit {
            channel,
            account,
            count,
            ..
        } = request
        else {
            return Err(Error::InvalidRequest);
        };
        let job = self
            .status(operation)
            .await
            .map_err(|_| Error::ReconciliationRequired)?;
        if job.operation_id != operation
            || job.channel != *channel
            || job.account != *account
            || job.count != *count
        {
            return Err(Error::ReconciliationRequired);
        }
        response::submission(operation, request, job)
    }
    fn arguments(&self, operation: &str, request: &ServiceRequest) -> Result<Vec<String>, Error> {
        match request {
            ServiceRequest::YoutubeProfiles => Ok(vec!["profiles".into()]),
            ServiceRequest::YoutubeRuns { channel, limit } => Ok(vec![
                "runs".into(),
                "--channel".into(),
                channel.clone(),
                "--limit".into(),
                limit.to_string(),
            ]),
            ServiceRequest::YoutubeSubmit {
                channel,
                account,
                count,
                approved_schedule,
            } => {
                if !self.config.allow_schedule || !approved_schedule {
                    return Err(Error::PolicyDenied);
                }
                Ok(vec![
                    "submit".into(),
                    "--operation-id".into(),
                    operation.into(),
                    "--channel".into(),
                    channel.clone(),
                    "--account".into(),
                    account.clone(),
                    "--count".into(),
                    count.to_string(),
                    "--approved-schedule".into(),
                ])
            }
            ServiceRequest::YoutubeStatus { operation_id } => {
                canonical_operation(operation_id)?;
                Ok(vec![
                    "status".into(),
                    "--operation-id".into(),
                    operation_id.clone(),
                ])
            }
            _ => Err(Error::InvalidRequest),
        }
    }
}
fn canonical_operation(value: &str) -> Result<(), Error> {
    if uuid::Uuid::parse_str(value).is_ok_and(|u| u.to_string() == value) {
        Ok(())
    } else {
        Err(Error::InvalidRequest)
    }
}
fn text(value: &str, max: usize) -> (String, bool) {
    let cleaned: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let cleaned = cleaned.trim();
    let mut end = cleaned.len().min(max);
    while !cleaned.is_char_boundary(end) {
        end -= 1;
    }
    (
        cleaned[..end].into(),
        end < cleaned.len() || cleaned != value,
    )
}
