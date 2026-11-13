# Architecture

The hub and worker are independently restartable Rust composition roots in one modular
workspace. The hub coordinates; the worker executes. They never synchronize SQLite
files. Versioned JSON messages and durable operation identifiers connect their state.

## Runtime flow

The owner CLI sends an authenticated task request to the hub. Services validate and
persist it in SQLite. An authenticated outbound worker WebSocket registers a worker ID
and capabilities. Services claim an eligible task under a lease. The worker journals
intent, executes the permitted diagnostic, journals the result, and reports it. The hub
validates ownership, lease, state, and result before committing completion and history.
The client can disconnect at any point; it does not own execution lifetime.

## Responsibility boundaries

`zaturo-core` owns domain state and typed diagnostic contracts. `zaturo-protocol` owns
wire envelopes. `zaturo-db` owns hub persistence and the separate worker journal.
`zaturo-services` owns task use cases. `zaturo-security` owns configuration and policy.
`zaturo-tools` owns deterministic execution. `zaturo-hub` composes HTTP, WebSocket,
CLI, and services. `zaturo-worker` composes reconnect, journaling, policy, and tools.
`xtask` is development tooling. Exact allowed internal edges are in
[architecture.toml](../architecture.toml), checked against Cargo metadata. A worker-to-hub development-only dependency composes
real loopback integration tests; production worker code cannot depend on the hub
([ADR-0008](DECISIONS.md)).

One serialized SQLite connection per database avoids a large pool. Queries and pages
are bounded. The production listener caps admitted TCP sockets before HTTP/authentication
and retains admission permits through upgrades. Dispatch uses in-memory notifications
with a 5-second durable-state fallback; notifications are never the persisted queue.
WAL, foreign keys, migrations, and transactions protect related records.
The diagnostic has no external side effects; recovery may repeat it safely. Future
side-effecting tools require an intent/result ledger and explicit reconciliation.

## Deliberately absent modules

Do not create empty `scheduler`, `orchestrator`, `llm`, or transport plugin crates.
Introduce a crate when a concrete implementation justifies its boundary. Planned APIs
are in their topic documents. The initial API/CLI takes priority over a dashboard.

OpenTatu informed the concise instructions, recipe map, explicit dependency manifest,
and executable repository checks. Its single-process investigation pipeline and provider
implementation are reference material, not ZaturoBot requirements or copied behavior.
