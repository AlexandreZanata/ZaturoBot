use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::{path::PathBuf, time::Duration};
use zaturo_core::NewTask;
use zaturo_hub::{AppState, config::HubConfig};

#[derive(Parser)]
#[command(version, about = "Private ZaturoBot control plane")]
struct Cli {
    #[arg(long, global = true, default_value = "config/hub.example.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Serve,
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    Workers {
        #[command(subcommand)]
        command: WorkerCommand,
    },
    Commerce {
        #[command(subcommand)]
        command: CommerceCommand,
    },
}
#[derive(Subcommand)]
enum WorkerCommand {
    List,
}
#[derive(Subcommand)]
enum CommerceCommand {
    SiteCatalog {
        #[arg(long)]
        output: Option<PathBuf>,
    },
    TelegramQueue {
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        limit: Option<u32>,
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
#[derive(Subcommand)]
enum TaskCommand {
    Create {
        #[arg(long, default_value = "diagnostic")]
        kind: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        idempotency_key: Option<String>,
        #[arg(long, default_value = "Run the approved read-only diagnostic.")]
        request: String,
        #[arg(long)]
        data_policy: Option<String>,
        #[arg(long)]
        service_request: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        limit: Option<u32>,
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    Show {
        id: String,
    },
    Cancel {
        id: String,
    },
    Pause {
        id: String,
    },
    Resume {
        id: String,
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
    let config = HubConfig::load(&cli.config)?;
    let owner = std::env::var("ZATURO_OWNER_TOKEN").context("ZATURO_OWNER_TOKEN is required")?;
    zaturo_security::Credential::new(&owner)?;
    if matches!(cli.command, Command::Serve) {
        let device =
            std::env::var("ZATURO_WORKER_TOKEN").context("ZATURO_WORKER_TOKEN is required")?;
        return zaturo_hub::serve(AppState::new(config, &owner, &device)?).await;
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let base = config.client_url.trim_end_matches('/');
    let get = |path: String| client.get(format!("{base}{path}")).bearer_auth(&owner);
    let post = |path: String| client.post(format!("{base}{path}")).bearer_auth(&owner);
    let response = match cli.command {
        Command::Serve => unreachable!(),
        Command::Workers { .. } => {
            get(format!("/v1/workers?limit={}", config.max_page_size))
                .send()
                .await?
        }
        Command::Commerce { command } => match command {
            CommerceCommand::SiteCatalog { output } => {
                let response = get("/v1/commerce/mercadolivre/site-catalog".into())
                    .send()
                    .await?;
                if let Some(path) = output {
                    let value: serde_json::Value = response.error_for_status()?.json().await?;
                    if let Some(parent) = path.parent()
                        && !parent.as_os_str().is_empty()
                    {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&path, serde_json::to_string_pretty(&value)?)?;
                    println!("Wrote catalog snapshot to {}", path.display());
                    return Ok(());
                }
                response
            }
            CommerceCommand::TelegramQueue {
                channel,
                status,
                limit,
            } => {
                let mut params = Vec::new();
                if let Some(channel_id) = channel {
                    params.push(("channel_id", channel_id));
                }
                if let Some(status) = status {
                    params.push(("status", status));
                }
                if let Some(limit) = limit {
                    params.push(("limit", limit.to_string()));
                }
                get("/v1/commerce/mercadolivre/telegram-queue".into())
                    .query(&params)
                    .send()
                    .await?
            }
            CommerceCommand::WhatsappDrafts {
                channel,
                limit,
                output,
            } => {
                let items: Vec<zaturo_core::TelegramOfferQueueItem> =
                    get("/v1/commerce/mercadolivre/telegram-queue".into())
                        .query(&[
                            ("channel_id", channel.as_str()),
                            ("limit", &limit.clamp(1, 100).to_string()),
                        ])
                        .send()
                        .await?
                        .error_for_status()?
                        .json()
                        .await?;
                let pkg = zaturo_core::build_whatsapp_assisted_package(
                    &items,
                    &channel,
                    None,
                    zaturo_hub::now_ms(),
                );
                let json_str = serde_json::to_string_pretty(&pkg)?;
                if let Some(path) = output {
                    if let Some(parent) = path.parent()
                        && !parent.as_os_str().is_empty()
                    {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(&path, json_str)?;
                    println!("Wrote WhatsApp assisted drafts to {}", path.display());
                } else {
                    println!("{json_str}");
                }
                return Ok(());
            }
        },
        Command::Task { command } => match command {
            TaskCommand::Create {
                kind,
                target,
                idempotency_key,
                request,
                data_policy,
                service_request,
            } => {
                let key = idempotency_key.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let mut task = match (kind.as_str(), data_policy.as_deref()) {
                    ("diagnostic", None) => NewTask::diagnostic(target, key),
                    ("assisted_diagnostic", Some("public-coding")) => {
                        NewTask::assisted_diagnostic(target, key)
                    }
                    ("service", None) => {
                        let path = service_request.as_ref().context(
                            "service requires --service-request PATH to a typed JSON request",
                        )?;
                        anyhow::ensure!(
                            std::fs::metadata(path)?.len() <= 4096,
                            "service request exceeds limit"
                        );
                        let request: zaturo_core::ServiceRequest = serde_json::from_str(
                            &std::fs::read_to_string(path)?,
                        )
                        .map_err(|_| {
                            anyhow::anyhow!("invalid typed service request; values are not logged")
                        })?;
                        let mut task = NewTask::diagnostic(target, key);
                        task.kind = zaturo_core::TaskKind::Service;
                        task.max_attempts = 1;
                        task.service = Some(request);
                        task
                    }
                    ("assisted_diagnostic", _) => bail!(
                        "assisted_diagnostic requires --data-policy public-coding; submit public coding context only"
                    ),
                    _ => bail!("unsupported task kind or disclosure policy"),
                };
                anyhow::ensure!(
                    kind == "service" || service_request.is_none(),
                    "--service-request is only valid for service tasks"
                );
                task.request = request;
                post("/v1/tasks".into()).json(&task).send().await?
            }
            TaskCommand::List { limit, offset } => {
                let limit = limit.unwrap_or(20.min(config.max_page_size));
                get(format!("/v1/tasks?limit={limit}&offset={offset}"))
                    .send()
                    .await?
            }
            TaskCommand::Show { id } => {
                uuid::Uuid::parse_str(&id).context("invalid task ID")?;
                let task: serde_json::Value = get(format!("/v1/tasks/{id}"))
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;
                let events: serde_json::Value = get(format!(
                    "/v1/tasks/{id}/events?limit={}",
                    config.max_page_size
                ))
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &serde_json::json!({"task":task,"events":events})
                    )?
                );
                return Ok(());
            }
            TaskCommand::Cancel { id } => {
                uuid::Uuid::parse_str(&id)?;
                post(format!("/v1/tasks/{id}/cancel")).send().await?
            }
            TaskCommand::Pause { id } => {
                uuid::Uuid::parse_str(&id)?;
                post(format!("/v1/tasks/{id}/pause")).send().await?
            }
            TaskCommand::Resume { id } => {
                uuid::Uuid::parse_str(&id)?;
                post(format!("/v1/tasks/{id}/resume")).send().await?
            }
        },
    };
    let value: serde_json::Value = response.error_for_status()?.json().await?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
