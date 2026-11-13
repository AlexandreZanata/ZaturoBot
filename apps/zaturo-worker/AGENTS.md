# Worker composition and transport

Own outbound authenticated WebSocket connections, strict local runtime configuration,
permission checks, and durable execution-journal coordination. Tools live in
`zaturo-tools`; SQL lives in `zaturo-db`. The diagnostic is the only executable task.
Wait for hub Ready before dispatch. Record intent before execution and result before
reporting. An unknown side effect must never be blindly retried. Read
[recovery](../../docs/TASK_LIFECYCLE.md) before editing the connection loop.
