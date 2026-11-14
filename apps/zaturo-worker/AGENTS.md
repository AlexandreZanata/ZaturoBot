# Worker composition and transport

Own outbound authenticated WebSocket connections, strict local runtime configuration,
permission checks, and durable execution-journal coordination. Tools live in
`zaturo-tools`; SQL lives in `zaturo-db`. Plain/model-assisted diagnostics and explicitly authorized fixed service requests are
the supported task kinds. `execution.rs` adapts the durable ledger and journal to the model router.
Never select a fake client from a production configuration or expose provider keys to the hub.
Wait for hub Ready before dispatch. Record intent before execution and result before
reporting. An unknown side effect must never be blindly retried. Read
[recovery](../../docs/TASK_LIFECYCLE.md) before editing the connection loop.
Keep the socket responsive while execution runs. Owned jobs must cancel on revocation
or disconnect; persisted reservations fence replay even after abrupt process death.

`files.rs` runs a separate private inbox loop; `zaturo-transfer` owns crypto and file
transport. Received bytes are inert and never enter task/model execution. Use a unique
local native age identity and private per-transfer inbox paths. Work-PC enrollment has
neither provider nor owner/admin credentials. See [files](../../docs/FILE_TRANSFERS.md).

Owner-authorized services now use `service_execution.rs` with separate durable fences.
`service_jobs.rs` observes actual heavy-job state independently of submission. Youtube
uses its existing Python project and separate fixed resource units. `private_views.rs`
requires an exact hub grant and registered owner USB lease, encrypts raw PNG in memory
and uploads only ephemeral ciphertext. It never stores screenshots in its journal or
file cache. Keep the exclusive public-coding model route and diagnostic tool policy intact.
