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
}
#[derive(Subcommand)]
enum WorkerCommand {
    List,
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
        Command::Task { command } => match command {
            TaskCommand::Create {
                kind,
                target,
                idempotency_key,
                request,
                data_policy,
            } => {
                let key = idempotency_key.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                let mut task = match (kind.as_str(), data_policy.as_deref()) {
                    ("diagnostic", None) => NewTask::diagnostic(target, key),
                    ("assisted_diagnostic", Some("public-coding")) => {
                        NewTask::assisted_diagnostic(target, key)
                    }
                    ("assisted_diagnostic", _) => bail!(
                        "assisted_diagnostic requires --data-policy public-coding; submit public coding context only"
                    ),
                    _ => bail!("unsupported task kind or disclosure policy"),
                };
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
