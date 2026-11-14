# Requirements

This document is the normative scope summary of the owner's foundation and subsequently
authorized Phase 2/deployment work.
Implementation status belongs to [roadmap](ROADMAP.md), not to duplicated checklists.

- R01: Use Rust 2024, Linux, a lightweight SQLite hub, and an outbound notebook worker.
  No Docker, Redis, PostgreSQL, browser, local model, or project compilation on the VPS.
- R02: Authenticate owner API and worker transport separately. Bind private/loopback by
  default. Require a VPN, TLS termination, or an authenticated SSH tunnel for remote access.
- R03: Persist task creation, assignment, transitions, results, and failures. Use stable
  task/operation identifiers, idempotency keys, UTC timestamps, leases, and bounded retries.
- R04: Recover pending work after hub restart. Reconcile worker journal state after
  reconnect. Never promise exactly-once external effects or blindly repeat uncertain ones.
- R05: Implement one typed read-only diagnostic. A task completes only after its typed
  result meets the diagnostic acceptance contract. Do not add arbitrary shell execution.
- R06: Bound queues, connections, messages, pagination, concurrency, retries, and runtime.
  Memory targets are below 150 MB normally and below 300 MB peak; prove them with measurements.
- R07: Owner cancellation and pause stop future dispatch. Future subprocess tools must
  terminate their owned process group safely and record uncertain effects.
- R08: Keep security policy independent of model output. Secrets and external account
  credentials stay off the hub. Reject untrusted content as authorization.
- R09: Maintain concise canonical agent instructions, crate contracts, dependency rules,
  component inventory, meaningful future designs, recipes, and one CI quality command.
- R10: All engineering artifacts are English. Future owner UI defaults to `pt-BR` with
  `en` available. Schedule interpretation uses `America/Cuiaba` initially and persists UTC.
- R11: Tests use synthetic data, temporary databases, injected time, and local transport.
  They do not need production accounts, paid requests, or arbitrary external network access.
- R12: Complete the authorized Phases 0–2 and private hub/worker installation; publication
  to the specified GitHub repository is explicitly authorized after validation. Never
  publish private keys, tokens, backups, or local owner data. Other repositories remain
  read-only. Do not silently implement later roadmap phases.
- R13: Coding-assisted tasks require public-coding classification, the exclusive owner
  model allowlist, strict structured actions, independent tool policy, and durable
  request/task/day/month budget admission. Unknown paid outcomes require reconciliation.

Acceptance: start hub and worker; create/list/show an authenticated diagnostic task;
inspect persisted event history; verify task completion and reconnect/restart behavior;
run `cargo xtask check`. See [testing](TESTING.md) for evidence boundaries.


## Owner-authorized device file extension

The owner requested bidirectional files and specific registered bot tasks between two
Linux Ubuntu/Debian PCs on different networks through the VPS, plus private setup material
for the destination Codex and publication of code only. Use separate device credentials,
resumable bounded encrypted transfer and private inboxes; preserve local files/certificates
in the unrelated PDF project. The work PC can run the existing diagnostic; only the home
worker has the bounded public-coding DeepSeek route. Mouse/desktop/browser/social posting
are future requests and are not implemented by receiving a file or submitting task text.


## Owner-authorized 0.4 scope

The owner additionally authorized YoutubeBot improvements (preserve extension/video/upload),
removal of that project's harness scaffolding, private local-data migration to the already
enrolled work PC, Linux Slint/Rust and Android Kotlin clients, branded system design and
Android USB foundations. This supersedes the earlier read-only rule for YoutubeBot only;
CelularLinuxADB and other projects remain read-only references. Keep scopes, provenance,
original local files and existing work-PC credentials/journal. Make recommended decisions
without user questions; unavailable hardware/interactive Google consent is reported honestly,
not bypassed. Native sensitive views remain separate from the persistent ordinary feed.


## Owner-authorized assistant and coordination extension (0.5)

The owner requested conversational AI from both native apps, low-context MCP tools for
an area shared with the other PC's Codex, typed video generation with precise login errors,
and real private USB-phone images in the Android chat. Use only deepseek-v4.1-flash.
Known explicit commands skip inference; other owner messages use a separately configured
Personal/Console route, one bounded call with clipped ordinary context and the existing
private provider credential. The coding Go route remains exclusive to PublicCoding.
Never automatically disclose files, cookies, screenshots or provider reasoning.

Shared instructions, comments and transfer references are inert persistent coordination.
The receiving Codex acts only when the owner requests it, within its own local permissions;
MCP cannot grant arbitrary shell, account, publication or device rights. Derive provenance
from authentication, bound output and fence stale/concurrent status updates by revision.
A request to generate videos grants generation only, not upload/scheduling. Check actual
Google/ChatGPT readiness, distinguish logged-out/unknown/unavailable states, and fence an
uncertain batch instead of resubmitting it. Retain all original video/extension behavior.

Opening the registered USB phone's messaging app is a separate explicit action; no SMS
sending, automatic code use, lock bypass or new Android permission. Private captures stay
endpoint-encrypted, RAM-only, consume-once and <=60 seconds, including the mobile display.
They appear temporarily inside Hub without persistent image history or model access.
Keep verified metadata-only device recipes on the work PC and report target-PC/physical
acceptance only after observation. Code publication after validation is authorized.
