use anyhow::{Result, ensure};
use chrono::{DateTime, Local};
use clap::Parser;
use slint::{ModelRc, VecModel};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{SyncSender, sync_channel},
    },
    time::Duration,
};
use zaturo_desktop::ui::{AppWindow, FeedRow, UiWeak, WorkerRow};
use zaturo_desktop::{
    api::{ChatRequest, FeedItem, HubClient, Worker},
    config::DesktopConfig,
};
#[derive(Parser)]
#[command(version, about = "ZaturoBot native Linux hub and chat client")]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    quit_after_seconds: Option<u64>,
    #[arg(long,value_parser=["hub","services","workers","files","private","settings"])]
    page: Option<String>,
}
#[derive(Clone, Copy, Default)]
struct RefreshMode {
    append: bool,
    reset: bool,
    force_tail: bool,
}
#[derive(Clone)]
struct Filters {
    day: String,
    kind: String,
    query: String,
    offset: u32,
}
impl Filters {
    fn cache_key(&self) -> String {
        format!("{}\u{1f}{}\u{1f}{}", self.day, self.kind, self.query.trim())
    }
}
enum Command {
    Refresh {
        generation: u64,
        filters: Filters,
        mode: RefreshMode,
    },
    Send {
        generation: u64,
        request: ChatRequest,
        filters: Filters,
    },
    Save {
        generation: u64,
        url: String,
        token: String,
        worker: String,
        filters: Filters,
    },
    Quit,
}
fn filters(ui: &AppWindow, offset: u32) -> Filters {
    Filters {
        day: ui.get_selected_day().into(),
        query: ui.get_search_query().into(),
        kind: match ui.get_kind_index() {
            1 => "chat",
            2 => "task",
            3 => "file",
            4 => "android",
            5 => "service",
            _ => "",
        }
        .into(),
        offset,
    }
}
fn enqueue(sender: &SyncSender<Command>, command: Command, ui: &AppWindow) {
    let writing = matches!(&command, Command::Send { .. });
    if sender.try_send(command).is_err() {
        if writing {
            ui.set_sending(false);
        }
        ui.set_busy(false);
        ui.set_status_message("Aguarde a solicitação atual terminar e atualize novamente.".into());
    } else {
        if writing {
            ui.set_sending(true);
        }
        ui.set_busy(true);
    }
}
fn show_state(
    ui: UiWeak,
    generation: Arc<AtomicU64>,
    expected: u64,
    apply: impl FnOnce(&AppWindow) + Send + 'static,
) {
    let _ = ui.upgrade_in_event_loop(move |ui| {
        if generation.load(Ordering::SeqCst) == expected {
            apply(&ui);
        }
    });
}
fn row(item: FeedItem) -> FeedRow {
    let time = DateTime::from_timestamp_millis(item.created_at)
        .map(|t| t.with_timezone(&Local).format("%H:%M").to_string())
        .unwrap_or_default();
    let role = if item.role == "owner" || item.role == "user" {
        "Você"
    } else if item.role == "assistant" {
        "ZaturoBot"
    } else {
        "Sistema"
    };
    let category = match item.kind.as_str() {
        "chat" => "Conversa",
        "task" => "Tarefa",
        "file" => "Arquivo",
        "android" => "Android",
        "service" => "Serviço",
        _ => "Atividade",
    };
    FeedRow {
        id: item.id.into(),
        time: time.into(),
        label: if item.kind == "chat" {
            role.to_owned()
        } else {
            format!("{role} · {category}")
        }
        .into(),
        text: item.text.into(),
        role: item.role.into(),
        task: item.task_id.unwrap_or_default().into(),
        artifact: item.artifact_id.unwrap_or_default().into(),
    }
}
fn worker_row(worker: Worker) -> WorkerRow {
    let seen = DateTime::from_timestamp_millis(worker.last_seen_at)
        .map(|t| t.with_timezone(&Local).format("%d/%m %H:%M").to_string())
        .unwrap_or_else(|| "não informado".into());
    WorkerRow {
        id: worker.id.into(),
        detail: format!(
            "Último contato: {seen} · {} capacidades",
            worker.capabilities.len()
        )
        .into(),
    }
}
fn service_summary(value: serde_json::Value) -> String {
    let list = value
        .as_array()
        .or_else(|| value.get("services").and_then(|v| v.as_array()));
    let Some(list) = list else {
        return "O hub respondeu; consulte as capacidades de cada worker.".into();
    };
    list.iter()
        .map(|service| {
            let name = service
                .get("name")
                .or_else(|| service.get("label"))
                .or_else(|| service.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("Serviço");
            let status = service
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| {
                    if service
                        .get("workers")
                        .and_then(|v| v.as_array())
                        .is_some_and(|v| !v.is_empty())
                    {
                        "registered"
                    } else {
                        "not_configured"
                    }
                });
            let state = match status {
                "available" | "ready" | "enabled" | "implemented" => "disponível",
                "not_configured" | "disabled" => "não configurado",
                "planned" => "planejado",
                "registered" => "registrado no hub",
                _ => status,
            };
            format!("{name} · {state}")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}
fn refresh(
    runtime: &tokio::runtime::Runtime,
    config: &DesktopConfig,
    ui: UiWeak,
    generation: Arc<AtomicU64>,
    expected: u64,
    filter: Filters,
    mode: RefreshMode,
) -> Result<()> {
    let token = config.token()?;
    let client = HubClient::new(config, &token)?;
    let offset_minutes = Local::now().offset().local_minus_utc() / 60;
    let (page, workers, services) = runtime.block_on(async {
        let (page, workers, services) = tokio::join!(
            client.feed(
                &filter.day,
                offset_minutes,
                &filter.kind,
                &filter.query,
                filter.offset
            ),
            client.workers(),
            client.services()
        );
        Ok::<_, anyhow::Error>((
            page?,
            workers.unwrap_or_default(),
            services.ok().map(service_summary),
        ))
    })?;
    let worker_rows = workers.into_iter().map(worker_row).collect::<Vec<_>>();
    let filter_key = filter.cache_key();
    show_state(ui, generation, expected, move |ui| {
        let tail = mode.force_tail || (!mode.append && ui.get_following_tail());
        let filter_changed = ui.select_feed_filter(filter_key);
        if let Some(items) = ui.merge_page(page, mode.append, mode.reset || filter_changed) {
            ui.prepare_scroll(if tail {
                1
            } else if mode.append {
                2
            } else {
                3
            });
            ui.set_events(ModelRc::new(VecModel::from(
                items.into_iter().map(row).collect::<Vec<_>>(),
            )));
        } else if tail {
            // Explicit navigation/send intents survive unchanged responses and delayed layout.
            ui.scroll_to_bottom();
        }
        ui.set_workers(ModelRc::new(VecModel::from(worker_rows)));
        ui.set_online(true);
        ui.set_connection_state("Hub conectado".into());
        ui.set_busy(false);
        ui.set_status_message("Histórico sincronizado com seu hub privado.".into());
        if let Some(services) = services {
            ui.set_services_summary(services.into());
        }
    });
    Ok(())
}
fn main() -> Result<()> {
    ensure!(
        cfg!(target_os = "linux"),
        "this client currently supports Linux only"
    );
    let args = Cli::parse();
    let home =
        PathBuf::from(std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is required"))?);
    let config_path = args
        .config
        .unwrap_or_else(|| home.join(".config/zaturobot/desktop.toml"));
    let config = DesktopConfig::load(&config_path, &home)?;
    let shared = Arc::new(Mutex::new(config.clone()));
    let generation = Arc::new(AtomicU64::new(1));
    let (sender, receiver) = sync_channel::<Command>(16);
    let closing = Arc::new(AtomicBool::new(false));
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .renderer_name("software".into())
        .select()?;
    let ui = AppWindow::new()?;
    ui.set_selected_day(Local::now().format("%Y-%m-%d").to_string().into());
    ui.set_connection_url(config.hub_url.into());
    ui.set_worker_target(config.target_worker.into());
    let weak = ui.as_weak();
    let thread_config = shared.clone();
    let thread_generation = generation.clone();
    let thread_closing = closing.clone();
    let network = std::thread::spawn(move || {
        let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        else {
            return;
        };
        while let Ok(command) = receiver.recv() {
            if thread_closing.load(Ordering::SeqCst) {
                break;
            }
            if let Command::Refresh { generation, .. } = &command
                && thread_generation.load(Ordering::SeqCst) != *generation
            {
                continue;
            }
            let mut current = match thread_config.lock() {
                Ok(config) => config.clone(),
                Err(_) => break,
            };
            let (expected, result) = match command {
                Command::Quit => break,
                Command::Refresh {
                    generation,
                    filters,
                    mode,
                } => (
                    generation,
                    refresh(
                        &runtime,
                        &current,
                        weak.clone(),
                        thread_generation.clone(),
                        generation,
                        filters,
                        mode,
                    ),
                ),
                Command::Send {
                    generation,
                    request,
                    filters,
                } => {
                    let outcome = (|| -> Result<()> {
                        let token = current.token()?;
                        let client = HubClient::new(&current, &token)?;
                        runtime.block_on(client.send(&request))?;
                        let mut filters = filters;
                        filters.day = Local::now().format("%Y-%m-%d").to_string();
                        filters.kind.clear();
                        filters.query.clear();
                        filters.offset = 0;
                        let day = filters.day.clone();
                        let accepted = request.action.is_none().then(|| request.message.clone());
                        show_state(
                            weak.clone(),
                            thread_generation.clone(),
                            generation,
                            move |ui| {
                                if accepted
                                    .as_ref()
                                    .is_some_and(|message| ui.get_composer() == message.as_str())
                                {
                                    ui.set_composer("".into());
                                }
                                ui.set_selected_day(day.into());
                                ui.set_kind_index(0);
                                ui.set_search_query("".into());
                                ui.clear_feed();
                                ui.set_sending(false);
                                ui.set_page("hub".into());
                                ui.scroll_to_bottom();
                            },
                        );
                        refresh(
                            &runtime,
                            &current,
                            weak.clone(),
                            thread_generation.clone(),
                            generation,
                            filters,
                            RefreshMode {
                                reset: true,
                                force_tail: true,
                                ..RefreshMode::default()
                            },
                        )
                    })();
                    (generation, outcome)
                }
                Command::Save {
                    generation,
                    url,
                    token,
                    worker,
                    filters,
                } => {
                    current.hub_url = url;
                    current.target_worker = worker;
                    let outcome = (|| -> Result<()> {
                        current.save(&config_path, &token)?;
                        *thread_config
                            .lock()
                            .map_err(|_| anyhow::anyhow!("configuration unavailable"))? =
                            current.clone();
                        refresh(
                            &runtime,
                            &current,
                            weak.clone(),
                            thread_generation.clone(),
                            generation,
                            filters,
                            RefreshMode {
                                reset: true,
                                force_tail: true,
                                ..RefreshMode::default()
                            },
                        )
                    })();
                    (generation, outcome)
                }
            };
            if result.is_err() {
                show_state(weak.clone(), thread_generation.clone(), expected, |ui| {
                    ui.set_busy(false);
                    ui.set_sending(false);
                    ui.set_online(false);
                    ui.set_connection_state("Hub indisponível".into());
                    ui.set_status_message("Não foi possível concluir. Verifique conexão, credencial e permissões; o histórico existente foi preservado.".into());
                });
            }
        }
    });
    for callback in 0..2 {
        let tx = sender.clone();
        let weak = ui.as_weak();
        let counter = generation.clone();
        let handler = move || {
            if let Some(ui) = weak.upgrade() {
                if ui.get_sending() {
                    ui.set_status_message(
                        "Aguarde a confirmação do envio antes de alterar os filtros.".into(),
                    );
                    return;
                }
                let expected = counter.fetch_add(1, Ordering::SeqCst) + 1;
                enqueue(
                    &tx,
                    Command::Refresh {
                        generation: expected,
                        filters: filters(&ui, 0),
                        mode: RefreshMode {
                            reset: callback == 1,
                            force_tail: callback == 1,
                            ..RefreshMode::default()
                        },
                    },
                    &ui,
                );
            }
        };
        if callback == 0 {
            ui.on_refresh(handler);
        } else {
            ui.on_filters_changed(handler);
        }
    }
    {
        let tx = sender.clone();
        let weak = ui.as_weak();
        let counter = generation.clone();
        ui.on_hub_opened(move || {
            if let Some(ui) = weak.upgrade() {
                ui.scroll_to_bottom();
                if ui.get_busy() || ui.get_sending() {
                    return;
                }
                let expected = counter.fetch_add(1, Ordering::SeqCst) + 1;
                enqueue(
                    &tx,
                    Command::Refresh {
                        generation: expected,
                        filters: filters(&ui, 0),
                        mode: RefreshMode {
                            force_tail: true,
                            ..RefreshMode::default()
                        },
                    },
                    &ui,
                );
            }
        });
    }
    {
        let tx = sender.clone();
        let weak = ui.as_weak();
        let counter = generation.clone();
        ui.on_load_more(move || {
            if let Some(ui) = weak.upgrade() {
                if ui.get_busy() || !ui.get_has_more() {
                    return;
                }
                if !ui.feed_filter_matches(&filters(&ui, 0).cache_key()) {
                    ui.set_status_message(
                        "Aplique os filtros antes de carregar mensagens anteriores.".into(),
                    );
                    return;
                }
                let expected = counter.fetch_add(1, Ordering::SeqCst) + 1;
                enqueue(
                    &tx,
                    Command::Refresh {
                        generation: expected,
                        filters: filters(&ui, ui.feed_next_offset()),
                        mode: RefreshMode {
                            append: true,
                            ..RefreshMode::default()
                        },
                    },
                    &ui,
                );
            }
        });
    }
    {
        let tx = sender.clone();
        let weak = ui.as_weak();
        let counter = generation.clone();
        ui.on_save_connection(move || {
            if let Some(ui) = weak.upgrade() {
                if ui.get_sending() {
                    ui.set_status_message(
                        "Aguarde a confirmação do envio antes de alterar a conexão.".into(),
                    );
                    return;
                }
                let expected = counter.fetch_add(1, Ordering::SeqCst) + 1;
                let command = Command::Save {
                    generation: expected,
                    url: ui.get_connection_url().to_string(),
                    token: ui.get_connection_token().to_string(),
                    worker: ui.get_worker_target().to_string(),
                    filters: filters(&ui, 0),
                };
                ui.set_connection_token("".into());
                enqueue(&tx, command, &ui);
            }
        });
    }
    {
        let tx = sender.clone();
        let weak = ui.as_weak();
        let counter = generation.clone();
        ui.on_send_message(move || {
            if let Some(ui) = weak.upgrade() {
                if ui.get_sending() {
                    return;
                }
                let message = ui.get_composer().to_string();
                if message.trim().is_empty() {
                    return;
                }
                let expected = counter.fetch_add(1, Ordering::SeqCst) + 1;
                let command = Command::Send {
                    generation: expected,
                    request: ChatRequest {
                        message,
                        idempotency_key: uuid::Uuid::new_v4().to_string(),
                        target_worker: None,
                        action: None,
                    },
                    filters: filters(&ui, 0),
                };
                enqueue(&tx, command, &ui);
            }
        });
    }
    {
        let tx = sender.clone();
        let weak = ui.as_weak();
        let counter = generation.clone();
        ui.on_diagnostic(move || {
            if let Some(ui) = weak.upgrade() {
                if ui.get_sending() {
                    return;
                }
                let expected = counter.fetch_add(1, Ordering::SeqCst) + 1;
                enqueue(
                    &tx,
                    Command::Send {
                        generation: expected,
                        request: ChatRequest {
                            message: "Execute o diagnóstico autorizado do computador selecionado."
                                .into(),
                            idempotency_key: uuid::Uuid::new_v4().to_string(),
                            target_worker: Some(ui.get_worker_target().to_string()),
                            action: Some("diagnostic".into()),
                        },
                        filters: filters(&ui, 0),
                    },
                    &ui,
                );
            }
        });
    }
    {
        let tx = sender.clone();
        let weak = ui.as_weak();
        let counter = generation.clone();
        ui.on_service_action(move |action| {
            if let Some(ui) = weak.upgrade() {
                if ui.get_sending() {
                    return;
                }
                if !matches!(action.as_str(), "youtube_profiles" | "android_devices") {
                    return;
                }
                let message = if action == "youtube_profiles" {
                    "Liste os perfis disponíveis do YouTube."
                } else {
                    "Liste os dispositivos Android autorizados no computador."
                };
                let expected = counter.fetch_add(1, Ordering::SeqCst) + 1;
                enqueue(
                    &tx,
                    Command::Send {
                        generation: expected,
                        request: ChatRequest {
                            message: message.into(),
                            idempotency_key: uuid::Uuid::new_v4().to_string(),
                            target_worker: Some(ui.get_worker_target().to_string()),
                            action: Some(action.to_string()),
                        },
                        filters: filters(&ui, 0),
                    },
                    &ui,
                );
            }
        });
    }
    {
        let config = shared.clone();
        let weak = ui.as_weak();
        ui.on_open_artifact(move |id|{if let Some(ui)=weak.upgrade(){let result=config.lock().ok().and_then(|c|zaturo_desktop::image::received_png(&c.inbox_root,&id).ok()).and_then(|path|slint::Image::load_from_path(&path).ok());if let Some(image)=result {ui.set_preview_image(image);ui.set_preview_open(true);}else{ui.set_status_message("Imagem aguardando entrega criptografada na caixa de entrada, ou formato não suportado. Telas sensíveis usam visualização separada.".into());}}});
    }
    let timer = slint::Timer::default();
    {
        let tx = sender.clone();
        let weak = ui.as_weak();
        let counter = generation.clone();
        timer.start(
            slint::TimerMode::Repeated,
            Duration::from_secs(5),
            move || {
                if let Some(ui) = weak.upgrade()
                    && ui.window().is_visible()
                    && !ui.window().is_minimized()
                    && ui.get_online()
                    && !ui.get_busy()
                    && !ui.get_sending()
                    && ui.get_page() == "hub"
                    && ui.get_at_bottom()
                    && ui.feed_filter_matches(&filters(&ui, 0).cache_key())
                {
                    let expected = counter.load(Ordering::SeqCst);
                    enqueue(
                        &tx,
                        Command::Refresh {
                            generation: expected,
                            filters: filters(&ui, 0),
                            mode: RefreshMode::default(),
                        },
                        &ui,
                    );
                }
            },
        );
    }
    if shared.lock().is_ok_and(|c| c.token().is_ok()) {
        ui.invoke_refresh();
    } else {
        ui.set_page("settings".into());
    }
    let private_viewer = std::rc::Rc::new(zaturo_desktop::private_view::PrivateViewer::new(
        shared.clone(),
        ui.as_weak(),
    )?);
    {
        let viewer = private_viewer.clone();
        let weak = ui.as_weak();
        ui.on_start_private(move || {
            if let Some(ui) = weak.upgrade() {
                if !ui.get_private_confirmed() {
                    return;
                }
                let watch = ui
                    .get_private_watch()
                    .parse::<u32>()
                    .ok()
                    .filter(|n| *n <= 120);
                let Some(watch) = watch else {
                    ui.set_private_status("Use uma espera entre 0 e 120 segundos.".into());
                    return;
                };
                let device = ui.get_private_device().to_string();
                if zaturo_core::validate_identifier(&device).is_err() {
                    ui.set_private_status("Informe um dispositivo Android registrado.".into());
                    return;
                }
                viewer.close();
                ui.set_private_confirmed(false);
                ui.set_private_image(slint::Image::default());
                ui.set_private_image_visible(false);
                ui.set_private_running(true);
                if !viewer.start(device, watch) {
                    ui.set_private_running(false);
                    ui.set_private_status("Aguarde a operação privada atual encerrar.".into());
                }
            }
        });
    }
    {
        let viewer = private_viewer.clone();
        let weak = ui.as_weak();
        ui.on_close_private(move || {
            viewer.close();
            if let Some(ui) = weak.upgrade() {
                ui.set_private_image(slint::Image::default());
                ui.set_private_image_visible(false);
                ui.set_private_running(false);
                ui.set_private_confirmed(false);
                ui.set_private_status("Sessão privada encerrada; nenhuma imagem foi salva.".into());
            }
        });
    }
    {
        use slint::winit_030::{EventResult, WinitWindowAccessor, winit::event::WindowEvent};
        let viewer = private_viewer.clone();
        let weak = ui.as_weak();
        ui.window().on_winit_window_event(move |_, event| {
            if matches!(
                event,
                WindowEvent::Focused(false)
                    | WindowEvent::Occluded(true)
                    | WindowEvent::CloseRequested
                    | WindowEvent::Destroyed
            ) {
                viewer.close();
                if let Some(ui) = weak.upgrade() {
                    ui.set_private_image(slint::Image::default());
                    ui.set_private_image_visible(false);
                    ui.set_private_running(false);
                    ui.set_private_confirmed(false);
                }
            }
            EventResult::Propagate
        });
    }
    let private_timer = slint::Timer::default();
    {
        let viewer = private_viewer.clone();
        let weak = ui.as_weak();
        private_timer.start(
            slint::TimerMode::Repeated,
            Duration::from_secs(1),
            move || {
                if let Some(ui) = weak.upgrade() {
                    if ui.get_private_running()
                        && (ui.get_page() != "private"
                            || !ui.window().is_visible()
                            || ui.window().is_minimized())
                    {
                        viewer.close();
                        ui.set_private_image(slint::Image::default());
                        ui.set_private_image_visible(false);
                        ui.set_private_running(false);
                    } else if viewer.tick() {
                        ui.set_private_image(slint::Image::default());
                        ui.set_private_image_visible(false);
                        ui.set_private_running(false);
                        ui.set_private_status(
                            "Sessão expirada; a imagem foi removida da interface.".into(),
                        );
                    }
                }
            },
        );
    }
    if let Some(page) = args.page {
        ui.set_page(page.into());
    }
    let quit = slint::Timer::default();
    if let Some(seconds) = args.quit_after_seconds {
        quit.start(
            slint::TimerMode::SingleShot,
            Duration::from_secs(seconds),
            || {
                let _ = slint::quit_event_loop();
            },
        );
    }
    ui.run()?;
    private_viewer.close();
    private_timer.stop();
    drop(private_timer);
    closing.store(true, Ordering::SeqCst);
    let _ = sender.try_send(Command::Quit);
    drop(ui);
    let _ = network.join();
    Ok(())
}
