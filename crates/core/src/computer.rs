//! Bounded semantic computer actions. Plans never supply their own approval authority.
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputerBackend {
    Desktop,
    Browser,
    Bitlupa,
    Social,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerAction {
    pub backend: ComputerBackend,
    pub tool: String,
    #[serde(default = "empty_object")]
    pub arguments: Value,
    #[serde(default = "empty_object")]
    pub target: Value,
    #[serde(default)]
    pub expected_snapshot_sha256: Option<String>,
}
fn empty_object() -> Value {
    serde_json::json!({})
}
impl ComputerAction {
    pub fn validates(&self) -> bool {
        let allowed = match self.backend {
            ComputerBackend::Desktop => matches!(
                self.tool.as_str(),
                "doctor"
                    | "list_windows"
                    | "get_app_state"
                    | "get_tree_page"
                    | "click"
                    | "scroll"
                    | "press_key"
                    | "type_text"
                    | "set_value"
                    | "perform_action"
                    | "activate_window"
            ),
            ComputerBackend::Browser => matches!(
                self.tool.as_str(),
                "doctor"
                    | "browser_navigate"
                    | "browser_snapshot"
                    | "browser_click"
                    | "browser_type"
                    | "browser_press_key"
            ),
            ComputerBackend::Bitlupa => self.tool == "search",
            ComputerBackend::Social => matches!(
                self.tool.as_str(),
                "readiness" | "prepare" | "execute" | "status"
            ),
        };
        allowed
            && self.arguments.is_object()
            && self.target.is_object()
            && serde_json::to_vec(self).is_ok_and(|v| v.len() <= 6144)
            && self
                .expected_snapshot_sha256
                .as_ref()
                .is_none_or(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            && !serde_json::to_string(self).is_ok_and(|s| {
                [
                    "oc_sk_",
                    "AGE-SECRET-",
                    "-----BEGIN ",
                    "javascript:",
                    "file://",
                    "data:text/html",
                ]
                .iter()
                .any(|v| s.contains(v))
            })
            && match self.backend {
                ComputerBackend::Bitlupa => {
                    self.arguments.as_object().is_some_and(|a| a.len() == 1)
                        && self
                            .arguments
                            .get("query")
                            .and_then(Value::as_str)
                            .is_some_and(|s| crate::ordinary_text(s, 256))
                }
                ComputerBackend::Browser if self.tool == "browser_navigate" => {
                    self.arguments.as_object().is_some_and(|a| a.len() == 1)
                        && self
                            .arguments
                            .get("url")
                            .and_then(Value::as_str)
                            .is_some_and(|s| {
                                s.starts_with("https://") && s.len() <= 2048 && !s.contains('@')
                            })
                }
                _ => true,
            }
    }
    pub fn requires_approval(&self) -> bool {
        match self.backend {
            ComputerBackend::Desktop => !matches!(
                self.tool.as_str(),
                "doctor" | "list_windows" | "get_app_state" | "get_tree_page"
            ),
            ComputerBackend::Browser => !matches!(
                self.tool.as_str(),
                "doctor" | "browser_navigate" | "browser_snapshot"
            ),
            ComputerBackend::Bitlupa => false,
            ComputerBackend::Social => self.tool == "execute",
        }
    }
    pub fn review(&self) -> String {
        // Exact JSON is the approval payload; a model's description cannot replace it.
        serde_json::to_string(self).unwrap_or_default()
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerLease {
    pub operation_id: String,
    pub lease_token: String,
    pub lease_expires_at: i64,
    pub deadline_at: i64,
    pub owner_approved: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerObservation {
    pub outcome: String,
    pub text: String,
    pub data: Value,
}
fn ordinary_observation(value: &Value) -> bool {
    match value {
        Value::Object(v) => v.iter().all(|(k, v)| {
            ![
                "pixels",
                "image_bytes",
                "image_url",
                "screenshot",
                "screenshot_base64",
                "base64",
                "authorization",
                "lease_token",
                "cookie",
                "password",
                "identity",
            ]
            .contains(&k.to_lowercase().as_str())
                && ordinary_observation(v)
        }),
        Value::Array(v) => v.iter().all(ordinary_observation),
        Value::String(s) => ![
            "data:image/",
            "oc_sk_",
            "AGE-SECRET-",
            "-----BEGIN PRIVATE",
            "-----BEGIN OPENSSH",
        ]
        .iter()
        .any(|k| s.contains(k)),
        _ => true,
    }
}
impl ComputerObservation {
    pub fn validates(&self) -> bool {
        matches!(
            self.outcome.as_str(),
            "completed" | "partial" | "blocked" | "uncertain"
        ) && crate::ordinary_text(&self.text, 6000)
            && serde_json::to_vec(self).is_ok_and(|v| v.len() <= 12288)
            && self.data.as_object().is_some()
            && ordinary_observation(&self.data)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComputerStepRequest {
    pub sequence: u32,
    pub action: ComputerAction,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerStep {
    pub id: String,
    pub parent_id: String,
    pub sequence: u32,
    pub action: ComputerAction,
    pub state: String,
    pub child_id: Option<String>,
    pub created_at: i64,
    pub expires_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerStepStatus {
    pub step: ComputerStep,
    pub observation: Option<ComputerObservation>,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_actions_cannot_be_shell_or_approval_grants() {
        let mut a = ComputerAction {
            backend: ComputerBackend::Desktop,
            tool: "run_shell".into(),
            arguments: empty_object(),
            target: empty_object(),
            expected_snapshot_sha256: None,
        };
        assert!(!a.validates());
        a.tool = "click".into();
        assert!(a.validates() && a.requires_approval());
        a.tool = "get_app_state".into();
        assert!(a.validates() && !a.requires_approval());
        let v = serde_json::json!({"backend":"desktop","tool":"click","owner_approved":true});
        assert!(serde_json::from_value::<ComputerAction>(v).is_err());
        a.backend = ComputerBackend::Browser;
        a.tool = "browser_navigate".into();
        a.arguments = serde_json::json!({"url":"file:///etc/passwd"});
        assert!(!a.validates());
    }
}

pub fn requested_computer_capture(request: &str) -> bool {
    let lower = request.to_lowercase();
    if ["não ", "nao ", "sem print", "sem captura"]
        .iter()
        .any(|v| lower.contains(v))
    {
        return false;
    }
    [
        "mande um print",
        "mande o print",
        "me mande print",
        "envie um print",
        "envie o print",
        "me envie print",
        "com um print",
        "com print",
        "capture a tela",
        "tire um print",
    ]
    .iter()
    .any(|v| lower.contains(v))
}

#[cfg(test)]
mod capture_tests {
    #[test]
    fn screenshot_requires_an_explicit_nonnegated_owner_request() {
        for message in [
            "planeje o sprint",
            "não mande um print",
            "faça isso sem print",
            "organize documentos",
        ] {
            assert!(!super::requested_computer_capture(message));
        }
        for message in [
            "organize e me mande um print",
            "agende e envie um print",
            "tire um print da tela",
        ] {
            assert!(super::requested_computer_capture(message));
        }
    }
}
