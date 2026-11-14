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

## Encrypted device relay

`zaturo-transfer` owns streaming age crypto, opaque filesystem spool and bounded device
HTTP/cache I/O. Core owns transfer contracts/limits; db owns metadata/checkpoint SQL;
services own quota, idempotency, ownership and delivery transitions. Hub composition binds
a registry-authenticated device identity to HTTP/WS and serializes disk/DB checkpoints.
Worker composition checks an independent private inbox every five seconds. Received files
never enter the model/task execution path. Both PCs use outbound restricted pinned SSH;
the hub stores ciphertext and public recipients only. See [file transfers](FILE_TRANSFERS.md).


## Native clients, activity and service adapters (0.4)

The owner explicitly authorized narrow native-client/Youtube/Android vertical slices.
Linux Rust/Slint and Android Kotlin clients share indexed UTC-persisted daily chat/activity;
new app credentials are independent of owner-admin and worker keys. The Android client
owns a restricted pinned SSH tunnel, so no public hub HTTP port is introduced.
`zaturo-youtube` is a fixed subprocess/JSON adapter; Python retains generation/upload.
`zaturo-android` owns serial-bound USB operations and control leases. Worker service
journals fence unknown external effects; independent heavy video units contain resources.
Task protocol 2 is retained through capability-gated additive service variants. Null new
task fields are omitted, so enrolled 0.3 workers retain their existing diagnostic flow.

Normal artifacts use the encrypted file spool and verified endpoint inboxes. Sensitive
screens take a separate age-encrypted RAM-only path, bound to the owner's endpoint key,
expiring/consumed once with no image SQL/spool/LLM access. Private acceptance permits only
fixed metadata and server-known summaries. See [native clients](NATIVE_CLIENTS.md) and
[YoutubeBot](YOUTUBE_SERVICE.md) for operational boundaries and actual limitations.
