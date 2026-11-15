# Architecture decisions

## ADR-0001: Split coordination from execution — accepted

The 1 GB VPS cannot host heavy work. Use a lightweight Rust hub and an outbound notebook
worker. SQLite databases remain local to their responsibilities. This keeps router
configuration unnecessary and permits independent restart. The tradeoff is explicit
leases, reconciliation, and protocol versioning rather than a shared process lifetime.

## ADR-0002: Minimum coherent workspace — accepted

Create only core, protocol, db, services, security, tools, two apps, and xtask.
Boundaries with executable responsibilities justify each package. Defer empty scheduler,
orchestrator, provider, and integration crates. Machine-enforce the allowed graph.

## ADR-0003: SQLite and operation-ledger recovery — accepted

Use WAL, foreign keys, migrations, bounded queries, and transactions. Track a stable
operation ID across attempts and a fresh lease token per assignment. Journal worker
intent before execution and result before transmission. Repeating the initial diagnostic
is safe; external-effect tools must add reconciliation before becoming dispatchable.

## ADR-0004: Private authenticated foundation — accepted

Separate owner and worker tokens, bind the configured worker identity, and reject
unregistered capabilities. Loopback is the development default. Remote installation
requires private VPN reachability or authenticated TLS termination; there is no public
unauthenticated control plane. TLS and VPN setup are deployment responsibilities.

## ADR-0005: Rust 1.99.0 and native services — accepted

Pin the installed verified stable Rust 1.99.0, use edition 2024 and native systemd units.
This is a reproducibility choice, not a claim to use the newest available release.
Do not require Docker. Compile release artifacts away from the constrained hub.

## ADR-0006: Initial provider and UI deferral — superseded for Phase 2 providers

The original vertical slice needed no LLM, dashboard, or desktop. API/CLI validation
came first. ADR-0010 now accepts the bounded Phase 2 provider route after dated official
verification and owner authorization. Dashboard and desktop remain later work.

## ADR-0007: License recommendation — proposed, owner decision pending

Recommend dual MIT OR Apache-2.0, a common permissive Rust ecosystem choice with the
Apache patent grant and broad reuse. This is a recommendation, not an adopted license.
No license grant is inferred from public repository visibility. The owner must confirm
before adding license texts or publishing a licensed release. Dependencies retain their
own licenses; review all notices when preparing distribution.

## ADR-0008: Test-only composition dependency — accepted

The worker's integration tests depend on the hub library to start a real loopback hub
with a temporary database and run the actual worker against it. This is a `dev-dependency`
only, explicitly permitted by `may_dev_depend_on` in `architecture.toml`. Production
worker dependencies may not include the hub. This tests the distributed boundary without
creating a production coupling or requiring external network services.

## ADR-0009: Authorized Phase 2 and private installation — accepted

The owner explicitly authorized hub activation on the prepared Oracle host, a worker
on the current PC, Phase 2 implementation, and GitHub publication after validation.
Use the existing verified SSH identity and port 22 for a persistent encrypted tunnel.
The hub remains loopback-bound at 127.0.0.1:7878; the PC forwards 127.0.0.1:17878.
This avoids a new public control port, router changes, DNS, or an unreviewed TLS proxy.
Hub systemd and PC user services are separate operational composition. User lingering
supports the current headless worker; this is not the future isolated desktop session.

## ADR-0010: Exclusive public-coding model route — accepted

The owner selected only deepseek-v4.1-flash and explicitly authorized bounded paid
verification with a worker-local credential. No alternative or fallback model is enabled.
The implementation admits only owner-classified public coding tasks, with explicit
Global-processing permission, a fixed verified provider endpoint, and independent policy
checks. Private code, accounts, documents, screenshots, and general personal automation
remain outside this route. Official evidence is dated in
[the provider research](OPENCODE_GO_RESEARCH.md); it is not a permanent privacy promise.
ADR-0021 adds an explicit Personal/Go use case without changing this diagnostic contract.

## ADR-0011: Durable model reservation ledger — accepted

Worker schema version 3 adds transactional request/task/UTC-day/UTC-month admission and
assisted-result persistence. Reserve before send; settle usage after validated receipt.
A pending or uncertain reservation consumes budget after restart, and an overrun becomes
retained debt. Reusing an operation/request identity cannot issue another paid request.
Only a complete cached assisted result can be reported again without inference.
This trades automatic recovery for bounded spend when delivery or charging is uncertain.

## ADR-0012: Protocol version 2 with fenced failures — accepted

The assisted completion and typed worker-failure envelopes extend the wire protocol.
Both ends must negotiate version 2; reject incompatible peers instead of silently
interpreting a new message as version 1. Existing direct diagnostic task/result fields
remain available, with optional assisted metadata in persisted records. Worker schema
upgrade and binary rollout require backups and coordinated hub/worker updates.

## ADR-0013: Publication guard over visible work and history — accepted

Keys, tokens, runtime files, and backups remain ignored. The quality command adds a
bounded scan of nonignored Git-visible work, index blobs, and reachable historical
blobs. Known sensitive paths/credential forms fail with path/category only. It never
opens ignored private material or echoes a match. Mutation tests cover staged and
historical leakage after cleaning the working file. This heuristic complements staged-
diff review; it cannot prove the absence of arbitrary encoded secrets.

## ADR-0014: Encrypted device relay and scoped enrollment — accepted

The owner authorized bidirectional files between Ubuntu/Debian PCs on different networks,
with the Oracle VPS as relay, GitHub publication and private destination setup material.
Use standard age encryption on the sender, recipient-local verified publication, bounded
VPS ciphertext retention and durable offsets. Files never become model instructions or
automatic executable tasks. Registry entries bind independent bearer hashes to device IDs
and public encryption recipients. The destination receives a new restricted SSH identity,
not the admin key, owner bearer or AI key. Basic multiple diagnostic workers are needed
for this scope; general multi-agent autonomy remains planned. Keep task protocol version 2;
the file API is additive HTTP and a separate five-second inbox loop preserves heartbeat
responsiveness. Desktop, browser and social-account actions are future scope.


## ADR-0015: Authorized native clients and typed service slices - accepted

The owner requested Linux Rust/Slint, Android-only Kotlin, a persistent daily chat/activity
hub, YoutubeBot preservation/improvement and Android USB foundations. Implement these
narrow slices without claiming complete later roadmap phases. Ordinary chat is durable;
only explicit supported actions execute. Separate app credentials avoid distributing the
owner-admin or provider key. Slint interpreter embeds native UI while preserving the
workspace unsafe-code forbid policy; generated FFI stays inside third-party dependencies.

## ADR-0016: Existing YoutubeBot remains the video implementation - accepted

Keep its extension/generation/upload core unchanged and expose a fixed bounded JSON CLI.
Durable operation identity, a separate job ledger and status-only reconciliation fence
external effects. Heavy actors use independent fixed user units rather than inheriting
the diagnostic worker's 512 MiB cap. Acceptance is distinct from actual job completion;
only verified final observations produce completion notices. Local-only data is privately
archived/segmented and staged without overwriting new source or existing destination data.

## ADR-0017: Sensitive screens use endpoint encryption and ephemeral RAM - accepted

The owner requested remote SMS-screen viewing as an Android foundation example. Phone
pixels never enter normal artifacts, SQL, task text, OCR or an external model. An explicit
finite owner grant binds a registered USB device and recipient key. The worker encrypts
raw PNG in memory using standard age, and the hub holds bounded ciphertext at most
60 seconds for one retrieval. App close/cancel revokes it. The desktop decrypts in memory;
Android mobile private-image rendering remains absent. Visual change is not semantic SMS
arrival detection, and Android lock/secure-window protections remain enforced.

## ADR-0018: Shared native chat presentation and explicit scroll intent - accepted

The owner requested compact mobile controls, bottom navigation, official Material icons,
flat outlined buttons, distinct right-gold/left-dark bubbles, and the same visual concept
on Linux with appropriate sidebar navigation. Reuse the native Kotlin/Slint clients and
the pinned local icon assets; do not add another UI runtime or copy a messenger's code.

A bounded client timeline adapts newest-first API pages to chronological display. Hub
opening always requests the bottom of its selected view. An accepted own send resets to
today, all kinds and empty search, then jumps to the actual measured bottom after a
fresh page even if rows are unchanged. Explicit bottom intent is distinct from incoming
refresh and older-page prepend: reading anchors, composer focus and RAM drafts survive
those passive updates. Raw page counts, stable-ID deduplication, sensitive filtering,
and a 200-row cap keep pagination correct and bounded.

The canonical [native-client contract](NATIVE_CLIENTS.md) records these transitions and
its official Telegram source reference. End/unread/anchor distinctions are behavioral
reference material only; the owner's always-end-on-open preference is authoritative.
No full Telegram/WhatsApp parity or unsupported messaging features are claimed.
Tab draft/search retention remains process-memory-only. API, model policy, credentials,
worker grants, screenshot protection and the private-view lifecycle are unchanged.


## ADR-0019: Bounded personal chat, inert shared tasks and private mobile images - accepted; routing superseded by ADR-0021

The following records the original route decision. ADR-0021 supersedes its Console-only
personal routing; the shared-task, generation and private-view contracts remain accepted.

The owner explicitly expanded the native service slice. Reuse the existing durable task
transport, authenticated apps, YoutubeBot actors and RAM private-view relay. Known owner
commands use a deterministic fast path; arbitrary conversation uses a separate Console
Personal route, one call without tools/thinking/retry, <=4 clipped ordinary context turns
and the same aggregate durable spend admission. Only DeepSeek V4.1 Flash is eligible.
Go remains coding-only; lack of Console balance/permission is reported without fallback.

The common area stores inert tasks, authenticated authors, bounded comments and encrypted
transfer IDs. MCP stdio exposes four stable compact tools; list summaries before fetching
a body. Revision/idempotency fences prevent stale overwrites but do not promise exactly-once
external execution. The other Codex requires the owner's current execution request.

Video chat commands authorize generation-only actors. Existing scheduled publication stays
separate. Positive browser evidence distinguishes missing login from unknown state; raw
browser/provider errors never enter chat. Native mobile views generate a dedicated local
age identity protected by Keystore, register only its public key under an explicit app
scope and decrypt with kage. No new plaintext endpoint, phone permission or image SQL/file
is introduced. Explicit fixed messaging-app navigation records only verified local recipes.
Runtime verification and remote-PC/device limitations belong to the acceptance evidence.


## ADR-0020: Semantic computer use and explicit social drafts - accepted; routing superseded by ADR-0021

ADR-0021 supersedes only the Personal/Console route below; the bounded backend,
independent grants, evidence and social draft decisions remain accepted.

The owner explicitly requested Linux/browser control from native chat, a BitLupa research
example, social-management foundations, private computer screenshots and GitHub branding.
Reuse Zaturo's durable hub/worker/provider/native layers. Choose pinned computer-use-linux
(MIT) for AT-SPI/window semantics and Microsoft Playwright MCP (Apache-2.0) for a dedicated
browser; do not embed OpenClaw's duplicate gateway or an unrestricted coding/REPL harness.
Personal inference remains exclusive DeepSeek Console and never receives screenshots.

Planning stays on the home worker with its existing provider key and separate bounded
profile/ledger. The work PC executes exact typed child steps under its current parent
lease, local grants, target/snapshot binding and emergency stop. A model cannot approve
input or publication: native owner commands confirm the exact observed element/draft.
Device endpoints cannot create unrelated/paid parent tasks. Pausing/ending/expiring the
parent revokes children. Unknown external effects are not automatically retried.

The social connector uses Postiz's documented API and its external scheduler, not a new
hub scheduler. Inert local drafts, exact hashes/previews and private JSON intent records
preserve uncertainty. Unsupported platform/comment operations remain unsupported. A200
acceptance does not establish publication; independent status evidence is required.

BitLupa currently has schema/catalogs but no enabled collector/API/database. Preserve its
original repository and use a narrow read-only external-database compatibility adapter.
Its fixed SELECTs are an external-schema transport exception to the hub/worker SQL rule:
all Zaturo persistence, policy and approval SQL remains in zaturo-db. No original private
migration is copied into public fixtures, no live collector is fabricated, and X7base
never silently resolves to X7Pro. This is stored-offer search, not completed monitoring.

Private PC capture reuses the endpoint-encrypted consume-once RAM relay and locally fixed
window grant. Only an explicit nonnegated owner request can add a completion capture hint;
native clients auto-request it only for their current in-memory pending task while visible.
An old feed hint can request a new current-window view, never a historical proof. OS/
renderer erasure and same-user app restrictions are not claims of a complete sandbox.


## ADR-0021: Explicit Personal Go subscription route - accepted

The owner explicitly requests the Go API for native chat and computer planning, following
OpenTatu's connection pattern and without additional Console credits. This supersedes the
Console-only portions of ADR-0019/0020. Preserve Personal classification rather than calling
personal traffic public coding. Go Personal requires allow_personal_go=true (defaultfalse),
allowed_privacy=[personal], max_attempts=1, Global consent and only deepseek-v4.1-flash.
The existing public-coding diagnostic remains two-step; Personal planning is bounded by
min(max_task_model_calls,8), ordinary chat one call, and the shared durable aggregate ledger.

The read-only reference is [OpenTatu at05e8bb3](https://github.com/AlexandreZanata/OpenTatu/tree/05e8bb33b9256eff35d5b6bc05ca8dee067a35c3).
Reuse its Go endpoint/Bearer environment pattern, not its Muse Spark models, training
consent, 900-second timeout or error handling. ZaturoBot identifies itself and uses its own
stable conversation session. Current provider documentation describes coding-agent-style
traffic; this is the owner's explicit local workload decision, not a new provider promise
of arbitrary-personal eligibility, quota circumvention or an OpenCode impersonation.

Personal Go acceptance requires typed billing_source go/go-plus and reported actual cost
zero. Allowance consumption remains separate; a missing charge is not zero. Keep Extra
Usage disabled: the provider can otherwise use balance after plan limits. A post-response
check cannot prevent/refund a charge if provider settings change. The supplied account
screenshot shows Extra Usage off and zero credit at that time; neither local opt-in nor
that image proves future billing. No Console fallback, top-up or automatic charged retry.

Preserve historical Console HTTP402 evidence. It describes the former route, not the new
Go route's prerequisite or live success. Record actual Go/native/task/billing acceptance
separately after observing it; fixtures do not establish the owner's live subscription,
accounts or target PC. See [the provider contract](LLM_PROVIDERS.md).


## ADR-0022: Real installed execution and consented captures - accepted

The owner explicitly authorized the work-PC end-to-end slice, including installed
OpenCode execution, one new generation-only two-video acceptance batch, real public
browser use and native private PC/USB captures. Preserve the old scheduled media request
as a separate operation. Its later explicit resume authorization supersedes its previous
pause, without authorizing arbitrary process signalling or relaunch.

Normalize known aliases before inference; interpret unknown messages through one bounded
Personal Go call and validate typed proposals against current owner text. Known generation
uses the installed OpenCode tool-selection call rather than the old direct dispatch.
Use its actual V2 interface/default-deny policy, a random-header loopback one-forward
budget gateway (upstream retries are otherwise insufficiently bounded), and one exact MCP
job tool. The installed protected integration stays local; no home key is copied. Preserve
DeepSeek flash exclusivity, aggregate budgets, one attempt, Go billing proof, zero reported
additional cost and no fallback. The owner's reported zero balance is not verification
of the current Extra Usage setting. See [the execution contract](OPENCODE_EXECUTION.md).

Wayland gets a separate explicit native ScreenCast/PipeWire path. The portal authorizes
one window for one nonpersistent live session; monitor/restore/fallback are forbidden.
Keep the old fixed-tuple X11 guard. The source-bound remote FD, bounded PNG appsink and
existing age relay keep pixels off disk, model and ordinary history. This is source
consent, not a claim that portal selection proves a legacy PID/window tuple.

The same phone cannot both foreground its FLAG_SECURE Hub and display a nonsecret screen
for USB capture. Add an explicitly native-confirmed prepared one-shot capture:15-second
fixed delay, original viewer/recipient, total expiry<=60s. Only opaque consent metadata
survives leaving Hub; there is no background image download. The owner explicitly opens
the prepared image after returning. Focus/navigation loss still clears every active
image and cancels normal view sessions. No model/MCP can manufacture native consent.

The owner directed administrative hub deployment and same-signature Android build to the
main PC. Publish only tested source and an inert revision-pinned shared handoff; preserve
this work PC's restricted forwarding key and all enrollment/application data. Actual
physical/account acceptance remains separate from synthetic tests and source status.
