use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use zaturo_worker::config::WorkerConfig;
#[derive(Parser)]
#[command(
    version,
    about = "ZaturoBot outbound worker and encrypted device file client"
)]
struct Cli {
    #[arg(long, global = true, default_value = "config/worker.example.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Connect,
    /// Generate a local age identity; prints only its public recipient.
    Identity {
        output: PathBuf,
    },
    Files {
        #[command(subcommand)]
        command: Files,
    },
    Android {
        #[command(subcommand)]
        command: Android,
    },
    Task {
        #[command(subcommand)]
        command: Task,
    },
    Commerce {
        #[command(subcommand)]
        command: Commerce,
    },
}
#[derive(Subcommand)]
enum Files {
    Peers,
    Send {
        path: PathBuf,
        #[arg(long)]
        to: String,
    },
    Resume {
        key: String,
    },
    List {
        #[arg(long)]
        outbox: bool,
    },
    Status {
        id: String,
    },
    Receive {
        id: String,
    },
    Sync,
}
#[derive(Subcommand)]
enum Android {
    Devices,
    Status { id: String },
}
#[derive(Subcommand)]
enum Task {
    Diagnostic,
    Show { id: String },
}
#[derive(Subcommand)]
enum Commerce {
    IngestMl {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value = "hardware-e-perifericos")]
        category: String,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        idempotency_key: Option<String>,
        #[arg(long)]
        source_base_dir: Option<PathBuf>,
        #[arg(long)]
        publish_telegram: bool,
        #[arg(long)]
        dry_run_telegram: bool,
        #[arg(long)]
        whatsapp_drafts_out: Option<PathBuf>,
    },
    TelegramPublish {
        #[arg(long)]
        channel: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        whatsapp_drafts_out: Option<PathBuf>,
    },
    TelegramPoll {
        #[arg(long)]
        offset: Option<i64>,
        #[arg(long, default_value_t = 0)]
        timeout_secs: u64,
    },
    TelegramDaemon {
        #[arg(long, default_value_t = 15)]
        poll_timeout_secs: u64,
        #[arg(long, default_value_t = 3)]
        tick_seconds: u64,
    },
    WhatsappDrafts {
        #[arg(long)]
        channel: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        output: Option<PathBuf>,
    },
    WatchMl {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        state_file: PathBuf,
        #[arg(long, default_value = "hardware-e-perifericos")]
        category: String,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        source_base_dir: Option<PathBuf>,
        #[arg(long)]
        whatsapp_drafts_out: Option<PathBuf>,
        #[arg(long)]
        reference_out: Option<PathBuf>,
        #[arg(long, default_value_t = 5)]
        poll_seconds: u64,
        #[arg(long)]
        once: bool,
    },
    SyncReference {
        #[arg(long)]
        output: PathBuf,
    },
    QueryReference {
        #[arg(long)]
        file: PathBuf,
        #[arg(long)]
        query: Option<String>,
        #[arg(long)]
        product_id: Option<String>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
}
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let cli = Cli::parse();
    if let Command::Identity { output } = &cli.command {
        if let Some(parent) = output.parent() {
            zaturo_transfer::blob::private_directory(parent)?;
        }
        println!("{}", zaturo_transfer::crypto::generate_identity(output)?);
        return Ok(());
    }
    if let Command::Commerce {
        command:
            Commerce::QueryReference {
                file,
                query,
                product_id,
                category,
                limit,
            },
    } = &cli.command
    {
        let matches = zaturo_worker::mercadolivre::load_and_query_reference_catalog(
            file,
            query.as_deref(),
            product_id.as_deref(),
            category.as_deref(),
            *limit,
        )?;
        println!("{}", serde_json::to_string_pretty(&matches)?);
        return Ok(());
    }
    let config = WorkerConfig::load(&cli.config)?;
    let token = std::env::var("ZATURO_WORKER_TOKEN").context("ZATURO_WORKER_TOKEN is required")?;
    let client = zaturo_worker::files::client(&config, &token)?;
    match cli.command {
        Command::Connect => zaturo_worker::run(config, token).await?,
        Command::Identity { .. } => unreachable!(),
        Command::Commerce { command } => match command {
            Commerce::IngestMl {
                input,
                category,
                channel,
                idempotency_key,
                source_base_dir,
                publish_telegram,
                dry_run_telegram,
                whatsapp_drafts_out,
            } => {
                let resolved_tg = config
                    .telegram
                    .as_ref()
                    .map(|t| t.with_env_overrides())
                    .transpose()?;
                let target_channel = channel
                    .or_else(|| resolved_tg.as_ref().map(|t| t.default_channel_id.clone()))
                    .unwrap_or_else(|| "@procurali".to_owned());
                let options = zaturo_worker::mercadolivre::BatchIngestOptions {
                    worker_id: config.worker_id.clone(),
                    default_category_slug: category,
                    channel_id: target_channel.clone(),
                    now_ms: zaturo_worker::now_ms(),
                };
                let report = zaturo_worker::mercadolivre::ingest_file_to_hub(
                    &config,
                    &token,
                    &input,
                    &options,
                    idempotency_key.as_deref(),
                    source_base_dir.as_deref(),
                )
                .await?;
                let should_publish_telegram = publish_telegram
                    || dry_run_telegram
                    || resolved_tg
                        .as_ref()
                        .is_some_and(|t| t.auto_publish_on_ingest || t.allow_live_publish);
                let mut telegram_report = None;
                if should_publish_telegram {
                    let tg_cfg = config.telegram.clone().context(
                        "[telegram] configuration section is required for Telegram publishing",
                    )?;
                    let tg_client =
                        zaturo_worker::telegram::TelegramBotClient::from_config(tg_cfg)?;
                    let tg_out = tg_client
                        .publish_queued_from_hub(
                            &config.hub_url,
                            &token,
                            Some(&target_channel),
                            50,
                            dry_run_telegram,
                            zaturo_worker::now_ms(),
                        )
                        .await?;
                    if let Some(out_path) = &whatsapp_drafts_out {
                        write_json_output(out_path, &tg_out.whatsapp_package)?;
                    }
                    telegram_report = Some(tg_out);
                } else if let Some(out_path) = &whatsapp_drafts_out {
                    let pkg = fetch_whatsapp_package(
                        &config,
                        &token,
                        &target_channel,
                        resolved_tg.as_ref(),
                        50,
                    )
                    .await?;
                    write_json_output(out_path, &pkg)?;
                }
                if let Some(tg) = telegram_report {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "ingest": report,
                            "telegram": tg,
                        }))?
                    );
                } else {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                }
            }
            Commerce::TelegramPublish {
                channel,
                limit,
                dry_run,
                whatsapp_drafts_out,
            } => {
                let tg_cfg = config
                    .telegram
                    .clone()
                    .context("[telegram] configuration section is required")?;
                let tg_client = zaturo_worker::telegram::TelegramBotClient::from_config(tg_cfg)?;
                let report = tg_client
                    .publish_queued_from_hub(
                        &config.hub_url,
                        &token,
                        channel.as_deref(),
                        limit,
                        dry_run,
                        zaturo_worker::now_ms(),
                    )
                    .await?;
                if let Some(out_path) = &whatsapp_drafts_out {
                    write_json_output(out_path, &report.whatsapp_package)?;
                }
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            Commerce::TelegramPoll {
                offset,
                timeout_secs,
            } => {
                let tg_cfg = config
                    .telegram
                    .clone()
                    .context("[telegram] configuration section is required")?;
                let tg_client = zaturo_worker::telegram::TelegramBotClient::from_config(tg_cfg)?;
                let report = tg_client
                    .poll_commands_once(&config.hub_url, &token, offset, timeout_secs)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
            Commerce::TelegramDaemon {
                poll_timeout_secs,
                tick_seconds,
            } => {
                let tg_cfg = config
                    .telegram
                    .clone()
                    .context("[telegram] configuration section is required")?;
                let tg_client = zaturo_worker::telegram::TelegramBotClient::from_config(tg_cfg)?;
                tg_client
                    .run_telegram_daemon_cycles(
                        &config.hub_url,
                        &token,
                        poll_timeout_secs,
                        std::time::Duration::from_secs(tick_seconds.max(1)),
                        None,
                    )
                    .await?;
            }
            Commerce::WhatsappDrafts {
                channel,
                limit,
                output,
            } => {
                let resolved_tg = config
                    .telegram
                    .as_ref()
                    .map(|t| t.with_env_overrides())
                    .transpose()?;
                let target_channel = channel
                    .or_else(|| resolved_tg.as_ref().map(|t| t.default_channel_id.clone()))
                    .unwrap_or_else(|| "@procurali".to_owned());
                let pkg = fetch_whatsapp_package(
                    &config,
                    &token,
                    &target_channel,
                    resolved_tg.as_ref(),
                    limit,
                )
                .await?;
                if let Some(out_path) = output {
                    write_json_output(&out_path, &pkg)?;
                    println!("Wrote WhatsApp assisted drafts to {}", out_path.display());
                } else {
                    println!("{}", serde_json::to_string_pretty(&pkg)?);
                }
            }
            Commerce::WatchMl {
                input,
                state_file,
                category,
                channel,
                source_base_dir,
                whatsapp_drafts_out,
                reference_out,
                poll_seconds,
                once,
            } => {
                let resolved_tg = config
                    .telegram
                    .as_ref()
                    .map(|t| t.with_env_overrides())
                    .transpose()?;
                let target_channel = channel
                    .or_else(|| resolved_tg.as_ref().map(|t| t.default_channel_id.clone()))
                    .unwrap_or_else(|| "@procurali".to_owned());
                let poll_interval = std::time::Duration::from_secs(poll_seconds.max(1));
                loop {
                    if input.exists() {
                        let options = zaturo_worker::mercadolivre::BatchIngestOptions {
                            worker_id: config.worker_id.clone(),
                            default_category_slug: category.clone(),
                            channel_id: target_channel.clone(),
                            now_ms: zaturo_worker::now_ms(),
                        };
                        match zaturo_worker::mercadolivre::ingest_file_if_changed(
                            &config,
                            &token,
                            &input,
                            &state_file,
                            &options,
                            source_base_dir.as_deref(),
                        )
                        .await
                        {
                            Ok(Some(report)) => {
                                if let Some(out_path) = &whatsapp_drafts_out {
                                    let pkg = fetch_whatsapp_package(
                                        &config,
                                        &token,
                                        &target_channel,
                                        resolved_tg.as_ref(),
                                        50,
                                    )
                                    .await?;
                                    write_json_output(out_path, &pkg)?;
                                }
                                let mut ref_summary = None;
                                if let Some(ref_path) = &reference_out {
                                    ref_summary = Some(
                                        zaturo_worker::mercadolivre::sync_reference_catalog(
                                            &config.hub_url,
                                            &token,
                                            ref_path,
                                        )
                                        .await?,
                                    );
                                }
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&serde_json::json!({
                                        "status": "ingested",
                                        "report": report,
                                        "reference_sync": ref_summary,
                                    }))?
                                );
                            }
                            Ok(None) => {
                                if let Some(out_path) = &whatsapp_drafts_out
                                    && once
                                    && !out_path.exists()
                                {
                                    let pkg = fetch_whatsapp_package(
                                        &config,
                                        &token,
                                        &target_channel,
                                        resolved_tg.as_ref(),
                                        50,
                                    )
                                    .await?;
                                    write_json_output(out_path, &pkg)?;
                                }
                                let mut ref_summary = None;
                                if let Some(ref_path) = &reference_out
                                    && (once || !ref_path.exists())
                                {
                                    ref_summary = Some(
                                        zaturo_worker::mercadolivre::sync_reference_catalog(
                                            &config.hub_url,
                                            &token,
                                            ref_path,
                                        )
                                        .await?,
                                    );
                                }
                                if once {
                                    println!(
                                        "{}",
                                        serde_json::to_string_pretty(&serde_json::json!({
                                            "status": "unchanged",
                                            "input": input.display().to_string(),
                                            "reference_sync": ref_summary,
                                        }))?
                                    );
                                }
                            }
                            Err(err) if !once => {
                                tracing::warn!(error = %err, "watch_ml_cycle_failed");
                            }
                            Err(err) => return Err(err),
                        }
                    } else if once {
                        anyhow::bail!("input batch file does not exist: {}", input.display());
                    }
                    if once {
                        break;
                    }
                    tokio::select! {
                        _ = tokio::signal::ctrl_c() => break,
                        _ = tokio::time::sleep(poll_interval) => {}
                    }
                }
            }
            Commerce::SyncReference { output } => {
                let summary = zaturo_worker::mercadolivre::sync_reference_catalog(
                    &config.hub_url,
                    &token,
                    &output,
                )
                .await?;
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
            Commerce::QueryReference { .. } => unreachable!(),
        },
        Command::Android { command } => {
            let android = config
                .android
                .as_ref()
                .context("[android] configuration is required")?;
            let adb = zaturo_android::AdbClient::new(&android.executable)?;
            match command {
                Android::Devices => println!(
                    "{}",
                    serde_json::to_string_pretty(&adb.discover(zaturo_worker::now_ms()).await?)?
                ),
                Android::Status { id } => {
                    let device = android
                        .devices
                        .iter()
                        .find(|d| d.device_id == id)
                        .context("Android device ID is not registered")?;
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &adb.inspect(device, zaturo_worker::now_ms()).await?
                        )?
                    );
                }
            }
        }
        Command::Task { command } => match command {
            Task::Diagnostic => {
                let request = zaturo_core::NewTask {
                    service: None,
                    kind: zaturo_core::TaskKind::Diagnostic,
                    request: "Run the registered read-only diagnostic on this device.".into(),
                    target_worker: config.worker_id.clone(),
                    idempotency_key: format!(
                        "diagnostic-{}-{}",
                        zaturo_worker::now_ms(),
                        std::process::id()
                    ),
                    priority: 0,
                    max_attempts: 2,
                    timeout_seconds: 120,
                    data_policy: None,
                };
                let task: zaturo_core::Task = client.post("/v1/device/tasks", &request).await?;
                println!("{}", serde_json::to_string_pretty(&task)?);
            }
            Task::Show { id } => {
                zaturo_transfer::blob::object_name(&id)?;
                let task: zaturo_core::Task = client.get(&format!("/v1/device/tasks/{id}")).await?;
                println!("{}", serde_json::to_string_pretty(&task)?);
            }
        },
        Command::Files { command } => {
            if let Files::Peers = command {
                println!("{}", serde_json::to_string_pretty(&client.peers().await?)?);
                return Ok(());
            }
            let files = config
                .files
                .as_ref()
                .context("[files] identity, cache and inbox are required")?;
            match command {
                Files::Peers => unreachable!(),
                Files::Send { path, to } => {
                    let key = client
                        .prepare(&path, &to, &files.identity, &files.cache)
                        .await?;
                    eprintln!("Resume key: {key}");
                    let transfer = client.upload(&key, &files.cache).await?;
                    println!("{}", serde_json::to_string_pretty(&transfer)?);
                }
                Files::Resume { key } => println!(
                    "{}",
                    serde_json::to_string_pretty(&client.upload(&key, &files.cache).await?)?
                ),
                Files::List { outbox } => println!(
                    "{}",
                    serde_json::to_string_pretty(&client.list(!outbox, false).await?)?
                ),
                Files::Status { id } => println!(
                    "{}",
                    serde_json::to_string_pretty(&client.status(&id).await?)?
                ),
                Files::Receive { id } => println!(
                    "{}",
                    client
                        .receive(&id, &files.identity, &files.cache, &files.inbox)
                        .await?
                        .display()
                ),
                Files::Sync => {
                    let report = client
                        .sync(&files.identity, &files.cache, &files.inbox)
                        .await?;
                    for path in report.received {
                        println!("{}", path.display());
                    }
                    ensure!(
                        report.failed == 0,
                        "{} inbox items require explicit retry",
                        report.failed
                    );
                }
            }
        }
    }
    Ok(())
}

async fn fetch_whatsapp_package(
    config: &WorkerConfig,
    token: &str,
    channel: &str,
    resolved_tg: Option<&zaturo_worker::telegram::TelegramBotConfig>,
    limit: u32,
) -> Result<zaturo_core::WhatsappAssistedPackage> {
    let items = zaturo_worker::telegram::fetch_hub_queue_with_token(
        &config.hub_url,
        token,
        Some(channel),
        None,
        limit,
    )
    .await?;
    let site_base_url = resolved_tg
        .and_then(|t| t.site_base_url.as_deref())
        .or_else(|| config.minio.as_ref().map(|m| m.public_base_url.as_str()));
    Ok(zaturo_core::build_whatsapp_assisted_package(
        &items,
        channel,
        site_base_url,
        zaturo_worker::now_ms(),
    ))
}

fn write_json_output<T: serde::Serialize>(path: &std::path::Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}
