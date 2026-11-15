//! Curated media evidence; opaque references never contain a filesystem path.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaStreamEvidence {
    pub kind: String,
    pub codec: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub channels: Option<u32>,
    pub sample_rate: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoArtifactEvidence {
    pub operation_id: String,
    pub run_id: String,
    pub reference: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub duration_millis: u64,
    pub streams: Vec<MediaStreamEvidence>,
    pub full_decode_verified: bool,
    pub ffprobe_sha256: String,
    pub ffmpeg_sha256: String,
    pub verified_at: i64,
    pub rendered_at: i64,
}
impl VideoArtifactEvidence {
    pub fn validates(&self, operation: &str) -> bool {
        super::service::canonical_operation_id(operation)
            && self.operation_id == operation
            && crate::safe_filename(&self.run_id)
            && self.reference.starts_with(&format!("ytvideo-{operation}-"))
            && crate::validate_identifier(&self.reference).is_ok()
            && self.size_bytes > 1024
            && self.size_bytes <= 8 * 1024 * 1024 * 1024
            && crate::valid_sha256(&self.sha256)
            && (1..=21_600_000).contains(&self.duration_millis)
            && self.full_decode_verified
            && self.verified_at > 0
            && self.rendered_at > 0
            && self.rendered_at <= self.verified_at
            && crate::valid_sha256(&self.ffprobe_sha256)
            && crate::valid_sha256(&self.ffmpeg_sha256)
            && (2..=8).contains(&self.streams.len())
            && self.streams.iter().any(|s| s.kind == "video")
            && self.streams.iter().any(|s| s.kind == "audio")
            && self.streams.iter().all(|s| {
                !s.codec.is_empty()
                    && s.codec.len() <= 32
                    && s.codec
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    && match s.kind.as_str() {
                        "video" => {
                            s.width.is_some_and(|v| (1..=8192).contains(&v))
                                && s.height.is_some_and(|v| (1..=8192).contains(&v))
                        }
                        "audio" => {
                            s.channels.is_some_and(|v| (1..=32).contains(&v))
                                && s.sample_rate.is_some_and(|v| (8000..=384000).contains(&v))
                        }
                        _ => false,
                    }
            })
    }
}
