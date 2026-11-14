//! Run only in a separate Xvfb display. Exercises synthetic encrypted frame -> UI -> expiry.
use anyhow::{Result, ensure};
use slint::Image;
use std::{
    io::Cursor,
    os::unix::fs::PermissionsExt,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use zaturo_desktop::{config::DesktopConfig, private_view::PrivateViewer, ui::AppWindow};
fn main() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let identity = temporary.path().join("identity");
    let recipient = zaturo_transfer::crypto::generate_identity(&identity)?;
    let pixels = image::RgbaImage::from_pixel(12, 20, image::Rgba([220, 185, 0, 255]));
    let mut png = Cursor::new(Vec::new());
    pixels.write_to(&mut png, image::ImageFormat::Png)?;
    let cipher = Arc::new(zaturo_transfer::crypto::encrypt_private_image(
        png.get_ref(),
        &recipient,
    )?);
    let token = "synthetic-private-view-frontend-smoke-token";
    let env = temporary.path().join("app.env");
    std::fs::write(&env, format!("ZATURO_APP_TOKEN={token}\n"))?;
    std::fs::set_permissions(&env, std::fs::Permissions::from_mode(0o600))?;
    let id = uuid::Uuid::new_v4().to_string();
    let id_copy = id.clone();
    let frames = Arc::new(AtomicUsize::new(0));
    let removals = Arc::new(AtomicUsize::new(0));
    let received = frames.clone();
    let removed = removals.clone();
    let expiry = Arc::new(std::sync::atomic::AtomicI64::new(0));
    let (address_tx, address_rx) = std::sync::mpsc::channel();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        runtime.block_on(async move {
            let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("test listener");address_tx.send(listener.local_addr().expect("test address")).expect("test receiver");tokio::pin!(stop_rx);
            loop {let accepted=tokio::select!{_=&mut stop_rx=>break,accepted=listener.accept()=>accepted};let Ok((mut socket,_))=accepted else{break;};let mut buffer=Vec::new();let mut block=[0;4096];loop {let count=socket.read(&mut block).await.expect("test request");if count==0{break;}buffer.extend_from_slice(&block[..count]);if let Some(index)=buffer.windows(4).position(|b|b==b"\r\n\r\n"){let header=String::from_utf8_lossy(&buffer[..index]);let length=header.lines().find_map(|l|l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|s|s.trim().parse::<usize>().ok())).unwrap_or(0);if buffer.len()>=index+4+length{break;}}assert!(buffer.len()<16384);}
                let split=buffer.windows(4).position(|b|b==b"\r\n\r\n").expect("request header");let header=String::from_utf8_lossy(&buffer[..split]);assert!(header.to_ascii_lowercase().contains(&format!("authorization: bearer {token}")));let first=header.lines().next().expect("request line");let mut status="200 OK";let body=if first.starts_with("POST ") {let request:serde_json::Value=serde_json::from_slice(&buffer[split+4..]).expect("creation body");assert_eq!(request["explicit_owner_confirmation"],true);serde_json::to_vec(&serde_json::json!({"id":id_copy,"state":"waiting","expires_at":chrono::Utc::now().timestamp_millis()+90000,"task_id":null})).expect("metadata")}
                else if first.starts_with("DELETE "){removed.fetch_add(1,Ordering::SeqCst);b"{}".to_vec()}
                else if first.contains("/content "){if received.fetch_add(1,Ordering::SeqCst)>0 {status="410 Gone";b"{}".to_vec()}else{cipher.to_vec()}}
                else {let initial=expiry.load(Ordering::SeqCst);let deadline=if initial==0 {let t=chrono::Utc::now().timestamp_millis()+3500;expiry.store(t,Ordering::SeqCst);t}else{initial};serde_json::to_vec(&serde_json::json!({"id":id_copy,"state":"ready","expires_at":deadline,"task_id":null})).expect("metadata")};let response=format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());socket.write_all(response.as_bytes()).await.expect("test header");socket.write_all(&body).await.expect("test body");
            }
        });
    });
    let address = address_rx.recv()?;
    let mut config = DesktopConfig::defaults(temporary.path());
    config.hub_url = format!("http://{address}");
    config.credentials_file = env;
    config.transfer_identity = Some(identity);
    let settings = Arc::new(Mutex::new(config));
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()?;
    let ui = AppWindow::new()?;
    ui.set_page("private".into());
    let viewer = std::rc::Rc::new(PrivateViewer::new(settings, ui.as_weak())?);
    let seen = Arc::new(AtomicBool::new(false));
    let weak = ui.as_weak();
    let observed = seen.clone();
    let owner = viewer.clone();
    let tick = slint::Timer::default();
    tick.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(100),
        move || {
            if let Some(ui) = weak.upgrade() {
                if ui.get_private_image_visible() {
                    observed.store(true, Ordering::SeqCst);
                }
                if owner.tick() {
                    ui.set_private_image(Image::default());
                    ui.set_private_image_visible(false);
                    ui.set_private_running(false);
                }
            }
        },
    );
    let start = slint::Timer::default();
    {
        let viewer = viewer.clone();
        start.start(
            slint::TimerMode::SingleShot,
            Duration::from_millis(200),
            move || {
                assert!(viewer.start("owner-phone".into(), 0));
            },
        );
    }
    let ended = Arc::new(AtomicBool::new(false));
    let finish = slint::Timer::default();
    {
        let weak = ui.as_weak();
        let ended = ended.clone();
        let seen = seen.clone();
        finish.start(
            slint::TimerMode::SingleShot,
            Duration::from_secs(6),
            move || {
                if let Some(ui) = weak.upgrade() {
                    ended.store(
                        seen.load(Ordering::SeqCst) && !ui.get_private_image_visible(),
                        Ordering::SeqCst,
                    );
                }
                let _ = slint::quit_event_loop();
            },
        );
    }
    ui.run()?;
    viewer.close();
    tick.stop();
    start.stop();
    finish.stop();
    drop(tick);
    drop(start);
    drop(ui);
    drop(viewer);
    let _ = stop_tx.send(());
    let _ = server.join();
    ensure!(
        seen.load(Ordering::SeqCst) && ended.load(Ordering::SeqCst),
        "private frame did not appear then clear at the server deadline"
    );
    ensure!(
        frames.load(Ordering::SeqCst) == 1 && removals.load(Ordering::SeqCst) >= 1,
        "frame must be fetched once and revoked"
    );
    ensure!(
        std::fs::read_dir(temporary.path())?.count() == 2,
        "no plaintext screen file may be created"
    );
    println!(
        "Synthetic private viewer passed: encrypted one-time fetch, frame shown, server deadline cleared it, no plaintext file."
    );
    Ok(())
}
