//! Version-one bounded JSON contracts. Authentication is outside message payloads.
use serde::{Deserialize, Serialize};
use zaturo_core::{Assignment, DiagnosticResult};

pub const VERSION: u16 = 1;
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClientMessage {
    Hello {
        version: u16,
        worker_id: String,
        capabilities: Vec<String>,
    },
    Heartbeat {
        lease_token: Option<String>,
    },
    Started {
        task_id: String,
        lease_token: String,
    },
    Completed {
        task_id: String,
        lease_token: String,
        result: DiagnosticResult,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ServerMessage {
    Welcome { version: u16 },
    Heartbeat,
    Assignment { assignment: Box<Assignment> },
    Ready { task_id: String },
    Acknowledged { task_id: String },
    Revoked { task_id: String },
    Error { code: String },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protocol_roundtrip_and_unknown_input() {
        let message = ClientMessage::Hello {
            version: VERSION,
            worker_id: "local-worker".into(),
            capabilities: vec!["diagnostic.v1".into()],
        };
        let encoded = serde_json::to_string(&message).unwrap();
        assert!(matches!(
            serde_json::from_str::<ClientMessage>(&encoded).unwrap(),
            ClientMessage::Hello {
                version: VERSION,
                ..
            }
        ));
        assert!(
            serde_json::from_str::<ClientMessage>(r#"{"type":"shell","command":"anything"}"#)
                .is_err()
        );
        assert!(
            serde_json::from_str::<ClientMessage>(
                r#"{"type":"heartbeat","lease_token":null,"grant":"root"}"#
            )
            .is_err()
        );
    }
}
