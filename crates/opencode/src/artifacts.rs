//! Durable opaque media evidence, available only for this executor's generation operation.
use super::{OpenCodeConfig, private_file, state};
use anyhow::{Result, ensure};
use zaturo_core::VideoArtifactEvidence;
use zaturo_youtube::YoutubeClient;
pub async fn inspect_artifacts(
    config: &OpenCodeConfig,
    operation: &str,
    youtube: &YoutubeClient,
) -> Result<Vec<VideoArtifactEvidence>> {
    let directory = state::directory(&config.state_root, operation)?;
    let intent: state::Intent =
        serde_json::from_slice(&private_file(&directory.join("intent.json"), 16384)?)?;
    ensure!(
        intent.order.operation_id == operation
            && intent.executable_sha256 == config.executable_sha256,
        "executor intent differs"
    );
    let (channel, count) = intent
        .order
        .generation()
        .ok_or_else(|| anyhow::anyhow!("generation order required"))?;
    let receipt: state::Receipt =
        serde_json::from_slice(&private_file(&directory.join("receipt.json"), 16384)?)?;
    ensure!(
        receipt
            .proof
            .is_some_and(|proof| proof.validates(operation, channel, count)),
        "execution proof missing"
    );
    let status = youtube.status(operation).await?;
    ensure!(
        status.channel == channel && status.count == count && status.mode == "generate",
        "generation binding differs"
    );
    let path = directory.join("artifacts.json");
    if path.exists() {
        let previous: Vec<VideoArtifactEvidence> =
            serde_json::from_slice(&private_file(&path, 65536)?)?;
        ensure!(
            youtube.artifacts_current(operation, &previous).await?,
            "cached artifact bytes differ; inspection repair required"
        );
        return Ok(previous);
    }
    let proofs = youtube.inspect_generated(operation).await?;
    ensure!(
        proofs.len() == count as usize && proofs.iter().all(|p| p.validates(operation)),
        "verified generation artifacts required"
    );
    state::atomic(&path, &serde_json::to_vec(&proofs)?)?;
    Ok(proofs)
}
