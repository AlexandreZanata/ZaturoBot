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
Typed zero-model commands and explicit-opt-in bounded DeepSeek Go personal conversation,
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


## Owner-authorized 0.6 computer and social foundation

<!-- feature:semantic-computer status:implemented -->
Bounded home-worker DeepSeek planning, lease-bound remote semantic steps, explicit exact
owner confirmations and current-parent executor grants. computer-use-linux and Playwright
MCP remain independently installed/granted dependencies. Private current-window captures
can appear temporarily in Hub after an explicitly requested completed task. Full desktop
isolation, arbitrary OS/shell control and universal application support remain planned.

<!-- feature:social-foundation status:implemented -->
The Postiz connector supports reviewed initial platforms, exact inert drafts, separately
approved create/schedule/draft, durable unknown-effect fences and actual status observation.
Unsupported network schemas/comments remain unavailable; original YoutubeBot stays intact.

<!-- feature:stored-price-search status:implemented -->
A read-only BitLupa bridge formats actual exact-variant stored offers with provenance,
payment/freshness and honest value limitations. No active BitLupa collector or X7base
catalog is invented. Live crawling/recurring price monitoring remain planned.


## Owner-authorized Personal Go routing update

ADR-0021 supersedes the earlier Console-only selection for chat/computer planning.
The existing model integration adds explicit allow_personal_go opt-in, retains Personal
classification and one attempt, and checks subscription billing proof separately from
allowance use. Go coding diagnostics keep two steps; Personal planning keeps its smaller
configured budget and eight-step cap. Console402 remains historical evidence. One normal live chat completed with Go billing
and zero reported balance charge; see the provider record. Keep Extra Usage disabled; no
new credit, fallback model or expanded account/desktop grant follows from this routing change.


## Owner-authorized installed execution slice

<!-- feature:installed-opencode status:implemented -->
Typed natural-intent routing and a hash-pinned real OpenCode runner with a one-forward
Personal Go gateway, exact job broker, durable reconciliation, owned cancellation and
full media inspection. Source implementation is not physical acceptance or deployment.
See [the acceptance requirements](OPENCODE_EXECUTION.md).

<!-- feature:wayland-private-capture status:implemented -->
Separate single-window nonpersistent ScreenCast/PipeWire PNG transport in memory, retaining
the fixed X11 restrictions and native one-time viewer protections. Actual portal consent,
source compatibility and Android display must be observed on the destination PC/phone.


<!-- feature:remote-private-capture status:implemented -->
Native Android URL selection/bot-owned page opening and SMS foreground images reuse the
ephemeral relay. Source and offline validation are separate from the required0.8 Hub/APK
rollout and actual device display acceptance. See [capture](REMOTE_PRIVATE_CAPTURE.md).

Remote private capture follow-up: Android0.8.1 defaults phone/SMS prints to the current
screen without SMS/change dependencies; ready prepared prints open automatically in
focused Hub. Older-client fallback returns native capture metadata rather than navigation.
Physical acceptance still requires the compatible Hub and same-signature APK rollout.

## Procurali commerce content and website planning

The owner requested a business model, creative/short-video instructions, organic discovery,
an SEO-oriented affiliate website, a grounded product finder, an owner-group directory
and a Fable identity/site handoff. See [the planning specification](planning/commerce-refined/PROCURALI-BUSINESS-AND-SITE.md).
These documents are prepared; runtime integrations, media generation, recurring article/
social publication and deployment remain unimplemented in this slice. Personal-account
WhatsApp group automation is not activated; an official compatible route needs review.
