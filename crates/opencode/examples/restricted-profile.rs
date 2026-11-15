//! Render a private profile for metadata-only preflight. This example never starts a model or tool.
use std::io::Read;
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    order: zaturo_core::OpenCodeWorkOrder,
    origin: String,
    nonce: String,
}
fn main() -> anyhow::Result<()> {
    let mut raw = Vec::new();
    std::io::stdin().take(16385).read_to_end(&mut raw)?;
    anyhow::ensure!(raw.len() <= 16384, "bounded profile input required");
    let value: Input = serde_json::from_slice(&raw)?;
    let profile =
        zaturo_opencode::restricted_configuration(&value.order, &value.origin, &value.nonce)?;
    println!("{}", serde_json::to_string_pretty(&profile)?);
    Ok(())
}
