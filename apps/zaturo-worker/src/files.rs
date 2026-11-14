//! Automatic private inbox delivery is separate from task/model execution.
use crate::config::WorkerConfig;
use anyhow::Result;
use zaturo_transfer::client::DeviceClient;
pub struct InboxJob(tokio::task::JoinHandle<()>);
impl Drop for InboxJob {
    fn drop(&mut self) {
        self.0.abort();
    }
}
pub fn client(config: &WorkerConfig, token: &str) -> Result<DeviceClient> {
    DeviceClient::new(&config.hub_url, token, &config.worker_id)
}
pub fn start_inbox(config: &WorkerConfig, token: &str) -> Result<Option<InboxJob>> {
    let Some(files) = config.files.clone() else {
        return Ok(None);
    };
    let client = client(config, token)?;
    zaturo_transfer::blob::private_directory(&files.cache)?;
    zaturo_transfer::blob::private_directory(&files.inbox)?;
    let public = zaturo_transfer::crypto::public_identity(&files.identity)?;
    Ok(Some(InboxJob(tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let verified = client
                .verify_identity()
                .await
                .is_ok_and(|peer| peer.encryption_recipient == public);
            if !verified {
                tracing::warn!("inbox_identity_unavailable");
                continue;
            }
            match client
                .sync(&files.identity, &files.cache, &files.inbox)
                .await
            {
                Ok(report) => {
                    for path in report.received {
                        tracing::info!(path=%path.display(),"file_received");
                    }
                    if report.failed > 0 {
                        tracing::warn!(count = report.failed, "inbox_items_require_retry");
                    }
                }
                Err(_) => tracing::warn!("inbox_sync_failed"),
            }
        }
    }))))
}
