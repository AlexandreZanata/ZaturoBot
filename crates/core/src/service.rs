//! Fixed service operations, never model-controlled shell text.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServiceRequest {
    YoutubeProfiles,
    YoutubeRuns {
        channel: String,
        limit: u32,
    },
    YoutubeSubmit {
        channel: String,
        account: String,
        count: u32,
        approved_schedule: bool,
    },
    YoutubeStatus {
        operation_id: String,
    },
    AndroidDevices,
    AndroidPrivateView {
        device_id: String,
        session_id: String,
        recipient: String,
        watch_seconds: u64,
        owner_confirmed: bool,
    },
}
impl ServiceRequest {
    pub fn validates(&self) -> bool {
        match self {
            Self::YoutubeProfiles | Self::AndroidDevices => true,
            Self::AndroidPrivateView {
                device_id,
                session_id,
                recipient,
                watch_seconds,
                owner_confirmed,
            } => {
                crate::validate_identifier(device_id).is_ok()
                    && canonical_operation_id(session_id)
                    && recipient.starts_with("age1")
                    && recipient.len() <= 128
                    && (*watch_seconds == 0 || (5..=120).contains(watch_seconds))
                    && *owner_confirmed
            }
            Self::YoutubeRuns { channel, limit } => {
                crate::validate_identifier(channel).is_ok() && (1..=20).contains(limit)
            }
            Self::YoutubeSubmit {
                channel,
                account,
                count,
                approved_schedule,
            } => {
                crate::validate_identifier(channel).is_ok()
                    && crate::validate_identifier(account).is_ok()
                    && (1..=10).contains(count)
                    && *approved_schedule
            }
            Self::YoutubeStatus { operation_id } => {
                operation_id.len() == 36
                    && operation_id
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() || b == b'-')
            }
        }
    }
    pub fn capability(&self) -> &'static str {
        match self {
            Self::AndroidDevices => "android.devices.v1",
            Self::AndroidPrivateView { .. } => "android.private.v1",
            Self::YoutubeSubmit { .. } => "youtube.submit.v1",
            _ => "youtube.read.v1",
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::YoutubeProfiles => "youtube_profiles",
            Self::YoutubeRuns { .. } => "youtube_runs",
            Self::YoutubeSubmit { .. } => "youtube_submit",
            Self::YoutubeStatus { .. } => "youtube_status",
            Self::AndroidPrivateView { .. } => "android_private_view",
            Self::AndroidDevices => "android_devices",
        }
    }
    pub fn side_effecting(&self) -> bool {
        matches!(self, Self::YoutubeSubmit { .. })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceResult {
    pub operation_id: String,
    pub action: String,
    pub outcome: String,
    pub summary: String,
    pub data: serde_json::Value,
}
pub const PRIVATE_VIEW_READY_SUMMARY: &str =
    "Imagem privada disponível uma vez, por até 60 segundos; conteúdo não salvo no histórico.";
pub const PRIVATE_VIEW_UNCHANGED_SUMMARY: &str =
    "Nenhuma alteração de tela detectada no intervalo autorizado.";
impl ServiceResult {
    pub fn validates(&self, operation: &str, request: &ServiceRequest) -> bool {
        if !request.validates()
            || self.operation_id != operation
            || self.action != request.name()
            || !matches!(self.outcome.as_str(), "completed" | "accepted")
            || self.summary.is_empty()
            || self.summary.len() > 512
            || !serde_json::to_vec(self).is_ok_and(|v| v.len() <= 4096)
        {
            return false;
        }
        let ServiceRequest::AndroidPrivateView { session_id, .. } = request else {
            return true;
        };
        if self.outcome != "completed" {
            return false;
        }
        let Some(metadata) = self.data.as_object() else {
            return false;
        };
        if metadata
            .get("session_id")
            .and_then(serde_json::Value::as_str)
            != Some(session_id.as_str())
        {
            return false;
        }
        if metadata.len() == 2
            && metadata.get("state").and_then(serde_json::Value::as_str) == Some("unchanged")
        {
            return self.summary == PRIVATE_VIEW_UNCHANGED_SUMMARY;
        }
        metadata.len() == 3
            && self.summary == PRIVATE_VIEW_READY_SUMMARY
            && metadata.get("delivery").and_then(serde_json::Value::as_str)
                == Some("ephemeral_encrypted_memory")
            && metadata
                .get("max_view_seconds")
                .and_then(serde_json::Value::as_u64)
                == Some(60)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceActivity {
    pub operation_id: String,
    pub state: String,
    pub summary: String,
}
impl ServiceActivity {
    pub fn validates(&self) -> bool {
        self.operation_id.len() == 36
            && matches!(
                self.state.as_str(),
                "accepted" | "running" | "completed" | "failed" | "uncertain"
            )
            && !self.summary.trim().is_empty()
            && self.summary.len() <= 512
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivateViewGrant {
    pub id: String,
    pub worker_id: String,
    pub device_id: String,
    pub recipient: String,
    pub watch_seconds: u64,
    pub expires_at: i64,
}

fn canonical_operation_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}

#[cfg(test)]
mod private_result_tests {
    use super::*;
    const OP: &str = "00000000-0000-4000-8000-000000000001";
    const SESSION: &str = "00000000-0000-4000-8000-000000000002";
    fn request() -> ServiceRequest {
        ServiceRequest::AndroidPrivateView {
            device_id: "owner-phone".into(),
            session_id: SESSION.into(),
            recipient: "age1synthetic-public-recipient".into(),
            watch_seconds: 0,
            owner_confirmed: true,
        }
    }
    fn ready() -> ServiceResult {
        ServiceResult {
            operation_id: OP.into(),
            action: "android_private_view".into(),
            outcome: "completed".into(),
            summary: PRIVATE_VIEW_READY_SUMMARY.into(),
            data: serde_json::json!({"session_id":SESSION,"delivery":"ephemeral_encrypted_memory","max_view_seconds":60}),
        }
    }
    #[test]
    fn private_result_contains_only_fixed_delivery_metadata_or_unchanged_state() {
        assert!(ready().validates(OP, &request()));
        let unchanged = ServiceResult {
            summary: PRIVATE_VIEW_UNCHANGED_SUMMARY.into(),
            data: serde_json::json!({"session_id":SESSION,"state":"unchanged"}),
            ..ready()
        };
        assert!(unchanged.validates(OP, &request()));
    }
    #[test]
    fn even_tiny_private_pixels_codes_and_free_form_private_summaries_are_rejected() {
        for (name, value) in [
            ("image", "U1lOVEhFVElDX0lNQUdF"),
            ("code", "000111"),
            ("caption", "synthetic private SMS content"),
        ] {
            let mut result = ready();
            result.data[name] = serde_json::json!(value);
            assert!(serde_json::to_vec(&result).unwrap().len() < 4096);
            assert!(!result.validates(OP, &request()));
        }
        let mut result = ready();
        result.summary = "Synthetic verification code 000111".into();
        assert!(!result.validates(OP, &request()));
        result.data = serde_json::json!({"session_id":SESSION,"state":"unchanged"});
        assert!(!result.validates(OP, &request()));
    }
    #[test]
    fn private_session_delivery_expiry_and_outcome_must_match_the_grant() {
        let mut result = ready();
        result.data["session_id"] = serde_json::json!(OP);
        assert!(!result.validates(OP, &request()));
        let mut result = ready();
        result.data["delivery"] = serde_json::json!("ordinary_file_spool");
        assert!(!result.validates(OP, &request()));
        let mut result = ready();
        result.data["max_view_seconds"] = serde_json::json!(61);
        assert!(!result.validates(OP, &request()));
        let mut result = ready();
        result.outcome = "accepted".into();
        assert!(!result.validates(OP, &request()));
        let mut unconfirmed = request();
        if let ServiceRequest::AndroidPrivateView {
            owner_confirmed, ..
        } = &mut unconfirmed
        {
            *owner_confirmed = false;
        }
        assert!(!ready().validates(OP, &unconfirmed));
    }
}
