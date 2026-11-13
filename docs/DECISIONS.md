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

## ADR-0006: Provider and UI deferral — accepted

The vertical slice needs no LLM, dashboard, or desktop. Their contracts remain planned.
Provider model IDs are candidates until official verification, data-policy authorization,
and workload eligibility are complete. API/CLI validation comes first.

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
