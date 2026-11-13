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
