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
        #[arg(long, default_value = "@procurali")]
        channel: String,
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
    WhatsappDrafts {
        #[arg(long, default_value = "@procurali")]
        channel: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
        #[arg(long)]
        output: Option<PathBuf>,
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
                let options = zaturo_worker::mercadolivre::BatchIngestOptions {
                    worker_id: config.worker_id.clone(),
                    default_category_slug: category,
                    channel_id: channel.clone(),
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
                let mut telegram_report = None;
                if publish_telegram || dry_run_telegram {
                    let tg_cfg = config.telegram.clone().context(
                        "[telegram] configuration section is required for Telegram publishing",
                    )?;
                    let tg_client =
                        zaturo_worker::telegram::TelegramBotClient::from_config(tg_cfg)?;
                    let tg_out = tg_client
                        .publish_queued_from_hub(
                            &config.hub_url,
                            &token,
                            Some(&channel),
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
                    let items = zaturo_worker::telegram::fetch_hub_queue_with_token(
                        &config.hub_url,
                        &token,
                        Some(&channel),
                        None,
                        50,
                    )
                    .await?;
                    let pkg = zaturo_core::build_whatsapp_assisted_package(
                        &items,
                        &channel,
                        config
                            .telegram
                            .as_ref()
                            .and_then(|t| t.site_base_url.as_deref()),
                        zaturo_worker::now_ms(),
                    );
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
                    if let Some(error) = &tg.hub_error {
                        anyhow::bail!("Telegram publishing stopped: {error}");
                    }
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
                if let Some(error) = &report.hub_error {
                    anyhow::bail!("Telegram publishing stopped: {error}");
                }
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
            Commerce::WhatsappDrafts {
                channel,
                limit,
                output,
            } => {
                let items = zaturo_worker::telegram::fetch_hub_queue_with_token(
                    &config.hub_url,
                    &token,
                    Some(&channel),
                    None,
                    limit,
                )
                .await?;
                let pkg = zaturo_core::build_whatsapp_assisted_package(
                    &items,
                    &channel,
                    config
                        .telegram
                        .as_ref()
                        .and_then(|t| t.site_base_url.as_deref()),
                    zaturo_worker::now_ms(),
                );
                if let Some(out_path) = output {
                    write_json_output(&out_path, &pkg)?;
                    println!("Wrote WhatsApp assisted drafts to {}", out_path.display());
                } else {
                    println!("{}", serde_json::to_string_pretty(&pkg)?);
                }
            }
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

fn write_json_output<T: serde::Serialize>(path: &std::path::Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}
