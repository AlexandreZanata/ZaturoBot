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
