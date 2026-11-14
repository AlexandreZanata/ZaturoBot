//! Observe durable Youtube jobs and publish real transitions independently of task submission.
use crate::{config::WorkerConfig, now_ms};
use futures_util::{StreamExt, stream};
use std::{sync::Arc, time::Duration};
use zaturo_core::{FeedItem, ServiceActivity, ServiceRequest};
use zaturo_db::WorkerJournal;
pub struct ServiceJobs(tokio::task::JoinHandle<()>);
impl Drop for ServiceJobs {
    fn drop(&mut self) {
        self.0.abort();
    }
}
pub fn start(
    config: &WorkerConfig,
    token: &str,
    journal: Arc<WorkerJournal>,
) -> anyhow::Result<Option<ServiceJobs>> {
    let Some(youtube) = config.youtube.clone() else {
        return Ok(None);
    };
    let adapter = zaturo_youtube::YoutubeClient::new(youtube)?;
    let client = crate::files::client(config, token)?;
    Ok(Some(ServiceJobs(tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let local = journal.clone();
            let jobs =
                match tokio::task::spawn_blocking(move || local.pending_service_jobs(20)).await {
                    Ok(Ok(jobs)) => jobs,
                    _ => {
                        tracing::warn!("service_job_journal_unavailable");
                        continue;
                    }
                };
            stream::iter(jobs)
                .map(|job| {
                    let journal = journal.clone();
                    let adapter = &adapter;
                    let client = &client;
                    async move {
                        let operation = job.operation_id;
                        let ServiceRequest::YoutubeSubmit {
                            channel,
                            account,
                            count,
                            ..
                        } = job.request
                        else {
                            return;
                        };
                        let local = journal.clone();
                        let id = operation.clone();
                        if !matches!(
                            tokio::task::spawn_blocking(move || {
                                local.service_polled(&id, now_ms())
                            })
                            .await,
                            Ok(Ok(()))
                        ) {
                            tracing::warn!("service_job_poll_journal_unavailable");
                            return;
                        }
                        let status = match tokio::time::timeout(
                            Duration::from_secs(5),
                            adapter.status(&operation),
                        )
                        .await
                        {
                            Ok(Ok(status)) => status,
                            _ => {
                                tracing::warn!("youtube_job_status_unavailable");
                                return;
                            }
                        };
                        if status.channel != channel
                            || status.account != account
                            || status.count != count
                        {
                            tracing::warn!("youtube_job_status_binding_mismatch");
                            return;
                        }
                        let state = status.state.as_str();
                        if job.last_notified_state.as_deref() == Some(state) {
                            return;
                        }
                        let update = ServiceActivity {
                            operation_id: operation.clone(),
                            state: state.into(),
                            summary: status.summary,
                        };
                        if !matches!(
                            tokio::time::timeout(
                                Duration::from_secs(5),
                                client.post::<_, FeedItem>("/v1/device/activity", &update),
                            )
                            .await,
                            Ok(Ok(_))
                        ) {
                            tracing::warn!("service_job_notice_deferred");
                            return;
                        }
                        let local = journal.clone();
                        let state = state.to_owned();
                        let _ = tokio::task::spawn_blocking(move || {
                            local.service_notified(&operation, &state, now_ms())
                        })
                        .await;
                    }
                })
                .buffer_unordered(2)
                .collect::<Vec<_>>()
                .await;
        }
    }))))
}
