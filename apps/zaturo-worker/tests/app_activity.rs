//! Native-client contracts over real HTTP/WebSocket/SQLite with synthetic identities only.
use chrono::{DateTime, FixedOffset};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use zaturo_core::{
    ChatAction, ChatReply, ChatRequest, FeedItem, FeedKind, FeedPage, NewTask, Task, TaskState,
};
use zaturo_db::WorkerJournal;
use zaturo_hub::{AppState, config::HubConfig};

const OWNER: &str = "synthetic-owner-for-app-activity-tests";
const HOME: &str = "synthetic-home-device-for-app-activity";
const WORK: &str = "synthetic-work-device-for-app-activity";
const READ: &str = "synthetic-read-only-app-for-activity";
const WRITE: &str = "synthetic-submitting-app-for-activity";

struct Hub {
    _directory: tempfile::TempDir,
    state: AppState,
    base: String,
    client: reqwest::Client,
    server: JoinHandle<()>,
}
impl Drop for Hub {
    fn drop(&mut self) {
        self.server.abort();
    }
}
fn private_file(path: &Path, value: impl AsRef<[u8]>) {
    fs::write(path, value).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
async fn serve(mut config: HubConfig) -> (AppState, String, JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    config.bind = listener.local_addr().unwrap();
    config.client_url = format!("http://{}", config.bind);
    let base = config.client_url.clone();
    let state = AppState::new(config, OWNER, HOME).unwrap();
    let router = zaturo_hub::router(state.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (state, base, server)
}
impl Hub {
    async fn start() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let mut devices = String::new();
        for (id, token) in [("local-worker", HOME), ("work-pc", WORK)] {
            let recipient =
                zaturo_transfer::crypto::generate_identity(&path.join(format!("{id}.identity")))
                    .unwrap();
            devices += &format!(
                "[[devices]]\nid={id:?}\ntoken_sha256='{:x}'\nencryption_recipient={recipient:?}\n",
                Sha256::digest(token)
            );
        }
        private_file(&path.join("devices.toml"), devices);
        private_file(
            &path.join("apps.toml"),
            format!(
                "[[clients]]\nid='read-app'\ntoken_sha256='{:x}'\ncan_submit=false\n[[clients]]\nid='write-app'\ntoken_sha256='{:x}'\ncan_submit=true\nviewer_device='local-worker'\n",
                Sha256::digest(READ),
                Sha256::digest(WRITE)
            ),
        );
        let config = HubConfig {
            bind: "127.0.0.1:0".parse().unwrap(),
            client_url: "http://127.0.0.1:1".into(),
            database: path.join("hub.db"),
            worker_id: "local-worker".into(),
            max_connections: 3,
            max_queued_tasks: 8,
            max_page_size: 100,
            lease_seconds: 30,
            max_workers: 2,
            devices_file: Some(path.join("devices.toml")),
            apps_file: Some(path.join("apps.toml")),
            transfer_root: Some(path.join("spool")),
            transfer_limits: Default::default(),
        };
        let (state, base, server) = serve(config).await;
        Self {
            _directory: directory,
            state,
            base,
            server,
            client: reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap(),
        }
    }
    async fn restart(&mut self) {
        self.server.abort();
        let (state, base, server) = serve(self.state.config.clone()).await;
        self.state = state;
        self.base = base;
        self.server = server;
    }
    async fn feed(&self, query: &[(&str, &str)]) -> FeedPage {
        self.client
            .get(format!("{}/v1/feed", self.base))
            .bearer_auth(READ)
            .query(query)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap()
    }
    async fn chat(&self, request: &ChatRequest) -> ChatReply {
        self.client
            .post(format!("{}/v1/chat", self.base))
            .bearer_auth(WRITE)
            .json(request)
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap()
    }
}
fn owner_day(timestamp: i64) -> String {
    DateTime::from_timestamp_millis(timestamp)
        .unwrap()
        .with_timezone(&FixedOffset::west_opt(4 * 3600).unwrap())
        .format("%Y-%m-%d")
        .to_string()
}

#[tokio::test]
async fn application_scopes_and_free_chat_replay_survive_http_restart() {
    let mut hub = Hub::start().await;
    let feed = format!("{}/v1/feed?day=2026-11-13", hub.base);
    assert_eq!(hub.client.get(&feed).send().await.unwrap().status(), 401);
    for token in [WORK, HOME] {
        assert_eq!(
            hub.client
                .get(&feed)
                .bearer_auth(token)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
    }
    for token in [OWNER, READ, WRITE] {
        assert_eq!(
            hub.client
                .get(&feed)
                .bearer_auth(token)
                .send()
                .await
                .unwrap()
                .status(),
            200
        );
    }
    assert_eq!(
        hub.state.viewer_device(WRITE).as_deref(),
        Some("local-worker")
    );
    assert_eq!(hub.state.viewer_device(READ), None);
    assert_eq!(
        hub.client
            .get(format!("{}/v1/workers", hub.base))
            .bearer_auth(READ)
            .send()
            .await
            .unwrap()
            .status(),
        200
    );
    for token in [READ, WRITE, WORK] {
        assert_eq!(
            hub.client
                .post(format!("{}/v1/tasks", hub.base))
                .bearer_auth(token)
                .json(&NewTask::diagnostic("work-pc", "app-is-not-admin"))
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
    }
    let request = ChatRequest {
        message: "Organizar vídeos de São Paulo e ações da semana.".into(),
        idempotency_key: "durable-free-chat".into(),
        target_worker: None,
        action: None,
    };
    for token in [READ, WORK] {
        assert_eq!(
            hub.client
                .post(format!("{}/v1/chat", hub.base))
                .bearer_auth(token)
                .json(&request)
                .send()
                .await
                .unwrap()
                .status(),
            401
        );
    }
    let original = hub.chat(&request).await;
    assert!(original.task.is_none());
    assert!(original.message.text.contains("sem execução automática"));
    assert!(hub.state.engine.list(100, 0).unwrap().is_empty());
    hub.restart().await;
    let replay = hub.chat(&request).await;
    assert_eq!(
        serde_json::to_value(&original).unwrap(),
        serde_json::to_value(&replay).unwrap()
    );
    let mut collision = request.clone();
    collision.message += " Different request.";
    assert_eq!(
        hub.client
            .post(format!("{}/v1/chat", hub.base))
            .bearer_auth(WRITE)
            .json(&collision)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    let day = owner_day(original.message.created_at);
    let page = hub
        .feed(&[("day", &day), ("offset_minutes", "-240"), ("kind", "chat")])
        .await;
    assert_eq!(page.items.len(), 2);
    assert!(!page.has_more);
    assert_eq!(
        page.items
            .iter()
            .filter(|i| i.role == "owner" && i.text == request.message)
            .count(),
        1
    );
    assert!(
        page.items
            .iter()
            .all(|i| !i.sensitive && i.task_id.is_none())
    );
    assert!(hub.state.engine.list(100, 0).unwrap().is_empty());
}

#[tokio::test]
async fn local_day_unicode_search_and_pagination_are_read_only_history_queries() {
    let hub = Hub::start().await;
    // 13 November 2026 00:00 UTC: owner day starts four hours later.
    const UTC_DAY: i64 = 1_794_528_000_000;
    const HOUR: i64 = 3_600_000;
    let fixtures = [
        (
            "previous",
            UTC_DAY + 4 * HOUR - 1,
            FeedKind::Service,
            "Vídeo anterior",
        ),
        (
            "first",
            UTC_DAY + 4 * HOUR,
            FeedKind::Chat,
            "São Paulo: ação vídeo A %_",
        ),
        (
            "second",
            UTC_DAY + 5 * HOUR,
            FeedKind::Service,
            "São Paulo: ação vídeo B",
        ),
        (
            "last",
            UTC_DAY + 28 * HOUR - 1,
            FeedKind::Service,
            "São Paulo: ação vídeo C",
        ),
        (
            "next",
            UTC_DAY + 28 * HOUR,
            FeedKind::Chat,
            "São Paulo próximo",
        ),
    ];
    for (id, created_at, kind, text) in fixtures {
        hub.state
            .engine
            .publish_activity(
                FeedItem {
                    id: id.into(),
                    created_at,
                    kind,
                    role: "assistant".into(),
                    text: text.into(),
                    task_id: None,
                    artifact_id: None,
                    sensitive: false,
                },
                &format!("fixture-{id}"),
            )
            .unwrap();
    }
    let first = hub
        .feed(&[
            ("day", "2026-11-13"),
            ("offset_minutes", "-240"),
            ("limit", "2"),
        ])
        .await;
    assert_eq!(
        first
            .items
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["last", "second"]
    );
    assert!(first.has_more);
    let second = hub
        .feed(&[
            ("day", "2026-11-13"),
            ("offset_minutes", "-240"),
            ("limit", "2"),
            ("offset", "2"),
        ])
        .await;
    assert_eq!(
        second
            .items
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["first"]
    );
    assert!(!second.has_more);
    let unicode = hub
        .feed(&[
            ("day", "2026-11-13"),
            ("offset_minutes", "-240"),
            ("q", "AÇÃO VÍDEO"),
            ("kind", "service"),
        ])
        .await;
    assert_eq!(
        unicode
            .items
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["last", "second"]
    );
    let literal = hub
        .feed(&[
            ("day", "2026-11-13"),
            ("offset_minutes", "-240"),
            ("q", "%_"),
        ])
        .await;
    assert_eq!(literal.items.len(), 1);
    assert_eq!(literal.items[0].id, "first");
    let utc = hub
        .feed(&[("day", "2026-11-13"), ("offset_minutes", "0")])
        .await;
    assert_eq!(
        utc.items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
        ["second", "first", "previous"]
    );
    let previous = hub
        .feed(&[("day", "2026-11-12"), ("offset_minutes", "-240")])
        .await;
    assert_eq!(previous.items[0].id, "previous");
    let next = hub
        .feed(&[("day", "2026-11-14"), ("offset_minutes", "-240")])
        .await;
    assert_eq!(next.items[0].id, "next");
    let again = hub
        .feed(&[
            ("day", "2026-11-13"),
            ("offset_minutes", "-240"),
            ("limit", "2"),
        ])
        .await;
    assert_eq!(again.items, first.items);
    assert_eq!(again.has_more, first.has_more);
    for query in [
        "day=2026-11-31",
        "day=2026-11-13&offset_minutes=841",
        "day=2026-11-13&limit=101",
    ] {
        assert_eq!(
            hub.client
                .get(format!("{}/v1/feed?{query}", hub.base))
                .bearer_auth(READ)
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
    }
}

#[tokio::test]
async fn explicit_app_diagnostic_reaches_real_worker_and_publishes_one_completion_notice() {
    let hub = Hub::start().await;
    let root = hub._directory.path();
    let config = zaturo_worker::config::WorkerConfig {
        hub_url: hub.base.clone(),
        worker_id: "work-pc".into(),
        database: root.join("worker.db"),
        workspace_root: root.join("workspaces"),
        artifact_root: root.join("artifacts"),
        capabilities: vec!["diagnostic.v1".into()],
        max_concurrent_tasks: 1,
        heartbeat_seconds: 1,
        reconnect_max_seconds: 2,
        models_config: None,
        files: None,
        youtube: None,
        android: None,
    };
    let journal = Arc::new(WorkerJournal::open(&config.database).unwrap());
    let worker =
        tokio::spawn(async move { zaturo_worker::connect_once(&config, WORK, journal).await });
    let reply = hub
        .chat(&ChatRequest {
            message: "Verifique o computador de trabalho.".into(),
            idempotency_key: "explicit-diagnostic".into(),
            target_worker: Some("work-pc".into()),
            action: Some(ChatAction::Diagnostic),
        })
        .await;
    let submitted = reply.task.unwrap();
    let completed: Task = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let task: Task = hub
                .client
                .get(format!("{}/v1/tasks/{}", hub.base, submitted.id))
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
                break task;
            }
            assert!(
                !task.state.is_terminal(),
                "diagnostic ended unexpectedly: {task:?}"
            );
            assert!(
                !worker.is_finished(),
                "synthetic worker exited before completion"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("real worker did not finish the typed diagnostic");
    assert_eq!(completed.attempts, 1);
    assert!(
        completed
            .result
            .as_ref()
            .unwrap()
            .validates(&completed.operation_id)
    );
    let day = owner_day(completed.updated_at);
    let page = hub
        .feed(&[("day", &day), ("offset_minutes", "-240"), ("kind", "task")])
        .await;
    let notices = page
        .items
        .iter()
        .filter(|i| i.task_id.as_deref() == Some(&completed.id))
        .collect::<Vec<_>>();
    assert_eq!(notices.len(), 1);
    assert!(notices[0].text.starts_with("Tarefa concluída"));
    assert!(!notices[0].sensitive);
    assert_eq!(notices[0].role, "assistant");
    let inventory: serde_json::Value = hub
        .client
        .get(format!("{}/v1/services", hub.base))
        .bearer_auth(READ)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        inventory["services"]
            .as_array()
            .unwrap()
            .iter()
            .any(|service| service["id"] == "diagnostic"
                && service["workers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|id| id == "work-pc"))
    );
    let old = hub
        .feed(&[("day", "2001-01-01"), ("offset_minutes", "-240")])
        .await;
    assert!(old.items.is_empty());
    assert_eq!(
        hub.feed(&[("day", &day), ("offset_minutes", "-240"), ("kind", "task")])
            .await
            .items,
        page.items
    );
    worker.abort();
}
