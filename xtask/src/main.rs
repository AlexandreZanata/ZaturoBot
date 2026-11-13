//! Repository validation used unchanged by local development and CI.

mod architecture;
mod contracts;
mod documentation;
mod secrets;

use anyhow::{Context, Result, bail};
use std::{path::Path, process::Command};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("check");
    let locked = args.iter().skip(1).any(|arg| arg == "--locked");
    if args.iter().skip(1).any(|arg| arg != "--locked") {
        bail!("usage: cargo xtask [check|arch|docs|secrets] [--locked]");
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("xtask must be located directly below the repository root")?;
    match command {
        "arch" => architecture::check(root, locked)?,
        "secrets" => secrets::check(root)?,
        "docs" => {
            documentation::check(root)?;
            contracts::check(root)?;
        }
        "check" => {
            architecture::check(root, locked)?;
            documentation::check(root)?;
            contracts::check(root)?;
            secrets::check(root)?;
            run(root, &["fmt", "--all", "--", "--check"])?;
            let lock_arg: &[&str] = if locked { &["--locked"] } else { &[] };
            let mut clippy = vec!["clippy", "--workspace", "--all-targets"];
            clippy.extend_from_slice(lock_arg);
            clippy.extend_from_slice(&["--", "-D", "warnings"]);
            run(root, &clippy)?;
            let mut tests = vec!["test", "--workspace"];
            tests.extend_from_slice(lock_arg);
            run(root, &tests)?;
            let mut docs = vec!["doc", "--workspace", "--no-deps"];
            docs.extend_from_slice(lock_arg);
            run(root, &docs)?;
        }
        _ => bail!("unknown command {command:?}; use check, arch, docs, or secrets"),
    }
    println!("Repository {command} checks passed.");
    Ok(())
}

fn run(root: &Path, args: &[&str]) -> Result<()> {
    println!("Running cargo {}", args.join(" "));
    let status = Command::new("cargo")
        .args(args)
        .current_dir(root)
        .env("RUSTDOCFLAGS", "-D warnings")
        .status()
        .with_context(|| format!("could not run cargo {}", args.join(" ")))?;
    if !status.success() {
        bail!("cargo {} failed with {status}", args.join(" "));
    }
    Ok(())
}
