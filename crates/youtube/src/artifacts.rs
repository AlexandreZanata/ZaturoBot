//! Fixed read-only media inspection after matched generation completion.
use crate::{Error, JobState, YoutubeClient};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};
use zaturo_core::{MediaStreamEvidence, VideoArtifactEvidence};
type Stamp = (u64, u64, u64, i64, i64);
fn stamp(info: &std::fs::Metadata) -> Stamp {
    (
        info.dev(),
        info.ino(),
        info.len(),
        info.mtime(),
        info.mtime_nsec(),
    )
}
fn digest(path: &Path) -> Result<(String, Stamp), Error> {
    if path.ancestors().any(Path::is_symlink) {
        return Err(Error::PolicyDenied);
    }
    let mut file = std::fs::File::open(path).map_err(|_| Error::Unavailable)?;
    let before = file.metadata().map_err(|_| Error::Unavailable)?;
    if !before.is_file()
        || before.permissions().mode() & 0o022 != 0
        || before.len() > 8 * 1024 * 1024 * 1024
    {
        return Err(Error::PolicyDenied);
    }
    let mut digest = Sha256::new();
    let mut block = [0_u8; 65536];
    loop {
        let count = file.read(&mut block).map_err(|_| Error::Unavailable)?;
        if count == 0 {
            break;
        }
        digest.update(&block[..count]);
    }
    if stamp(&before) != stamp(&file.metadata().map_err(|_| Error::Unavailable)?) {
        return Err(Error::ReconciliationRequired);
    }
    Ok((format!("{:x}", digest.finalize()), stamp(&before)))
}
fn streams(value: &Value) -> Result<(u64, Vec<MediaStreamEvidence>), Error> {
    let duration = value["format"]["duration"]
        .as_str()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.0 && *v <= 21600.0)
        .ok_or(Error::InvalidOutput)?;
    let rows = value["streams"]
        .as_array()
        .filter(|v| (2..=8).contains(&v.len()))
        .ok_or(Error::InvalidOutput)?;
    let values = rows
        .iter()
        .map(|s| {
            Ok(MediaStreamEvidence {
                kind: s["codec_type"].as_str().ok_or(Error::InvalidOutput)?.into(),
                codec: s["codec_name"].as_str().ok_or(Error::InvalidOutput)?.into(),
                width: s["width"].as_u64().and_then(|v| v.try_into().ok()),
                height: s["height"].as_u64().and_then(|v| v.try_into().ok()),
                channels: s["channels"].as_u64().and_then(|v| v.try_into().ok()),
                sample_rate: s["sample_rate"].as_str().and_then(|v| v.parse().ok()),
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(((duration * 1000.0).round() as u64, values))
}
impl YoutubeClient {
    /// Input paths come only from matched ready run IDs, never a task/model filesystem argument.
    pub async fn inspect_generated(
        &self,
        operation: &str,
    ) -> Result<Vec<VideoArtifactEvidence>, Error> {
        let status = self.status(operation).await?;
        if status.state != JobState::Completed
            || status.mode != "generate"
            || status.generated_count != status.count
            || status.scheduled_count != 0
            || status.failed_count != 0
            || status.runs.len() != status.count as usize
        {
            return Err(Error::ReconciliationRequired);
        }
        let mut ids = std::collections::BTreeSet::new();
        if status.runs.iter().any(|r| {
            r.state != "ready"
                || !r.artifacts.iter().any(|v| v == "video.mp4")
                || !ids.insert(r.id.clone())
        }) {
            return Err(Error::InvalidOutput);
        }
        let ffprobe = PathBuf::from("/usr/bin/ffprobe");
        let ffmpeg = PathBuf::from("/usr/bin/ffmpeg");
        let tools = tokio::task::spawn_blocking(move || {
            Ok::<_, Error>((digest(&ffprobe)?.0, digest(&ffmpeg)?.0))
        })
        .await
        .map_err(|_| Error::Unavailable)??;
        let mut evidence = Vec::new();
        for run in status.runs {
            let path = self
                .project_root
                .join(".local/ytbot/runs")
                .join(&run.id)
                .join("video.mp4");
            let source = path.clone();
            let (sha256, before) = tokio::task::spawn_blocking(move || digest(&source))
                .await
                .map_err(|_| Error::Unavailable)??;
            if before.3 < status.created_at {
                return Err(Error::ReconciliationRequired);
            }
            let rendered_at = before
                .3
                .saturating_mul(1000)
                .saturating_add(before.4 / 1_000_000);
            let argument = path.to_str().ok_or(Error::InvalidConfiguration)?;
            let raw=crate::process::media("/usr/bin/ffprobe",&["-v","error","-show_entries","format=duration:stream=codec_type,codec_name,width,height,channels,sample_rate","-of","json",argument],Duration::from_secs(15)).await?;
            let value: Value = serde_json::from_slice(&raw).map_err(|_| Error::InvalidOutput)?;
            let (duration_millis, streams) = streams(&value)?;
            crate::process::media(
                "/usr/bin/ffmpeg",
                &[
                    "-nostdin",
                    "-v",
                    "error",
                    "-xerror",
                    "-threads",
                    "1",
                    "-filter_threads",
                    "1",
                    "-i",
                    argument,
                    "-map",
                    "0:v:0",
                    "-map",
                    "0:a:0",
                    "-f",
                    "null",
                    "-",
                ],
                Duration::from_secs(1200),
            )
            .await?;
            if before != stamp(&std::fs::metadata(&path).map_err(|_| Error::Unavailable)?) {
                return Err(Error::ReconciliationRequired);
            }
            let suffix = format!("{:x}", Sha256::digest(format!("{operation}:{}", run.id)));
            let reference = format!("ytvideo-{operation}-{}", &suffix[..16]);
            let proof = VideoArtifactEvidence {
                operation_id: operation.into(),
                run_id: run.id,
                reference,
                size_bytes: before.2,
                sha256,
                duration_millis,
                streams,
                full_decode_verified: true,
                rendered_at,
                ffprobe_sha256: tools.0.clone(),
                ffmpeg_sha256: tools.1.clone(),
                verified_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| Error::Unavailable)?
                    .as_millis()
                    .try_into()
                    .map_err(|_| Error::InvalidOutput)?,
            };
            if !proof.validates(operation) {
                return Err(Error::InvalidOutput);
            }
            evidence.push(proof);
        }
        if evidence
            .iter()
            .map(|v| &v.sha256)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != evidence.len()
        {
            return Err(Error::InvalidOutput);
        }
        Ok(evidence)
    }
}

impl YoutubeClient {
    /// A cached decoding receipt remains usable only while the same ready runs have identical bytes.
    pub async fn artifacts_current(
        &self,
        operation: &str,
        proofs: &[VideoArtifactEvidence],
    ) -> Result<bool, Error> {
        let status = self.status(operation).await?;
        if status.state != JobState::Completed
            || status.mode != "generate"
            || status.generated_count != status.count
            || status.scheduled_count != 0
            || status.failed_count != 0
            || proofs.len() != status.count as usize
        {
            return Ok(false);
        }
        for proof in proofs {
            if !proof.validates(operation)
                || !status.runs.iter().any(|r| {
                    r.id == proof.run_id
                        && r.state == "ready"
                        && r.artifacts.iter().any(|a| a == "video.mp4")
                })
            {
                return Ok(false);
            }
            let path = self
                .project_root
                .join(".local/ytbot/runs")
                .join(&proof.run_id)
                .join("video.mp4");
            let (sha, size) = tokio::task::spawn_blocking(move || {
                digest(&path).map(|(hash, stamp)| (hash, stamp.2))
            })
            .await
            .map_err(|_| Error::Unavailable)??;
            if sha != proof.sha256 || size != proof.size_bytes {
                return Ok(false);
            }
        }
        Ok(proofs
            .iter()
            .map(|p| &p.run_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == proofs.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_duration_and_missing_streams_do_not_prove_media() {
        for v in [
            serde_json::json!({"format":{"duration":"NaN"},"streams":[]}),
            serde_json::json!({"format":{"duration":"10"},"streams":[]}),
        ] {
            assert!(streams(&v).is_err());
        }
    }
}
