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
