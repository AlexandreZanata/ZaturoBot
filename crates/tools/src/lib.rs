//! Safe, bounded foundation diagnostic; no shell, filesystem, or network access.
use zaturo_core::DiagnosticResult;

pub const TOOL_IDS: &[&str] = &["diagnostic.v1"];

/// The registry describes execution boundaries, not permissions granted by a model.
pub struct ToolSpec {
    pub id: &'static str,
    pub description: &'static str,
    pub permission: &'static str,
    pub timeout_millis: u64,
    pub max_output_bytes: usize,
    pub cancellation: &'static str,
}

pub const DIAGNOSTIC: ToolSpec = ToolSpec {
    id: "diagnostic.v1",
    description: "Report the worker operating system and architecture without external side effects.",
    permission: "read_only",
    timeout_millis: 1000,
    max_output_bytes: 4096,
    cancellation: "Check cancellation before dispatch; the synchronous observation creates no subprocess.",
};

pub fn input_schema() -> serde_json::Value {
    serde_json::json!({"type":"object","properties":{"operation_id":{"type":"string","maxLength":64}},"required":["operation_id"],"additionalProperties":false})
}

pub fn output_schema() -> serde_json::Value {
    serde_json::json!({"type":"object","properties":{"operation_id":{"type":"string"},"tool":{"const":"diagnostic.v1"},"os":{"const":"linux"},"arch":{"type":"string","maxLength":64},"success":{"const":true},"summary":{"type":"string","maxLength":1024}},"required":["operation_id","tool","os","arch","success","summary"],"additionalProperties":false})
}

pub fn diagnostic(operation_id: &str) -> DiagnosticResult {
    DiagnosticResult {
        operation_id: operation_id.into(),
        tool: "diagnostic.v1".into(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
        success: cfg!(target_os = "linux"),
        summary: "Typed diagnostic completed without external side effects.".into(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn diagnostic_has_stable_identity_and_bounded_output() {
        let first = super::diagnostic("operation-1");
        let second = super::diagnostic("operation-1");
        assert_eq!(first, second);
        assert_eq!(first.tool, super::TOOL_IDS[0]);
        assert!(first.summary.len() < 256);
        assert_eq!(super::DIAGNOSTIC.id, first.tool);
        assert_eq!(super::input_schema()["additionalProperties"], false);
        assert_eq!(
            super::output_schema()["properties"]["success"]["const"],
            true
        );
    }
}
