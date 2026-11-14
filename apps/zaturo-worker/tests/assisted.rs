//! Actual transport and durable budgets with synthetic model responses only.
use std::{path::Path, sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use zaturo_core::{ModelCallState, ModelUsage, NewTask, Task, TaskState};
use zaturo_db::WorkerJournal;
use zaturo_hub::{AppState, config::HubConfig};
use zaturo_llm::{
    DEEPSEEK_MODEL, ModelResponse, ModelsConfig, Router,
    fake::{FakeClient, FakeReply},
};
use zaturo_worker::config::WorkerConfig;

const OWNER: &str = "synthetic-owner-token-for-assisted-tests-only";
const DEVICE: &str = "synthetic-device-token-for-assisted-tests-only";
const TOOL: &str = r#"{"action":"tool","tool":"diagnostic.v1"}"#;
const FINISH: &str = r#"{"action":"finish","summary":"The typed worker diagnostic passed; no project files were inspected."}"#;

struct Hub {
    state: AppState,
    base: String,
    server: JoinHandle<()>,
}
impl Drop for Hub {
    fn drop(&mut self) {
        self.server.abort();
    }
}
async fn hub(path: &Path) -> Hub {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let config = HubConfig {
        apps_file: None,
        devices_file: None,
        transfer_root: None,
        transfer_limits: Default::default(),
        bind: address,
        client_url: format!("http://{address}"),
        database: path.join("hub.db"),
        worker_id: "local-worker".into(),
        max_connections: 2,
        max_queued_tasks: 8,
        max_page_size: 100,
        lease_seconds: 30,
        max_workers: 1,
    };
    let state = AppState::new(config, OWNER, DEVICE).unwrap();
    let routes = zaturo_hub::router(state.clone());
    let listener = zaturo_hub::listener::BoundedListener::new(listener, 16).unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, routes).await.unwrap();
    });
    Hub {
        state,
        base: format!("http://{address}"),
        server,
    }
}
fn worker(path: &Path, base: &str) -> WorkerConfig {
    WorkerConfig {
        files: None,
        youtube: None,
        android: None,
        hub_url: base.into(),
        worker_id: "local-worker".into(),
        database: path.join("worker.db"),
        workspace_root: path.join("workspaces"),
        artifact_root: path.join("artifacts"),
        capabilities: vec!["diagnostic.v1".into(), "agent.diagnostic.v1".into()],
        max_concurrent_tasks: 1,
        heartbeat_seconds: 1,
        reconnect_max_seconds: 2,
        models_config: Some(path.join("injected-models.toml")),
    }
}
fn model_response(content: &str) -> ModelResponse {
    ModelResponse {
        model: DEEPSEEK_MODEL.into(),
        content: content.into(),
        usage: ModelUsage {
            input_tokens: 10,
            output_tokens: 20,
            cost_microusd: Some(1),
        },
        finish_reason: "stop".into(),
        duration_ms: 1,
        model_calls: 1,
        usage_complete: true,
    }
}
fn models(replies: Vec<FakeReply>) -> (Arc<Router>, Arc<FakeClient>) {
    let mut config: ModelsConfig =
        toml::from_str(include_str!("../../../config/models.example.toml")).unwrap();
    config.enabled = true;
    config.allow_global_processing = true;
    config.timeout_seconds = 60;
    config.max_attempts = 1;
    let client = Arc::new(FakeClient::new(replies));
    (
        Arc::new(Router::new(config, client.clone()).unwrap()),
        client,
    )
}
fn launch(
    config: WorkerConfig,
    journal: Arc<WorkerJournal>,
    router: Arc<Router>,
) -> JoinHandle<anyhow::Result<()>> {
    tokio::spawn(async move {
        zaturo_worker::connect_once_with_router(&config, DEVICE, journal, Some(router)).await
    })
}
async fn submit(hub: &Hub, key: &str, assisted: bool) -> Task {
    let request = if assisted {
        NewTask::assisted_diagnostic("local-worker", key)
    } else {
        NewTask::diagnostic("local-worker", key)
    };
    reqwest::Client::new()
        .post(format!("{}/v1/tasks", hub.base))
        .bearer_auth(OWNER)
        .json(&request)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}
async fn wait_state(hub: &Hub, id: &str, state: TaskState) -> Task {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let task = hub.state.engine.get(id).unwrap();
            if task.state == state {
                return task;
            }
            assert!(
                !task.state.is_terminal(),
                "unexpected terminal state: {task:?}"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("task did not reach the expected state")
}
async fn model_called(client: &FakeClient) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while client.calls() == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
async fn disconnected(hub: &Hub) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !hub.state.connected.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn assisted_transport_completes_with_two_model_calls_and_durable_tool_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let hub = hub(dir.path()).await;
    let config = worker(dir.path(), &hub.base);
    let journal = Arc::new(WorkerJournal::open(&config.database).unwrap());
    let (router, client) = models(vec![
        FakeReply::Answer(model_response(TOOL)),
        FakeReply::Answer(model_response(FINISH)),
    ]);
    let running = launch(config, journal.clone(), router);
    let task = submit(&hub, "assisted-complete", true).await;
    let done = wait_state(&hub, &task.id, TaskState::Completed).await;
    let evidence = done.orchestration.unwrap();
    assert_eq!(client.calls(), 2);
    assert_eq!(evidence.steps, 2);
    assert_eq!(evidence.model_calls, 2);
    assert_eq!(evidence.usage.input_tokens, 20);
    assert_eq!(evidence.usage.output_tokens, 40);
    assert_eq!(evidence.usage.cost_microusd, Some(2));
    assert!(evidence.usage_complete);
    assert_eq!(journal.result(&task.operation_id).unwrap(), done.result);
    assert_eq!(
        journal
            .begin_assisted(&task.operation_id, &task.id, zaturo_worker::now_ms())
            .unwrap()
            .unwrap()
            .orchestration,
        evidence
    );
    assert!(
        journal
            .model_call_usage(&format!("{}:1:1", task.operation_id))
            .unwrap()
            .is_some()
    );
    assert!(
        journal
            .model_call_usage(&format!("{}:2:1", task.operation_id))
            .unwrap()
            .is_some()
    );
    running.abort();
}

#[tokio::test]
async fn malicious_model_proposal_fails_without_dispatch_or_untrusted_failure_text() {
    let dir = tempfile::tempdir().unwrap();
    let hub = hub(dir.path()).await;
    let config = worker(dir.path(), &hub.base);
    let journal = Arc::new(WorkerJournal::open(&config.database).unwrap());
    let (router, client) = models(vec![FakeReply::Answer(model_response(
        r#"{"action":"tool","tool":"terminal.execute"}"#,
    ))]);
    let running = launch(config, journal.clone(), router);
    let task = submit(&hub, "malicious", true).await;
    let failed = wait_state(&hub, &task.id, TaskState::Failed).await;
    assert_eq!(failed.failure.as_deref(), Some("policy_denied"));
    assert_eq!(client.calls(), 1);
    assert!(failed.result.is_none() && failed.orchestration.is_none());
    assert!(journal.result(&task.operation_id).unwrap().is_none());
    let next = submit(&hub, "plain-after-denial", false).await;
    wait_state(&hub, &next.id, TaskState::Completed).await;
    assert_eq!(client.calls(), 1);
    running.abort();
}

#[tokio::test]
async fn cancellation_stops_a_delayed_model_while_heartbeats_remain_responsive() {
    let dir = tempfile::tempdir().unwrap();
    let hub = hub(dir.path()).await;
    let config = worker(dir.path(), &hub.base);
    let journal = Arc::new(WorkerJournal::open(&config.database).unwrap());
    let (router, client) = models(vec![FakeReply::Delay(
        Duration::from_secs(30),
        model_response(TOOL),
    )]);
    let running = launch(config, journal.clone(), router.clone());
    let task = submit(&hub, "cancel-delayed", true).await;
    model_called(&client).await;
    let initial = hub.state.engine.workers(1, 0).unwrap()[0].last_seen_at;
    tokio::time::timeout(Duration::from_secs(3), async {
        while hub.state.engine.workers(1, 0).unwrap()[0].last_seen_at <= initial {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("model execution blocked worker heartbeats");
    assert_eq!(
        hub.state.engine.get(&task.id).unwrap().state,
        TaskState::Running
    );
    let cancelled: Task = reqwest::Client::new()
        .post(format!("{}/v1/tasks/{}/cancel", hub.base, task.id))
        .bearer_auth(OWNER)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cancelled.state, TaskState::Cancelled);
    let request_id = format!("{}:1:1", task.operation_id);
    tokio::time::timeout(Duration::from_secs(3), async {
        while journal.model_call_state(&request_id).unwrap() != Some(ModelCallState::Uncertain) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("cancellation did not preserve the in-flight call as uncertain");
    let next = submit(&hub, "plain-after-cancel", false).await;
    wait_state(&hub, &next.id, TaskState::Completed).await;
    assert_eq!(client.calls(), 1);
    assert!(journal.result(&task.operation_id).unwrap().is_none());
    assert_eq!(
        hub.state.engine.get(&task.id).unwrap().state,
        TaskState::Cancelled
    );
    running.abort();
}

#[tokio::test]
async fn abrupt_worker_restart_never_resends_a_reserved_model_request() {
    let dir = tempfile::tempdir().unwrap();
    let hub = hub(dir.path()).await;
    let config = worker(dir.path(), &hub.base);
    let journal = Arc::new(WorkerJournal::open(&config.database).unwrap());
    let (router, first_client) = models(vec![FakeReply::Delay(
        Duration::from_secs(30),
        model_response(TOOL),
    )]);
    let first = launch(config.clone(), journal.clone(), router);
    let task = submit(&hub, "restart-in-flight", true).await;
    model_called(&first_client).await;
    first.abort();
    let _ = first.await;
    disconnected(&hub).await;
    drop(journal);
    let journal = Arc::new(WorkerJournal::open(&config.database).unwrap());
    let (router, second_client) = models(vec![
        FakeReply::Answer(model_response(TOOL)),
        FakeReply::Answer(model_response(FINISH)),
    ]);
    let second = launch(config, journal.clone(), router);
    let failed = wait_state(&hub, &task.id, TaskState::Failed).await;
    assert_eq!(failed.failure.as_deref(), Some("reconciliation_required"));
    assert_eq!(first_client.calls(), 1);
    assert_eq!(second_client.calls(), 0);
    assert!(journal.result(&task.operation_id).unwrap().is_none());
    assert!(
        journal
            .model_call_usage(&format!("{}:1:1", task.operation_id))
            .unwrap()
            .is_none()
    );
    second.abort();
}
