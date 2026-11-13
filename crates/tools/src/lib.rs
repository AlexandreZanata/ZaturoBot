//! Safe, bounded foundation diagnostic; no shell, filesystem, or network access.
use zaturo_core::DiagnosticResult;

pub const TOOL_IDS: &[&str] = &["diagnostic.v1"];

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
    }
}
