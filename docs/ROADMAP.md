# Roadmap and feature status

`config/registry.toml` is the machine-readable status catalog. Markers below are checked
against it. Implemented means the foundation slice exists; measured evidence belongs to
the engineering report. Planned contracts do not imply runtime availability.

<!-- feature:foundation status:implemented -->
## Phases 0/1: foundation

Rust workspace, canonical agent documentation, checked dependency boundaries, CI command,
SQLite task engine, authenticated owner/worker communication, deterministic diagnostic,
worker journal, bounded leases/retries, and local run/deployment instructions. The API/CLI
is the initial client. Dashboard and real production installation are deferred.

<!-- feature:ai-orchestration status:planned -->
## Phase 2: AI orchestration

Provider-independent router, verified eligible OpenCode Go adapter, structured planner,
tool registry expansion, fake model tests, privacy authorization, and budget accounting.
Gate: prohibited proposals cannot cross policy; errors/restarts cannot spend unboundedly.

<!-- feature:computer-use status:planned -->
## Phase 3: Linux computer use

Dedicated isolated desktop, semantic browser/accessibility tools, screenshots, explicit
human takeover, bounded process control, and private observation. Gate: isolation and
input revocation verified on the target notebook without assuming GPU acceleration.

<!-- feature:vault status:planned -->
## Phase 4: secure vault

Reviewed encryption/key management, broker, OAuth, scope/approval policies, rotation,
revocation, and recovery. Gate: synthetic-secret tamper/revoke/restore tests.

<!-- feature:integrations status:planned -->
## Phase 5: personal integrations

GitHub project workflows, existing video adapter, compliant price monitoring, and Gmail.
Gate: fixture-based tests and explicit owner authorization before real external effects.

<!-- feature:scheduling status:planned -->
## Phase 6: autonomous scheduling

Timezone-aware windows, task dependencies, recurring occurrences, daily limits,
notifications, checkpoints, and uncertain-effect reconciliation. Gate: fake-clock restart,
missed-window, permission, and budget tests.

<!-- feature:clients status:planned -->
## Phase 7: desktop/mobile experience

Responsive panel/PWA, notifications, observation, approval UI, and a Kotlin Multiplatform
Android client if justified. Brazilian Portuguese default; English additional locale.
Gate: secure sessions, accessible controls, owner takeover, and reconnect tests.

<!-- feature:advanced-autonomy status:planned -->
## Phase 8: advanced autonomy

Multiple workers, specialist agents, durable memory, delegation, and provider optimization.
Gate: a single-worker agent is already reliable and measured before complexity grows.

This pass stops after validation of Phases 0/1. No future phase is implicitly authorized.
