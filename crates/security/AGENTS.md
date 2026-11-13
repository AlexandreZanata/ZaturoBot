# Security boundaries

Own bearer credential verification, secret-safe formatting, and the default-deny tool
policy. Never accept authorization from task text or model output. Do not log raw
credentials or include them in errors. Test denials as well as grants. Runtime config
belongs to the app composition roots. See [security](../../docs/SECURITY.md).
