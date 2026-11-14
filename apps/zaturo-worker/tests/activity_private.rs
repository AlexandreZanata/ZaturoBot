//! Full hub/worker private-view transport with synthetic pixels and a fake USB ADB only.
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, sync::Arc, time::Duration};
use zaturo_core::{AndroidCapability, RegisteredAndroidDevice};
use zaturo_hub::{AppState, config::HubConfig};
const OWNER: &str = "synthetic-owner-for-private-view-tests";
const HOME: &str = "synthetic-home-device-for-private-tests";
const WORK: &str = "synthetic-work-device-for-private-tests";
const APP: &str = "synthetic-home-app-for-private-view-tests";
const OTHER: &str = "synthetic-other-app-for-private-view-tests";
const MOBILE: &str = "synthetic-mobile-app-without-screen-grant";
#[tokio::test]
async fn private_frame_is_recipient_bound_once_memory_only_and_cancellable() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    let mut devices = String::new();
    for (id, key) in [("local-worker", HOME), ("work-pc", WORK)] {
        let recipient =
            zaturo_transfer::crypto::generate_identity(&p.join(format!("{id}.identity"))).unwrap();
        devices += &format!(
            "[[devices]]\nid={id:?}\ntoken_sha256=\"{:x}\"\nencryption_recipient={recipient:?}\n",
            Sha256::digest(key)
        );
    }
    fs::write(p.join("devices.toml"), devices).unwrap();
    fs::set_permissions(p.join("devices.toml"), fs::Permissions::from_mode(0o600)).unwrap();
    let apps = format!(
        "[[clients]]\nid='home-app'\ntoken_sha256='{:x}'\ncan_submit=true\nviewer_device='local-worker'\n[[clients]]\nid='other-app'\ntoken_sha256='{:x}'\ncan_submit=true\nviewer_device='work-pc'\n[[clients]]\nid='mobile-app'\ntoken_sha256='{:x}'\ncan_submit=true\n",
        Sha256::digest(APP),
        Sha256::digest(OTHER),
        Sha256::digest(MOBILE)
    );
    fs::write(p.join("apps.toml"), apps).unwrap();
    fs::set_permissions(p.join("apps.toml"), fs::Permissions::from_mode(0o600)).unwrap();
    let fake = p.join("adb");
    fs::write(&fake,r#"#!/usr/bin/python3
import sys,struct,zlib
args=sys.argv[1:]
if args[:4]==['-H','127.0.0.1','-P','5037']:args=args[4:]
if args==['devices','-l']:
 print('List of devices attached\nUSB_TEST device usb:1-1 product:test model:Pixel device:synthetic transport_id:1\n')
elif args[-3:]==['exec-out','screencap','-p']:
 def chunk(tag,data):return struct.pack('>I',len(data))+tag+data+struct.pack('>I',zlib.crc32(tag+data)&0xffffffff)
 png=b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',2,2,8,6,0,0,0))+chunk(b'IDAT',zlib.compress((b'\0'+b'\xff\xdd\0\xff'*2)*2))+chunk(b'IEND',b'')
 sys.stdout.buffer.write(png)
else:
 print('test')
"#).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let origin = format!("http://{address}");
    let hub = HubConfig {
        bind: address,
        client_url: origin.clone(),
        database: p.join("hub.db"),
        worker_id: "local-worker".into(),
        max_connections: 3,
        max_queued_tasks: 8,
        max_page_size: 100,
        lease_seconds: 30,
        max_workers: 2,
        devices_file: Some(p.join("devices.toml")),
        apps_file: Some(p.join("apps.toml")),
        transfer_root: Some(p.join("spool")),
        transfer_limits: Default::default(),
    };
    let state = AppState::new(hub, OWNER, HOME).unwrap();
    let router = zaturo_hub::router(state.clone());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let mut config: zaturo_worker::config::WorkerConfig =
        toml::from_str(include_str!("../../../config/worker.example.toml")).unwrap();
    config.hub_url = origin.clone();
    config.worker_id = "work-pc".into();
    config.database = p.join("work.db");
    config.workspace_root = p.join("workspace");
    config.artifact_root = p.join("artifacts");
    config
        .capabilities
        .extend(["android.devices.v1".into(), "android.private.v1".into()]);
    config.android = Some(zaturo_worker::config::AndroidConfig {
        executable: fake,
        devices: vec![RegisteredAndroidDevice {
            device_id: "owner-phone".into(),
            adb_serial: "USB_TEST".into(),
            display_name: Some("Synthetic phone".into()),
            capabilities: vec![
                AndroidCapability::DeviceStatus,
                AndroidCapability::SensitiveScreenView,
            ],
        }],
        allow_sensitive_views: true,
    });
    let journal = Arc::new(zaturo_db::WorkerJournal::open(&config.database).unwrap());
    let worker =
        tokio::spawn(async move { zaturo_worker::connect_once(&config, WORK, journal).await });
    let http = reqwest::Client::builder().no_proxy().build().unwrap();
    let body = serde_json::json!({"worker_id":"work-pc","device_id":"owner-phone","watch_seconds":0,"explicit_owner_confirmation":true});
    assert_eq!(
        http.post(format!("{origin}/v1/private-views"))
            .bearer_auth(MOBILE)
            .json(&body)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let mut denied = body.clone();
    denied["explicit_owner_confirmation"] = false.into();
    assert_eq!(
        http.post(format!("{origin}/v1/private-views"))
            .bearer_auth(APP)
            .json(&denied)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let session: serde_json::Value = http
        .post(format!("{origin}/v1/private-views"))
        .bearer_auth(APP)
        .json(&body)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = session["id"].as_str().unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let status: serde_json::Value = http
                .get(format!("{origin}/v1/private-views/{id}"))
                .bearer_auth(APP)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .await
                .unwrap();
            if status["state"] == "ready" {
                break;
            }
            if worker.is_finished() {
                panic!(
                    "worker stopped: {:?}",
                    state.engine.get(session["task_id"].as_str().unwrap())
                );
            }
            let observed = state
                .engine
                .get(session["task_id"].as_str().unwrap())
                .unwrap();
            assert!(
                !matches!(observed.state, zaturo_core::TaskState::Failed),
                "private task failed: {observed:?}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        http.get(format!("{origin}/v1/private-views/{id}"))
            .bearer_auth(OTHER)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        http.get(format!("{origin}/v1/device/private-views/{id}/grant"))
            .bearer_auth(HOME)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    let response = http
        .get(format!("{origin}/v1/private-views/{id}/content"))
        .bearer_auth(APP)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    assert!(
        response
            .headers()
            .get("cache-control")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("no-store")
    );
    let cipher = response.bytes().await.unwrap();
    let png =
        zaturo_transfer::crypto::decrypt_private_image(&cipher, &p.join("local-worker.identity"))
            .unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(
        zaturo_transfer::crypto::decrypt_private_image(&cipher, &p.join("work-pc.identity"))
            .is_err()
    );
    assert_eq!(
        http.get(format!("{origin}/v1/private-views/{id}/content"))
            .bearer_auth(APP)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        http.get(format!("{origin}/v1/device/private-views/{id}/grant"))
            .bearer_auth(WORK)
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    assert_eq!(
        http.post(format!("{origin}/v1/device/private-views/{id}/content"))
            .bearer_auth(WORK)
            .body(cipher.to_vec())
            .send()
            .await
            .unwrap()
            .status(),
        204,
        "lost upload reply is idempotent even after consumption"
    );
    assert_eq!(fs::read_dir(p.join("spool")).unwrap().count(), 0);
    assert!(
        !fs::read(p.join("hub.db"))
            .unwrap()
            .windows(8)
            .any(|w| w == b"\x89PNG\r\n\x1a\n")
    );
    assert_eq!(
        http.delete(format!("{origin}/v1/private-views/{id}"))
            .bearer_auth(APP)
            .send()
            .await
            .unwrap()
            .status(),
        204
    );
    let mut watch = body.clone();
    watch["watch_seconds"] = 5.into();
    let session: serde_json::Value = http
        .post(format!("{origin}/v1/private-views"))
        .bearer_auth(APP)
        .json(&watch)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = session["id"].as_str().unwrap();
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let status: serde_json::Value = http
                .get(format!("{origin}/v1/private-views/{id}"))
                .bearer_auth(APP)
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .await
                .unwrap();
            if status["state"] == "unchanged" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap();
    http.delete(format!("{origin}/v1/private-views/{id}"))
        .bearer_auth(APP)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    watch["watch_seconds"] = 60.into();
    let session: serde_json::Value = http
        .post(format!("{origin}/v1/private-views"))
        .bearer_auth(APP)
        .json(&watch)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = session["id"].as_str().unwrap();
    let task = session["task_id"].as_str().unwrap();
    http.delete(format!("{origin}/v1/private-views/{id}"))
        .bearer_auth(APP)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    assert_eq!(
        state.engine.get(task).unwrap().state,
        zaturo_core::TaskState::Cancelled
    );
    assert_eq!(
        http.get(format!("{origin}/v1/private-views/{id}"))
            .bearer_auth(APP)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    worker.abort();
    server.abort();
}
