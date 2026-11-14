# Roadmap and feature status

`config/registry.toml` is the machine-readable status catalog. Markers below are checked
against it. Implemented means the foundation slice exists; measured evidence belongs to
the engineering report. Planned contracts do not imply runtime availability.

<!-- feature:foundation status:implemented -->
## Phases 0/1: foundation

Rust workspace, canonical agent documentation, checked dependency boundaries, CI command,
SQLite task engine, authenticated owner/worker communication, deterministic diagnostic,
worker journal, bounded leases/retries, and local run/deployment instructions. The API/CLI
is the initial client. Private installation is separately owner-authorized; its actual
state is operational evidence, while the dashboard remains deferred.

<!-- feature:ai-orchestration status:implemented -->
## Phase 2: AI orchestration

The bounded public-coding diagnostic adds the exclusive DeepSeek route, versioned static
prompt, strict structured actions, independent typed-tool policy, fake-model/HTTP
fixtures, cancellation, and durable request/task/day/month token admission. This phase
does not implement arbitrary code changes or general personal tasks.
Gate: prohibited proposals cannot cross policy; errors/restarts cannot spend unboundedly.
The final integration/activation evidence is recorded in the Phase 2 engineering report.

<!-- feature:device-files status:implemented -->
## Authorized Phase 2 extension: device files

Independent device credentials, multiple bounded diagnostic workers, encrypted store-and-forward
files over outbound SSH tunnels, durable chunk checkpoints, and Ubuntu/Debian enrollment.
See [file transfers](FILE_TRANSFERS.md). Desktop input and account integrations remain planned.

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

Specialist agents, durable memory, delegation, and provider optimization. Basic independent
diagnostic workers are already supported by the authorized device-file extension.
Gate: a single-worker agent is already reliable and measured before complexity grows.

This pass includes the explicitly authorized Phase 2, device-file extension and private installation.
Phase 3 computer use and every later feature still require their own scoped request.


## Owner-authorized 0.4 vertical slices

The owner requested YoutubeBot preservation/improvement, Linux Slint and native Kotlin
Android clients, persistent daily activity/chat and controlled Android USB foundations.
These concrete slices are authorized even though the complete later phases remain planned.

<!-- feature:native-clients status:implemented -->
Initial native Linux and Android clients share owner/application-authenticated chat/feed;
Android uses its own pinned restricted SSH key and Keystore-encrypted enrollment.
The polished native presentation includes locally bundled Material icons, distinct chat
bubbles and verified latest-message positioning on Hub entry and confirmed sends;
passive history reading retains its anchor. Android presentation version: 0.4.1.

<!-- feature:android-usb status:implemented -->
Serial-bound USB state, controlled owner screenshot/finite visual-change watch and
short-lived age-encrypted RAM delivery to the desktop private viewer. No OTP OCR, SMS
semantic harvesting, automatic sign-in or lock/secure-screen bypass. Physical USB acceptance
remains deployment-specific; the separately authorized 0.5 mobile viewer is listed below.

<!-- feature:youtube-service status:implemented -->
The existing YoutubeBot video/extension/upload pipeline is preserved in its own repository;
a fixed typed service adapter, independent heavy job units and actual completion notices
integrate it with this hub. Real Google consent and channel actions remain user-interactive.


## Owner-authorized bounded assistant extension

<!-- feature:assistant-chat status:implemented -->
Typed zero-model commands and separate bounded DeepSeek Console personal conversation,
with limited ordinary context, durable spend/uncertainty and honest service readiness.
Generation-only YoutubeBot work does not imply publication or general computer use.

<!-- feature:shared-coordination status:implemented -->
Authenticated inert shared tasks, revisioned comments and encrypted transfer references,
with four compact MCP stdio tools. A receiving Codex acts on its owner's request, not by
polling instructions as permission. This is not autonomous multi-agent execution.

<!-- feature:mobile-private-view status:implemented -->
Dedicated mobile age recipients and temporary RAM decryption/display reuse the private
relay. Explicit serial-bound messaging-app navigation records local metadata recipes.
Physical acceptance on the work PC must be recorded separately; code tests do not prove
an absent phone, interactive login or destination deployment worked.


## Autonomous Commerce — planning only

The owner requested an isolated commercial-agency plan: verified offer curation, original
content, per-project visual identity, permitted social operation and results in the Hub.
The [13-document plan](planning/autonomous-commerce/README.md) audits both ZaturoBot and
MeuAssistente. Its [implementation phases](planning/autonomous-commerce/09-IMPLEMENTATION-ROADMAP.md)
start with an offline brand/evidence/draft/report slice, then an assisted commercial pilot,
then a separately gated single-channel publisher. These proposal milestones do not change
existing registry feature status or authorize production accounts, paid calls or messages.
Concurrent uncommitted computer-use work is excluded from this branch baseline.
