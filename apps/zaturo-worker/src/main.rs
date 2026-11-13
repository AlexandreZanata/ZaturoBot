use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use zaturo_worker::config::WorkerConfig;

#[derive(Parser)]
#[command(version, about = "Outbound ZaturoBot diagnostic worker")]
struct Cli {
    #[arg(long, global = true, default_value = "config/worker.example.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Connect,
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
    let config = WorkerConfig::load(&cli.config)?;
    let token = std::env::var("ZATURO_WORKER_TOKEN").context("ZATURO_WORKER_TOKEN is required")?;
    zaturo_worker::run(config, token).await
}
