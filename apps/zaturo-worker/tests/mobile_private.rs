//! Mobile public-key enrollment and RAM-only relay ownership, with synthetic bytes only.
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt};
use zaturo_hub::{AppState, config::HubConfig};
const OWNER: &str = "synthetic-owner-for-mobile-private-tests";
const WORK: &str = "synthetic-work-for-mobile-private-tests";
const A: &str = "synthetic-mobile-a-for-private-tests";
const B: &str = "synthetic-mobile-b-for-private-tests";
const DENIED: &str = "synthetic-app-without-private-view-grant";
#[tokio::test]
async fn mobile_keeps_its_identity_off_hub_and_cannot_read_another_viewers_frame() {
    let directory = tempfile::tempdir().unwrap();
    let p = directory.path();
    let recipient = zaturo_transfer::crypto::generate_identity(&p.join("work.identity")).unwrap();
    fs::write(
        p.join("devices.toml"),
        format!(
            "[[devices]]\nid='work-pc'\ntoken_sha256='{:x}'\nencryption_recipient={recipient:?}\n",
            Sha256::digest(WORK)
        ),
    )
    .unwrap();
    let mut apps = String::new();
    for (id, token, allow) in [
        ("mobile-a", A, true),
        ("mobile-b", B, true),
        ("ordinary", DENIED, false),
    ] {
        apps += &format!(
            "[[clients]]\nid={id:?}\ntoken_sha256='{:x}'\ncan_submit=true\nallow_private_views={allow}\n",
            Sha256::digest(token)
        );
    }
    fs::write(p.join("apps.toml"), apps).unwrap();
    for name in ["apps.toml", "devices.toml"] {
        fs::set_permissions(p.join(name), fs::Permissions::from_mode(0o600)).unwrap();
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let base = format!("http://{address}");
    let state = AppState::new(
        HubConfig {
            bind: address,
            client_url: base.clone(),
            database: p.join("hub.db"),
            worker_id: "work-pc".into(),
            max_connections: 3,
            max_queued_tasks: 8,
            max_page_size: 20,
            lease_seconds: 30,
            max_workers: 1,
            devices_file: Some(p.join("devices.toml")),
            apps_file: Some(p.join("apps.toml")),
            transfer_root: Some(p.join("spool")),
            transfer_limits: Default::default(),
        },
        OWNER,
        WORK,
    )
    .unwrap();
    let router = zaturo_hub::router(state.clone());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    let a = zaturo_transfer::crypto::generate_identity(&p.join("a.identity")).unwrap();
    let b = zaturo_transfer::crypto::generate_identity(&p.join("b.identity")).unwrap();
    for token in [WORK, DENIED] {
        assert_eq!(
            client
                .post(format!("{base}/v1/private-viewers"))
                .bearer_auth(token)
                .json(&serde_json::json!({"recipient":a,"explicit_owner_confirmation":true}))
                .send()
                .await
                .unwrap()
                .status(),
            403
        );
    }
    assert_eq!(
        client
            .post(format!("{base}/v1/private-viewers"))
            .bearer_auth(A)
            .json(&serde_json::json!({"recipient":a,"explicit_owner_confirmation":false}))
            .send()
            .await
            .unwrap()
            .status(),
        403
    );
    for (token, key) in [(A, &a), (B, &b)] {
        client
            .post(format!("{base}/v1/private-viewers"))
            .bearer_auth(token)
            .json(&serde_json::json!({"recipient":key,"explicit_owner_confirmation":true}))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }
    assert_eq!(
        state.engine.app_viewer("app:mobile-a").unwrap().as_deref(),
        Some(a.as_str())
    );
    let created:serde_json::Value=client.post(format!("{base}/v1/private-views")).bearer_auth(A).json(&serde_json::json!({"worker_id":"work-pc","device_id":"owner-phone","watch_seconds":0,"explicit_owner_confirmation":true})).send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
    let id = created["id"].as_str().unwrap();
    assert_eq!(
        client
            .get(format!("{base}/v1/private-views/{id}"))
            .bearer_auth(B)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let grant: zaturo_core::PrivateViewGrant = client
        .get(format!("{base}/v1/device/private-views/{id}/grant"))
        .bearer_auth(WORK)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(grant.recipient, a);
    let pixels = b"\x89PNG\r\n\x1a\nSYNTHETIC_PRIVATE_PIXELS_DO_NOT_PERSIST";
    let cipher = zaturo_transfer::crypto::encrypt_private_image(pixels, &a).unwrap();
    client
        .post(format!("{base}/v1/device/private-views/{id}/content"))
        .bearer_auth(WORK)
        .body(cipher.clone())
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    assert_eq!(
        client
            .get(format!("{base}/v1/private-views/{id}/content"))
            .bearer_auth(B)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let response = client
        .get(format!("{base}/v1/private-views/{id}/content"))
        .bearer_auth(A)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    assert!(
        response.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("no-store")
    );
    let bytes = response.bytes().await.unwrap();
    assert_eq!(&bytes[..], &cipher);
    assert_eq!(
        zaturo_transfer::crypto::decrypt_private_image(&bytes, &p.join("a.identity"))
            .unwrap()
            .as_slice(),
        pixels
    );
    assert!(zaturo_transfer::crypto::decrypt_private_image(&bytes, &p.join("b.identity")).is_err());
    assert_eq!(
        client
            .get(format!("{base}/v1/private-views/{id}/content"))
            .bearer_auth(A)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    for name in ["hub.db", "hub.db-wal"] {
        if let Ok(bytes) = fs::read(p.join(name)) {
            assert!(!bytes.windows(pixels.len()).any(|v| v == pixels));
            assert!(!bytes.windows(cipher.len()).any(|v| v == cipher));
        }
    }
    server.abort();
}
