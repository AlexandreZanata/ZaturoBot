# Credential vault

Status: planned for Phase 4. The foundation stores no personal account credentials and
implements no encrypted vault. Hub/worker transport tokens are service bootstrap
credentials; redaction is not encrypted storage.

The boundary is model → orchestrator → policy engine → credential broker → encrypted
vault → external service. A model proposes a capability request. The broker independently
checks task, integration, resource, approval, expiry, and scope, then performs the allowed
operation. It returns a bounded result, never the raw credential merely because a model
asked. Google integrations use OAuth; never store the owner's Google password.

Use reviewed authenticated encryption and an established key-management design. Select
an envelope scheme with a random vault data key protected by an OS-backed or owner-unlock
key; document exact primitives and recovery tradeoffs before implementation. No custom
cryptography. Keep encrypted material and key protection separate. The hub sees opaque
credential references and approval metadata only.

Lock/unlock must revoke broker leases. Unattended access requires narrowly scoped,
expiring grants and an explicit decision about reboot unlock; never hide an unencrypted
master key in configuration. Rotation/revocation invalidate affected grants. Audits record
credential reference, capability, task, decision, and time without token contents.

Dependencies: OAuth callback design, process isolation, encrypted backup/restore,
owner recovery procedure, and provider disclosure rules. Threats: stolen notebook,
process-memory access, confused deputy requests, logs, revoked tokens, and backup loss.
Acceptance includes encryption tampering, lock/revoke races, least-scope broker tests,
secret-free logs/prompts, and a documented recovery drill using synthetic credentials.
