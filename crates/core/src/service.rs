//! Fixed service operations, never model-controlled shell text.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServiceRequest {
    AssistantRoute {
        message: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        context: Vec<ConversationTurn>,
        target_worker: String,
        planner_worker: String,
    },
    YoutubeOpenCodeGenerate {
        channel: String,
        count: u32,
    },
    ComputerTask {
        request: String,
        target_worker: String,
    },
    ComputerStep {
        parent_id: String,
        sequence: u32,
        #[serde(rename = "tool_action")]
        action: crate::ComputerAction,
        owner_approved: bool,
    },
    PriceSearch {
        query: String,
    },
    DesktopPrivateView {
        session_id: String,
        recipient: String,
        owner_confirmed: bool,
    },
    BrowserPrivateView {
        session_id: String,
        recipient: String,
        owner_confirmed: bool,
    },
    AndroidMessagesPrivateView {
        device_id: String,
        session_id: String,
        recipient: String,
        return_to_hub: bool,
        owner_confirmed: bool,
    },
    AssistantChat {
        message: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        context: Vec<ConversationTurn>,
    },
    YoutubeGenerate {
        channel: String,
        count: u32,
    },
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
    AndroidOpenMessages {
        device_id: String,
        owner_confirmed: bool,
    },
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
            Self::AssistantRoute {
                message,
                context,
                target_worker,
                planner_worker,
            } => {
                crate::ordinary_text(message, 2048)
                    && crate::validate_identifier(target_worker).is_ok()
                    && crate::validate_identifier(planner_worker).is_ok()
                    && context.len() <= 4
                    && context.iter().all(|t| t.validates())
                    && context.iter().map(|t| t.text.len()).sum::<usize>() <= 1024
            }
            Self::YoutubeOpenCodeGenerate { channel, count } => {
                crate::canonical_youtube_channel(channel) == Some(channel.as_str())
                    && (1..=10).contains(count)
            }
            Self::ComputerTask {
                request,
                target_worker,
            } => {
                crate::ordinary_text(request, 2048)
                    && crate::validate_identifier(target_worker).is_ok()
            }
            Self::ComputerStep {
                parent_id,
                sequence,
                action,
                owner_approved,
            } => {
                canonical_operation_id(parent_id)
                    && (1..=8).contains(sequence)
                    && action.validates()
                    && (!action.requires_approval() || *owner_approved)
            }
            Self::PriceSearch { query } => crate::ordinary_text(query, 256),
            Self::DesktopPrivateView {
                session_id,
                recipient,
                owner_confirmed,
            }
            | Self::BrowserPrivateView {
                session_id,
                recipient,
                owner_confirmed,
            } => {
                canonical_operation_id(session_id)
                    && recipient.starts_with("age1")
                    && recipient.len() <= 128
                    && *owner_confirmed
            }
            Self::AndroidOpenMessages {
                device_id,
                owner_confirmed,
            } => crate::validate_identifier(device_id).is_ok() && *owner_confirmed,
            Self::AssistantChat { message, context } => {
                crate::ordinary_text(message, 2048)
                    && context.len() <= 4
                    && context.iter().all(|t| t.validates())
                    && context.iter().map(|t| t.text.len()).sum::<usize>() <= 1024
            }
            Self::YoutubeGenerate { channel, count } => {
                crate::validate_identifier(channel).is_ok() && (1..=10).contains(count)
            }
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
            Self::AndroidMessagesPrivateView {
                device_id,
                session_id,
                recipient,
                owner_confirmed,
                ..
            } => {
                crate::validate_identifier(device_id).is_ok()
                    && canonical_operation_id(session_id)
                    && recipient.starts_with("age1")
                    && recipient.len() <= 128
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
            Self::AssistantRoute { .. } => "assistant.route.v1",
            Self::YoutubeOpenCodeGenerate { .. } => "youtube.opencode.v1",
            Self::ComputerTask { .. } => "computer.agent.v1",
            Self::ComputerStep { .. } => "computer.action.v1",
            Self::PriceSearch { .. } => "price.search.v1",
            Self::DesktopPrivateView { .. } => "computer.private.v1",
            Self::BrowserPrivateView { .. } => "browser.private.v1",
            Self::AndroidMessagesPrivateView { .. } => "android.messages.private.v1",
            Self::AndroidOpenMessages { .. } => "android.messages.v1",
            Self::AssistantChat { .. } => "assistant.chat.v1",
            Self::YoutubeGenerate { .. } => "youtube.generate.v1",
            Self::AndroidDevices => "android.devices.v1",
            Self::AndroidPrivateView { .. } => "android.private.v1",
            Self::YoutubeSubmit { .. } => "youtube.submit.v1",
            _ => "youtube.read.v1",
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::AssistantRoute { .. } => "assistant_route",
            Self::YoutubeOpenCodeGenerate { .. } => "youtube_opencode_generate",
            Self::ComputerTask { .. } => "computer_task",
            Self::ComputerStep { .. } => "computer_step",
            Self::PriceSearch { .. } => "price_search",
            Self::DesktopPrivateView { .. } => "desktop_private_view",
            Self::BrowserPrivateView { .. } => "browser_private_view",
            Self::AndroidMessagesPrivateView { .. } => "android_messages_private_view",
            Self::AndroidOpenMessages { .. } => "android_open_messages",
            Self::AssistantChat { .. } => "assistant_chat",
            Self::YoutubeGenerate { .. } => "youtube_generate",
            Self::YoutubeProfiles => "youtube_profiles",
            Self::YoutubeRuns { .. } => "youtube_runs",
            Self::YoutubeSubmit { .. } => "youtube_submit",
            Self::YoutubeStatus { .. } => "youtube_status",
            Self::AndroidPrivateView { .. } => "android_private_view",
            Self::AndroidDevices => "android_devices",
        }
    }
    pub fn side_effecting(&self) -> bool {
        matches!(
            self,
            Self::DesktopPrivateView { .. }
                | Self::BrowserPrivateView { .. }
                | Self::AndroidMessagesPrivateView { .. }
                | Self::AssistantRoute { .. }
                | Self::YoutubeOpenCodeGenerate { .. }
                | Self::ComputerTask { .. }
                | Self::ComputerStep { .. }
                | Self::AndroidOpenMessages { .. }
                | Self::YoutubeSubmit { .. }
                | Self::YoutubeGenerate { .. }
                | Self::AssistantChat { .. }
        )
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
            || !matches!(
                self.outcome.as_str(),
                "completed" | "accepted" | "blocked" | "partial"
            )
            || self.summary.is_empty()
            || self.summary.len()
                > if matches!(
                    request,
                    ServiceRequest::AssistantChat { .. }
                        | ServiceRequest::AssistantRoute { .. }
                        | ServiceRequest::ComputerTask { .. }
                        | ServiceRequest::PriceSearch { .. }
                ) {
                    2048
                } else {
                    512
                }
            || !serde_json::to_vec(self).is_ok_and(|v| {
                v.len()
                    <= if matches!(request, ServiceRequest::ComputerStep { .. }) {
                        16384
                    } else if matches!(
                        request,
                        ServiceRequest::ComputerTask { .. } | ServiceRequest::PriceSearch { .. }
                    ) {
                        8192
                    } else {
                        4096
                    }
            })
        {
            return false;
        }
        let session_id = match request {
            ServiceRequest::YoutubeOpenCodeGenerate { channel, count } => {
                return self.outcome == "blocked"
                    || self
                        .data
                        .get("opencode")
                        .cloned()
                        .and_then(|v| serde_json::from_value::<crate::OpenCodeEvidence>(v).ok())
                        .is_some_and(|proof| proof.validates(operation, channel, *count));
            }
            ServiceRequest::AssistantRoute { message, .. } => {
                return self.outcome == "blocked"
                    || (self.outcome == "completed"
                        && self.data["model"] == "deepseek-v4.1-flash"
                        && self.data["model_calls"] == 1
                        && matches!(
                            self.data["billing"]["source"].as_str(),
                            Some("go" | "go_plus" | "go-plus")
                        )
                        && self
                            .data
                            .get("usage")
                            .cloned()
                            .and_then(|v| serde_json::from_value::<crate::ModelUsage>(v).ok())
                            .is_some_and(|u| u.validates() && u.cost_microusd == Some(0))
                        && self
                            .data
                            .get("decision")
                            .cloned()
                            .and_then(|v| serde_json::from_value::<crate::IntentDecision>(v).ok())
                            .is_some_and(|v| v.validates_for(message)));
            }
            ServiceRequest::AndroidPrivateView { session_id, .. }
            | ServiceRequest::DesktopPrivateView { session_id, .. }
            | ServiceRequest::BrowserPrivateView { session_id, .. }
            | ServiceRequest::AndroidMessagesPrivateView { session_id, .. } => session_id,
            ServiceRequest::ComputerStep { .. } => {
                return self
                    .data
                    .get("observation")
                    .cloned()
                    .and_then(|v| serde_json::from_value::<crate::ComputerObservation>(v).ok())
                    .is_some_and(|o| o.validates());
            }
            _ => return true,
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
            return matches!(request,ServiceRequest::AndroidPrivateView{watch_seconds,..} if *watch_seconds>0)
                && self.summary == PRIVATE_VIEW_UNCHANGED_SUMMARY;
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
    #[serde(default, skip_serializing_if = "PrivateCapture::is_current")]
    pub capture: PrivateCapture,
}

/// Selection is private grant data, never model context or persisted task text.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PrivateCapture {
    #[default]
    Current,
    Browser {
        url: String,
    },
    Messages {
        return_to_hub: bool,
    },
}
impl std::fmt::Debug for PrivateCapture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Current => "Current",
            Self::Browser { .. } => "Browser([REDACTED])",
            Self::Messages { .. } => "Messages",
        })
    }
}
impl PrivateCapture {
    pub fn is_current(&self) -> bool {
        matches!(self, Self::Current)
    }
}
/// Lexical bound only. The reviewed browser also validates parsed URL and every network address.
pub fn valid_private_page_url(value: &str) -> bool {
    let authority = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .and_then(|v| v.split(['/', '?', '#']).next());
    value.len() <= 2048
        && !value
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '\\')
        && authority.is_some_and(|v| !v.is_empty() && !v.contains(['@', '%']))
}

pub(crate) fn canonical_operation_id(value: &str) -> bool {
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
    fn one_shot_sources_never_complete_as_unchanged() {
        for request in [
            request(),
            ServiceRequest::AndroidMessagesPrivateView {
                device_id: "owner-phone".into(),
                session_id: SESSION.into(),
                recipient: "age1synthetic-public-recipient".into(),
                owner_confirmed: true,
                return_to_hub: false,
            },
            ServiceRequest::DesktopPrivateView {
                session_id: SESSION.into(),
                recipient: "age1synthetic-public-recipient".into(),
                owner_confirmed: true,
            },
            ServiceRequest::BrowserPrivateView {
                session_id: SESSION.into(),
                recipient: "age1synthetic-public-recipient".into(),
                owner_confirmed: true,
            },
        ] {
            let result = ServiceResult {
                action: request.name().into(),
                summary: PRIVATE_VIEW_UNCHANGED_SUMMARY.into(),
                data: serde_json::json!({"session_id":SESSION,"state":"unchanged"}),
                ..ready()
            };
            assert!(!result.validates(OP, &request));
            let delivered = ServiceResult {
                action: request.name().into(),
                ..ready()
            };
            assert!(delivered.validates(OP, &request));
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
        assert!(!unchanged.validates(OP, &request()));
        let mut watch = request();
        if let ServiceRequest::AndroidPrivateView { watch_seconds, .. } = &mut watch {
            *watch_seconds = 5;
        }
        assert!(unchanged.validates(OP, &watch));
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationTurn {
    pub role: String,
    pub text: String,
}
impl ConversationTurn {
    pub fn validates(&self) -> bool {
        matches!(self.role.as_str(), "owner" | "assistant") && crate::ordinary_text(&self.text, 512)
    }
}
