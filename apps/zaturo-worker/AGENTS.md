# Worker composition and transport

Own outbound authenticated WebSocket connections, strict local runtime configuration,
permission checks, and durable execution-journal coordination. Tools live in
`zaturo-tools`; SQL lives in `zaturo-db`. Plain and model-assisted diagnostics are the
only task kinds. `execution.rs` adapts the durable ledger and journal to the model router.
Never select a fake client from a production configuration or expose provider keys to the hub.
Wait for hub Ready before dispatch. Record intent before execution and result before
reporting. An unknown side effect must never be blindly retried. Read
[recovery](../../docs/TASK_LIFECYCLE.md) before editing the connection loop.
Keep the socket responsive while execution runs. Owned jobs must cancel on revocation
or disconnect; persisted reservations fence replay even after abrupt process death.
