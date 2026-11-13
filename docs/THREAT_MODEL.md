# Threat model

Assets are owner authorization, service tokens, tasks, audit history, worker files,
future account credentials, and artifacts. Adversaries include unauthenticated network
clients, a compromised worker, malicious repository/web/email content, and accidental
operator actions. A host administrator can read process memory and databases; host
compromise is outside the service-token boundary and needs OS-level protection.

## Trust boundaries and mitigations

- Owner to hub: authenticate every control/data endpoint; use private reachability,
  bounded TCP admission/idle time before authentication, bounded requests, explicit
  errors, and separate owner/worker roles.
- Worker to hub: authenticate before upgrade, bind identity, validate protocol version,
  capabilities, lease tokens, task IDs, and typed results. A compromised authenticated
  worker can fabricate diagnostic data or claimed model metadata; the hub validates
  bounded evidence but cannot attest remote machine integrity or provider execution.
- Hub to worker: accept only versioned assignments inside configured capabilities;
  journal intent and completion, validate operation identity, and reject unknown tools.
- External content to future model: treat retrieved data as untrusted evidence. It cannot
  change system prompts, approvals, credential scopes, or tool policy.
- Runtime to storage: transactions protect state/event consistency; file permissions and
  backups protect access. WAL is not encryption. Do not copy only the main SQLite file
  while active and assume it is a consistent backup.

## Failure and abuse cases

Replay and stale completion are addressed by durable operation identity, idempotency,
lease ownership, and state validation. Queue floods and oversized messages are bounded.
Disconnect creates uncertainty: repeat only the read-only diagnostic; future irreversible
actions require reconciliation and often owner review. Cancellation prevents further
accepted work but cannot undo an external action already committed.

Residual risks: static service-token theft requires operator rotation; no hardware
attestation, full local sandbox, encrypted vault, or provider privacy enforcement is
implemented. Remote desktop, OAuth, email, uploads, and privileged tooling require a
new threat review before activation. See [security](SECURITY.md).

## Phase 2 model boundary

The worker's provider key and owner disclosure choices are separate from task text.
Model output crosses a strict action parser, then deterministic policy and executor
checks. A prompt-injected request cannot add a tool or grant access. Public-coding
classification is an explicit owner assertion; it does not automatically detect secrets
inside mislabeled text. No private repository or account data is retrieved by this tool.

Provider requests can incur spend even if their response is lost. Transactional local
reservations, uncertainty retention, bounded retry, and debt admission contain that risk;
they do not establish exactly-once provider charging. A compromised host can bypass
local controls, so OS/file permissions and protected service identities remain necessary.
