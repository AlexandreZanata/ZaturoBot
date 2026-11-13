//! Real HTTP, WebSocket, SQLite and worker transport with synthetic credentials only.
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use zaturo_core::{NewTask, Task, TaskEvent, TaskState};
use zaturo_db::WorkerJournal;
use zaturo_hub::{AppState, config::HubConfig};
use zaturo_worker::config::WorkerConfig;

const OWNER: &str = "synthetic-owner-token-for-local-tests-only";
const DEVICE: &str = "synthetic-device-token-for-local-tests-only";

async fn start_hub(path: &std::path::Path) -> (AppState, String, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let config = HubConfig {
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
    let router = zaturo_hub::router(state.clone());
    let listener = zaturo_hub::listener::BoundedListener::new(listener, 32).unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (state, format!("http://{address}"), server)
}

fn worker_config(path: &std::path::Path, url: String) -> WorkerConfig {
    WorkerConfig {
        hub_url: url,
        worker_id: "local-worker".into(),
        database: path.join("worker.db"),
        workspace_root: path.join("workspaces"),
        artifact_root: path.join("artifacts"),
        capabilities: vec!["diagnostic.v1".into()],
        max_concurrent_tasks: 1,
        heartbeat_seconds: 1,
        reconnect_max_seconds: 2,
    }
}

async fn submit(client: &reqwest::Client, base: &str, key: &str) -> Task {
    client
        .post(format!("{base}/v1/tasks"))
        .bearer_auth(OWNER)
        .json(&NewTask::diagnostic("local-worker", key))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap()
}
async fn completed(client: &reqwest::Client, base: &str, id: &str) -> Task {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let task: Task = client
                .get(format!("{base}/v1/tasks/{id}"))
                .bearer_auth(OWNER)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .await
                .unwrap();
            if task.state == TaskState::Completed {
                return task;
            }
            assert!(
                !task.state.is_terminal(),
                "task unexpectedly terminated: {task:?}"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("worker did not complete task")
}

#[tokio::test]
async fn real_worker_authenticated_execution_history_and_reconnect() {
    let dir = tempfile::tempdir().unwrap();
    let (state, base, server) = start_hub(dir.path()).await;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    assert_eq!(
        client
            .get(format!("{base}/v1/tasks"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        client
            .get(format!("{base}/v1/tasks"))
            .bearer_auth(DEVICE)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    let task = submit(&client, &base, "first-task").await;
    let same = submit(&client, &base, "first-task").await;
    assert_eq!(task.id, same.id);
    let config = worker_config(dir.path(), base.clone());
    let journal = Arc::new(WorkerJournal::open(&config.database).unwrap());
    let first_worker = tokio::spawn({
        let config = config.clone();
        let journal = journal.clone();
        async move { zaturo_worker::connect_once(&config, DEVICE, journal).await }
    });
    let done = completed(&client, &base, &task.id).await;
    assert_eq!(done.attempts, 1);
    assert!(done.result.as_ref().unwrap().validates(&done.operation_id));
    let events: Vec<TaskEvent> = client
        .get(format!("{base}/v1/tasks/{}/events", task.id))
        .bearer_auth(OWNER)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        events.iter().map(|e| e.state).collect::<Vec<_>>(),
        vec![
            TaskState::Created,
            TaskState::Queued,
            TaskState::Assigned,
            TaskState::Running,
            TaskState::Completed
        ]
    );
    first_worker.abort();
    let _ = first_worker.await;
    tokio::time::timeout(Duration::from_secs(3), async {
        while !state.connected.lock().unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let next = submit(&client, &base, "after-reconnect").await;
    drop(journal);
    let journal = Arc::new(WorkerJournal::open(&config.database).unwrap());
    assert!(journal.result(&done.operation_id).unwrap().is_some());
    let second_worker = tokio::spawn({
        let journal = journal.clone();
        async move { zaturo_worker::connect_once(&config, DEVICE, journal).await }
    });
    completed(&client, &base, &next.id).await;
    second_worker.abort();
    server.abort();
}

#[tokio::test]
async fn hub_restart_reconciles_running_task_with_worker_durable_result() {
    let dir = tempfile::tempdir().unwrap();
    let (state, base, server) = start_hub(dir.path()).await;
    let task = state
        .engine
        .create(
            NewTask::diagnostic("local-worker", "restart"),
            zaturo_hub::now_ms(),
        )
        .unwrap();
    state
        .engine
        .register(
            "local-worker",
            &["diagnostic.v1".into()],
            zaturo_hub::now_ms(),
        )
        .unwrap();
    let assignment = state
        .engine
        .claim("local-worker", zaturo_hub::now_ms())
        .unwrap()
        .unwrap();
    state
        .engine
        .start(
            &task.id,
            "local-worker",
            &assignment.lease_token,
            zaturo_hub::now_ms(),
        )
        .unwrap();
    let config = worker_config(dir.path(), base);
    let journal = WorkerJournal::open(&config.database).unwrap();
    journal
        .begin(&task.operation_id, &task.id, zaturo_worker::now_ms())
        .unwrap();
    let original = zaturo_tools::diagnostic(&task.operation_id);
    journal
        .record_result(&task.operation_id, &original, zaturo_worker::now_ms())
        .unwrap();
    drop(journal);
    server.abort();
    let _ = server.await;
    drop(state);
    let (restarted, base, server) = start_hub(dir.path()).await;
    assert_eq!(
        restarted.engine.get(&task.id).unwrap().state,
        TaskState::Running
    );
    let config = worker_config(dir.path(), base.clone());
    let journal = Arc::new(WorkerJournal::open(&config.database).unwrap());
    let worker =
        tokio::spawn(async move { zaturo_worker::connect_once(&config, DEVICE, journal).await });
    let done = completed(&reqwest::Client::new(), &base, &task.id).await;
    assert_eq!(done.attempts, 1);
    assert_eq!(done.result, Some(original));
    worker.abort();
    server.abort();
}

#[tokio::test]
async fn worker_auth_and_http_resource_failures_are_enforced() {
    let dir = tempfile::tempdir().unwrap();
    let (_, base, server) = start_hub(dir.path()).await;
    let config = worker_config(dir.path(), base.clone());
    let journal = Arc::new(WorkerJournal::open(&config.database).unwrap());
    assert!(
        zaturo_worker::connect_once(&config, OWNER, journal)
            .await
            .is_err()
    );
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .get(format!("{base}/v1/tasks?limit=101"))
            .bearer_auth(OWNER)
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(
        client
            .post(format!("{base}/v1/tasks"))
            .bearer_auth(OWNER)
            .header("content-type", "application/json")
            .body("x".repeat(17 * 1024))
            .send()
            .await
            .unwrap()
            .status(),
        413
    );
    let task = submit(&client, &base, "cancel").await;
    let cancelled: Task = client
        .post(format!("{base}/v1/tasks/{}/cancel", task.id))
        .bearer_auth(OWNER)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(cancelled.state, TaskState::Cancelled);
    server.abort();
}
