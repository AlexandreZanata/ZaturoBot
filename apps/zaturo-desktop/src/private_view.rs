//! Ephemeral owner-only screen viewer. Never writes decrypted images or emits image content.
use crate::{
    api::{HubClient, PrivateView},
    config::DesktopConfig,
    ui::UiWeak,
};
use anyhow::{Result, ensure};
use std::{
    io::Cursor,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    time::Instant,
};
use zeroize::Zeroizing;
struct Session {
    metadata: PrivateView,
    shown_until: Option<Instant>,
    shown_wall_deadline: Option<i64>,
    waiting_deadline: Instant,
}
enum Command {
    Start {
        version: u64,
        device: String,
        watch: u32,
    },
    Poll {
        version: u64,
    },
    Cancel {
        session: PrivateView,
    },
    Quit,
}
pub struct PrivateViewer {
    sender: SyncSender<Command>,
    version: Arc<AtomicU64>,
    session: Arc<Mutex<Option<Session>>>,
    thread: Option<std::thread::JoinHandle<()>>,
    polling: Arc<AtomicBool>,
}
impl PrivateViewer {
    pub fn new(config: Arc<Mutex<DesktopConfig>>, ui: UiWeak) -> Result<Self> {
        let (sender, receiver) = sync_channel(4);
        let version = Arc::new(AtomicU64::new(0));
        let session: Arc<Mutex<Option<Session>>> = Arc::new(Mutex::new(None));
        let polling = Arc::new(AtomicBool::new(false));
        let in_flight = polling.clone();
        let current = version.clone();
        let state = session.clone();
        let thread = std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            while let Ok(command) = receiver.recv() {
                if matches!(command, Command::Quit) {
                    break;
                }
                let Ok(settings) = config.lock().map(|c| c.clone()) else {
                    break;
                };
                let client = settings
                    .token()
                    .and_then(|token| HubClient::new(&settings, &token));
                match command {
                    Command::Start {
                        version,
                        device,
                        watch,
                    } => {
                        let result = client.and_then(|client| {
                            let identity =
                                settings.transfer_identity.as_ref().ok_or_else(|| {
                                    anyhow::anyhow!("private transfer identity is not configured")
                                })?;
                            zaturo_transfer::crypto::public_identity(identity)?;
                            runtime.block_on(client.create_private(
                                &settings.target_worker,
                                &device,
                                watch,
                            ))
                        });
                        match result {
                            Ok(metadata) => {
                                if current.load(Ordering::SeqCst) != version {
                                    if let Ok(token) = settings
                                        .token()
                                        .and_then(|token| HubClient::new(&settings, &token))
                                    {
                                        let _ =
                                            runtime.block_on(token.cancel_private(&metadata.id));
                                    }
                                    continue;
                                }
                                if let Ok(mut state) = state.lock() {
                                    *state = Some(Session {
                                        metadata,
                                        shown_until: None,
                                        shown_wall_deadline: None,
                                        waiting_deadline: Instant::now()
                                            + std::time::Duration::from_secs(u64::from(watch) + 60),
                                    });
                                }
                                let _ = ui.upgrade_in_event_loop(move |ui| {
                                    ui.set_private_running(true);
                                    ui.set_private_status(
                                        "Pedido privado autorizado. Aguardando a captura cifrada."
                                            .into(),
                                    );
                                });
                            }
                            Err(_) => {
                                let _=ui.upgrade_in_event_loop(|ui|{ui.set_private_running(false);ui.set_private_status("Não foi possível autorizar. Verifique o telefone registrado, a permissão e a conexão.".into());});
                            }
                        }
                    }
                    Command::Poll { version } => {
                        let _guard = PollGuard(in_flight.clone());
                        let pending = state.lock().ok().and_then(|s| {
                            s.as_ref()
                                .filter(|s| s.shown_until.is_none())
                                .map(|s| s.metadata.clone())
                        });
                        let Some(pending) = pending else {
                            continue;
                        };
                        if current.load(Ordering::SeqCst) != version {
                            continue;
                        }
                        let outcome = (|| -> Result<ViewOutcome> {
                            let client = client?;
                            let metadata = runtime.block_on(client.private_status(&pending.id))?;
                            ensure!(
                                metadata.expires_at > chrono::Utc::now().timestamp_millis(),
                                "private view expired"
                            );
                            if metadata.state == "waiting" {
                                return Ok(ViewOutcome::Waiting);
                            }
                            if matches!(metadata.state.as_str(), "unchanged" | "no_change") {
                                return Ok(ViewOutcome::NoFrame(
                                    "Nenhuma alteração visual no intervalo autorizado.",
                                ));
                            }
                            if matches!(metadata.state.as_str(), "failed" | "cancelled") {
                                return Ok(ViewOutcome::NoFrame(
                                    "A captura foi cancelada ou o dispositivo não pôde concluir a operação.",
                                ));
                            }
                            ensure!(
                                metadata.state == "ready",
                                "private view is no longer available"
                            );
                            let encrypted =
                                runtime.block_on(client.private_content(&pending.id))?;
                            let identity =
                                settings.transfer_identity.as_ref().ok_or_else(|| {
                                    anyhow::anyhow!("private transfer identity is not configured")
                                })?;
                            let plaintext = zaturo_transfer::crypto::decrypt_private_image(
                                &encrypted, identity,
                            )?;
                            Ok(ViewOutcome::Pixels(
                                decode_png(&plaintext)?,
                                metadata.expires_at,
                            ))
                        })();
                        match outcome {
                            Ok(ViewOutcome::Pixels(pixels, frame_expiry))
                                if current.load(Ordering::SeqCst) == version =>
                            {
                                if let Ok(mut state) = state.lock()
                                    && let Some(session) = state.as_mut()
                                {
                                    let now = chrono::Utc::now().timestamp_millis();
                                    let deadline = frame_expiry.min(now + 59000);
                                    session.metadata.expires_at = frame_expiry;
                                    session.shown_until = Some(
                                        Instant::now()
                                            + std::time::Duration::from_millis(
                                                deadline.saturating_sub(now).max(0) as u64,
                                            ),
                                    );
                                    session.shown_wall_deadline = Some(deadline);
                                }
                                let check = current.clone();
                                let _=ui.upgrade_in_event_loop(move |ui|{if check.load(Ordering::SeqCst)!=version || ui.get_page()!="private" || !ui.window().is_visible() || chrono::Utc::now().timestamp_millis()>=frame_expiry {return;}let buffer=slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&pixels.bytes,pixels.width,pixels.height);ui.set_private_image(slint::Image::from_rgba8(buffer));ui.set_private_image_visible(true);ui.set_private_status("Sessão privada · a imagem será apagada automaticamente em até 60 segundos.".into());});
                            }
                            Ok(ViewOutcome::Waiting) => {}
                            Ok(ViewOutcome::NoFrame(message)) => {
                                if let Ok(mut state) = state.lock() {
                                    state.take();
                                }
                                if let Ok(client) = settings
                                    .token()
                                    .and_then(|token| HubClient::new(&settings, &token))
                                {
                                    let _ = runtime.block_on(client.cancel_private(&pending.id));
                                }
                                let _ = ui.upgrade_in_event_loop(move |ui| {
                                    ui.set_private_image(slint::Image::default());
                                    ui.set_private_image_visible(false);
                                    ui.set_private_running(false);
                                    ui.set_private_status(message.into());
                                });
                            }
                            _ => {
                                if let Ok(client) = settings
                                    .token()
                                    .and_then(|token| HubClient::new(&settings, &token))
                                {
                                    let _ = runtime.block_on(client.cancel_private(&pending.id));
                                }
                                if let Ok(mut state) = state.lock() {
                                    state.take();
                                }
                                let _=ui.upgrade_in_event_loop(|ui|{ui.set_private_image(slint::Image::default());ui.set_private_image_visible(false);ui.set_private_running(false);ui.set_private_status("A sessão expirou, foi consumida ou não pôde ser verificada. Nenhuma imagem foi salva.".into());});
                            }
                        }
                    }
                    Command::Cancel { session } => {
                        if let Ok(client) = client {
                            let _ = runtime.block_on(client.cancel_private(&session.id));
                        }
                    }
                    Command::Quit => break,
                }
            }
        });
        Ok(Self {
            sender,
            version,
            session,
            thread: Some(thread),
            polling,
        })
    }
    pub fn start(&self, device: String, watch: u32) -> bool {
        let version = self.version.fetch_add(1, Ordering::SeqCst) + 1;
        self.sender
            .try_send(Command::Start {
                version,
                device,
                watch,
            })
            .is_ok()
    }
    pub fn tick(&self) -> bool {
        let expired = self
            .session
            .lock()
            .ok()
            .and_then(|s| {
                s.as_ref().map(|s| {
                    Instant::now() >= s.waiting_deadline
                        || s.shown_until.is_some_and(|until| Instant::now() >= until)
                        || s.shown_wall_deadline
                            .is_some_and(|until| until <= chrono::Utc::now().timestamp_millis())
                        || s.metadata.expires_at <= chrono::Utc::now().timestamp_millis()
                })
            })
            .unwrap_or(false);
        if expired {
            self.close();
            return true;
        }
        if self
            .polling
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
            && self
                .sender
                .try_send(Command::Poll {
                    version: self.version.load(Ordering::SeqCst),
                })
                .is_err()
        {
            self.polling.store(false, Ordering::SeqCst);
        }
        false
    }
    pub fn close(&self) {
        self.version.fetch_add(1, Ordering::SeqCst);
        let session = self.session.lock().ok().and_then(|mut s| s.take());
        if let Some(session) = session {
            let _ = self.sender.try_send(Command::Cancel {
                session: session.metadata,
            });
        }
    }
}
impl Drop for PrivateViewer {
    fn drop(&mut self) {
        self.close();
        let _ = self.sender.send(Command::Quit);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
pub struct PrivatePixels {
    pub bytes: Zeroizing<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}
pub fn decode_png(bytes: &[u8]) -> Result<PrivatePixels> {
    ensure!(bytes.len() <= 8 * 1024 * 1024, "private PNG exceeds limit");
    let mut reader = image::ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(80 * 1024 * 1024);
    reader.limits(limits);
    let pixels = reader.decode()?.into_rgba8();
    let width = pixels.width();
    let height = pixels.height();
    Ok(PrivatePixels {
        bytes: Zeroizing::new(pixels.into_raw()),
        width,
        height,
    })
}

struct PollGuard(Arc<AtomicBool>);
impl Drop for PollGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

enum ViewOutcome {
    Waiting,
    NoFrame(&'static str),
    Pixels(PrivatePixels, i64),
}
