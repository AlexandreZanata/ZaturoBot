//! Actual loopback HTTP/WS, private encrypted files, device isolation and interrupted upload.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    sync::Arc,
    time::Duration,
};
use zaturo_core::{CreateTransfer, FileTransfer, NewTask, Task, TaskState, TransferState};
use zaturo_hub::{AppState, config::HubConfig};
use zaturo_transfer::{blob, client::DeviceClient, crypto};
const OWNER: &str = "synthetic-owner-token-for-transfer-tests";
const HOME: &str = "synthetic-home-token-for-transfer-tests";
const WORK: &str = "synthetic-work-token-for-transfer-tests";
const OTHER: &str = "synthetic-other-token-for-transfer-tests";
#[tokio::test]
async fn two_devices_resume_encrypted_delivery_diagnostic_and_denials() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    let mut registry = String::new();
    for (id, token) in [("home-pc", HOME), ("work-pc", WORK), ("other-pc", OTHER)] {
        let recipient = crypto::generate_identity(&p.join(id)).unwrap();
        registry.push_str(&format!(
            "[[devices]]\nid={id:?}\ntoken_sha256=\"{:x}\"\nencryption_recipient={recipient:?}\n",
            Sha256::digest(token.as_bytes())
        ));
    }
    let registry_path = p.join("devices.toml");
    fs::write(&registry_path, registry).unwrap();
    fs::set_permissions(&registry_path, fs::Permissions::from_mode(0o600)).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let origin = format!("http://{address}");
    let config = HubConfig {
        bind: address,
        client_url: origin.clone(),
        database: p.join("hub.db"),
        worker_id: "home-pc".into(),
        max_connections: 4,
        max_queued_tasks: 8,
        max_page_size: 100,
        lease_seconds: 30,
        max_workers: 3,
        devices_file: Some(registry_path),
        transfer_root: Some(p.join("spool")),
        transfer_limits: Default::default(),
    };
    let state = AppState::new(config, OWNER, "synthetic-disabled-legacy-device-token").unwrap();
    let router = zaturo_hub::router(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let home = DeviceClient::new(&origin, HOME, "home-pc").unwrap();
    let work = DeviceClient::new(&origin, WORK, "work-pc").unwrap();
    let other = DeviceClient::new(&origin, OTHER, "other-pc").unwrap();
    let http = reqwest::Client::new();
    assert_eq!(
        http.get(format!("{origin}/v1/device/peers"))
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        http.get(format!("{origin}/v1/device/peers"))
            .bearer_auth(OWNER)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        http.get(format!("{origin}/v1/workers"))
            .bearer_auth(WORK)
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(
        http.get(format!("{origin}/v1/device/peers"))
            .bearer_auth("synthetic-disabled-legacy-device-token")
            .send()
            .await
            .unwrap()
            .status(),
        401
    );
    assert_eq!(work.peers().await.unwrap().len(), 3);
    let payload = vec![83u8; zaturo_core::DEFAULT_CHUNK_BYTES + 12345];
    fs::write(p.join("report.bin"), &payload).unwrap();
    let home_cache = p.join("home-cache");
    let key = home
        .prepare(
            &p.join("report.bin"),
            "work-pc",
            &p.join("home-pc"),
            &home_cache,
        )
        .await
        .unwrap();
    let request: CreateTransfer = serde_json::from_reader(
        blob::read_file(&home_cache.join(format!("{key}.request.json"))).unwrap(),
    )
    .unwrap();
    let t: FileTransfer = home.post("/v1/device/transfers", &request).await.unwrap();
    let mut cipher = blob::read_file(&home_cache.join(format!("{key}.send.age"))).unwrap();
    let mut first = vec![0; 65536];
    cipher.read_exact(&mut first).unwrap();
    let url = format!("{origin}/v1/device/transfers/{}/chunk?offset=0", t.id);
    assert!(
        http.put(&url)
            .bearer_auth(WORK)
            .body(first.clone())
            .send()
            .await
            .unwrap()
            .status()
            .is_client_error()
    );
    assert!(
        http.put(&url)
            .bearer_auth(HOME)
            .body(first.clone())
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
    );
    assert!(
        http.put(&url)
            .bearer_auth(HOME)
            .body(first.clone())
            .send()
            .await
            .unwrap()
            .status()
            .is_success(),
        "exact replay after lost reply"
    );
    let mut changed = first.clone();
    changed[40] ^= 1;
    assert!(
        http.put(&url)
            .bearer_auth(HOME)
            .body(changed)
            .send()
            .await
            .unwrap()
            .status()
            .is_client_error()
    );
    let spool = p.join("spool").join(blob::object_name(&t.id).unwrap());
    let mut orphan = fs::OpenOptions::new().append(true).open(&spool).unwrap();
    orphan.write_all(b"orphan after simulated crash").unwrap();
    drop(orphan);
    assert_eq!(
        home.status(&t.id).await.unwrap().uploaded_bytes,
        first.len() as u64
    );
    let t = home.upload(&key, &home_cache).await.unwrap();
    assert_eq!(t.state, TransferState::Ready);
    assert!(other.status(&t.id).await.is_err());
    assert_eq!(
        http.get(format!(
            "{origin}/v1/device/transfers/{}/chunk?offset=0",
            t.id
        ))
        .bearer_auth(OTHER)
        .send()
        .await
        .unwrap()
        .status(),
        404
    );
    assert!(
        !fs::read(&spool)
            .unwrap()
            .windows(40)
            .any(|b| b == [83u8; 40])
    );
    let inbox = p.join("work-inbox");
    let work_cache = p.join("work-cache");
    let out = work
        .receive(&t.id, &p.join("work-pc"), &work_cache, &inbox)
        .await
        .unwrap();
    assert_eq!(fs::read(&out).unwrap(), payload);
    assert_eq!(
        home.status(&t.id).await.unwrap().state,
        TransferState::Delivered
    );
    // Repeated delivery is idempotent; a changed destination is never replaced.
    work.receive(&t.id, &p.join("work-pc"), &work_cache, &inbox)
        .await
        .unwrap();
    fs::write(&out, b"owner edited this").unwrap();
    assert!(
        work.receive(&t.id, &p.join("work-pc"), &work_cache, &inbox)
            .await
            .is_err()
    );
    assert_eq!(fs::read(&out).unwrap(), b"owner edited this");
    fs::write(p.join("reply.txt"), b"returned from work").unwrap();
    let back = work
        .prepare(
            &p.join("reply.txt"),
            "home-pc",
            &p.join("work-pc"),
            &work_cache,
        )
        .await
        .unwrap();
    let back = work.upload(&back, &work_cache).await.unwrap();
    let received = home
        .receive(
            &back.id,
            &p.join("home-pc"),
            &home_cache,
            &p.join("home-inbox"),
        )
        .await
        .unwrap();
    assert_eq!(fs::read(received).unwrap(), b"returned from work");
    // A legitimate hidden filename must never collide with plaintext staging or disappear after ack.
    fs::write(
        p.join(".plaintext.partial"),
        b"hidden file must survive verified publication",
    )
    .unwrap();
    let hidden = home
        .prepare(
            &p.join(".plaintext.partial"),
            "work-pc",
            &p.join("home-pc"),
            &home_cache,
        )
        .await
        .unwrap();
    let hidden = home.upload(&hidden, &home_cache).await.unwrap();
    let hidden = work
        .receive(&hidden.id, &p.join("work-pc"), &work_cache, &inbox)
        .await
        .unwrap();
    assert_eq!(
        fs::read(hidden).unwrap(),
        b"hidden file must survive verified publication"
    );
    // A whole page of invalid encrypted files cannot starve a later valid arrival.
    let malformed = b"age-encryption.org/v1\ninvalid encrypted fixture";
    for n in 0..20 {
        let bad = CreateTransfer {
            recipient: "work-pc".into(),
            filename: format!("bad-{n}.bin"),
            size_bytes: malformed.len() as u64,
            sha256: format!("{:x}", Sha256::digest(malformed)),
            idempotency_key: format!("bad-{n}"),
        };
        let t: FileTransfer = home.post("/v1/device/transfers", &bad).await.unwrap();
        assert!(
            http.put(format!(
                "{origin}/v1/device/transfers/{}/chunk?offset=0",
                t.id
            ))
            .bearer_auth(HOME)
            .body(malformed.to_vec())
            .send()
            .await
            .unwrap()
            .status()
            .is_success()
        );
        let _: FileTransfer = home
            .post(
                &format!("/v1/device/transfers/{}/complete", t.id),
                &serde_json::json!({}),
            )
            .await
            .unwrap();
    }
    fs::write(p.join("after-bad.txt"), b"valid file after a blocked page").unwrap();
    let key = home
        .prepare(
            &p.join("after-bad.txt"),
            "work-pc",
            &p.join("home-pc"),
            &home_cache,
        )
        .await
        .unwrap();
    home.upload(&key, &home_cache).await.unwrap();
    let report = work
        .sync(&p.join("work-pc"), &work_cache, &inbox)
        .await
        .unwrap();
    assert_eq!(report.failed, 20);
    assert_eq!(report.received.len(), 1);
    assert_eq!(
        fs::read(&report.received[0]).unwrap(),
        b"valid file after a blocked page"
    );
    // A token cannot claim a different WebSocket worker identity.
    let mut worker: zaturo_worker::config::WorkerConfig =
        toml::from_str(include_str!("../../../config/worker.example.toml")).unwrap();
    worker.hub_url = origin.clone();
    worker.worker_id = "home-pc".into();
    worker.database = p.join("worker.db");
    worker.workspace_root = p.join("workspace");
    worker.artifact_root = p.join("artifact");
    let journal = Arc::new(zaturo_db::WorkerJournal::open(&worker.database).unwrap());
    tokio::time::timeout(
        Duration::from_secs(3),
        zaturo_worker::connect_once(&worker, WORK, journal.clone()),
    )
    .await
    .unwrap()
    .ok();
    assert!(state.engine.workers(100, 0).unwrap().is_empty());
    worker.worker_id = "work-pc".into();
    let job =
        tokio::spawn(async move { zaturo_worker::connect_once(&worker, WORK, journal).await });
    let task: Task = work
        .post(
            "/v1/device/tasks",
            &NewTask::diagnostic("work-pc", "work-diagnostic"),
        )
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let t: Task = work
                .get(&format!("/v1/device/tasks/{}", task.id))
                .await
                .unwrap();
            if t.state.is_terminal() {
                break t;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(result.state, TaskState::Completed);
    assert!(
        home.get::<Task>(&format!("/v1/device/tasks/{}", task.id))
            .await
            .is_err()
    );
    assert!(
        work.post::<_, Task>(
            "/v1/device/tasks",
            &NewTask::diagnostic("home-pc", "wrong-target")
        )
        .await
        .is_err()
    );
    job.abort();
    server.abort();
}

#[tokio::test]
async fn transient_http_retries_are_bounded_and_auth_denials_are_not_retried() {
    use axum::{Json, Router, http::StatusCode, routing::post};
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = Arc::new(AtomicUsize::new(0));
    let denied = Arc::new(AtomicUsize::new(0));
    let always = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let denial = denied.clone();
    let failure = always.clone();
    let app = Router::new()
        .route(
            "/retry",
            post(move |Json(body): Json<serde_json::Value>| {
                let counter = counter.clone();
                async move {
                    assert_eq!(
                        body,
                        serde_json::json!({"idempotency_key":"same-safe-request"})
                    );
                    let status = if counter.fetch_add(1, Ordering::SeqCst) < 2 {
                        StatusCode::SERVICE_UNAVAILABLE
                    } else {
                        StatusCode::OK
                    };
                    (status, Json(body))
                }
            }),
        )
        .route(
            "/deny",
            post(move || {
                let counter = denial.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    StatusCode::UNAUTHORIZED
                }
            }),
        )
        .route(
            "/always",
            post(move || {
                let counter = failure.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    StatusCode::SERVICE_UNAVAILABLE
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = DeviceClient::new(&origin, WORK, "work-pc").unwrap();
    let body = serde_json::json!({"idempotency_key":"same-safe-request"});
    assert_eq!(
        client
            .post::<_, serde_json::Value>("/retry", &body)
            .await
            .unwrap(),
        body
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert!(
        client
            .post::<_, serde_json::Value>("/deny", &body)
            .await
            .is_err()
    );
    assert_eq!(denied.load(Ordering::SeqCst), 1);
    assert!(
        client
            .post::<_, serde_json::Value>("/always", &body)
            .await
            .is_err()
    );
    assert_eq!(always.load(Ordering::SeqCst), 4);
    server.abort();
}
