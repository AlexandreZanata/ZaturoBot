# Architecture

The hub and worker are independently restartable Rust composition roots in one modular
workspace. The hub coordinates; the worker executes. They never synchronize SQLite
files. Versioned JSON messages and durable operation identifiers connect their state.

## Runtime flow

The owner CLI sends an authenticated task request to the hub. Services validate and
persist it in SQLite. An authenticated outbound worker WebSocket registers a worker ID
and capabilities. Services claim an eligible task under a lease. The worker journals
intent, executes the permitted direct diagnostic or bounded model-assisted diagnostic,
journals the result, and reports it. The hub
validates ownership, lease, state, and result before committing completion and history.
The client can disconnect at any point; it does not own execution lifetime.

## Responsibility boundaries

`zaturo-core` owns domain state and typed diagnostic contracts. `zaturo-protocol` owns
wire envelopes. `zaturo-db` owns hub persistence and the separate worker journal.
`zaturo-services` owns task use cases. `zaturo-security` owns configuration and policy.
`zaturo-tools` owns deterministic execution. `zaturo-llm` owns provider transport, routing,
request bounds and cancellation; `zaturo-orchestrator` owns the strict planning loop.
`zaturo-hub` composes HTTP, WebSocket, CLI, and services. `zaturo-worker` composes reconnect,
journaling, durable spend admission, policy, models, and tools.
`xtask` is development tooling. Exact allowed internal edges are in
[architecture.toml](../architecture.toml), checked against Cargo metadata. A worker-to-hub development-only dependency composes
real loopback integration tests; production worker code cannot depend on the hub
([ADR-0008](DECISIONS.md)).

One serialized SQLite connection per database avoids a large pool. Queries and pages
are bounded. The production listener caps admitted TCP sockets before HTTP/authentication
and retains admission permits through upgrades. Dispatch uses in-memory notifications
with a 5-second durable-state fallback; notifications are never the persisted queue.
WAL, foreign keys, migrations, and transactions protect related records.
The direct diagnostic has no external side effects; recovery may repeat it safely.
Model calls are external paid requests: durable reservations retain uncertain spend,
and a lost response cannot be treated as proof that nothing happened. Complete assisted
outcomes can be resent from the worker journal; unknown paid requests stop for
reconciliation. Future side-effecting tools require their own reconciliation contract.

## Deliberately absent modules

Do not create empty `scheduler`, vault, desktop, integration, or transport plugin crates.
Introduce a crate when a concrete implementation justifies its boundary. Planned APIs
are in their topic documents. The initial API/CLI takes priority over a dashboard.

OpenTatu informed the concise instructions, recipe map, explicit dependency manifest,
and executable repository checks. Its single-process investigation pipeline and provider
implementation are reference material, not ZaturoBot requirements or copied behavior.
