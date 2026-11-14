# Security boundaries

Own bearer credential verification, secret-safe formatting, and the default-deny tool
policy. Never accept authorization from task text or model output. Do not log raw
credentials or include them in errors. Test denials as well as grants. Runtime config
belongs to the app composition roots. See [security](../../docs/SECURITY.md).

Registry credentials load strict lowercase SHA-256 digests and verify with the same
constant-time comparison. Registry entries contain digests only; keep raw credentials in their private role
environments and never log them.
