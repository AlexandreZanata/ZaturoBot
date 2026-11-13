# Communication protocol

Status: version 2 implemented for the single configured worker. Canonical wire shapes
are `zaturo-protocol`; domain payloads are `zaturo-core`. JSON enum tags use `type` and
snake_case variant names. Unsupported variants/fields fail decoding. WebSocket frames/messages and incoming HTTP bodies
are bounded by `MAX_MESSAGE_BYTES` (16 KiB); authentication never appears in payloads.
HTTP responses use bounded pagination and record sizes and can exceed 16 KiB.

## Owner HTTP API

The owner CLI calls `client_url` using `Authorization: Bearer <owner token>`.

- `GET /healthz`: non-secret liveness response, no authentication required.
- `POST /v1/tasks`: typed `NewTask` including kind, request, target worker, idempotency
  key, priority, max attempts, timeout, and optional explicit data policy. Supported kinds
  are `diagnostic` and `assisted_diagnostic`; the latter requires `public_coding`.
- `GET /v1/tasks?limit=20&offset=0`: bounded task page.
- `GET /v1/tasks/{id}`: one task by UUID.
- `GET /v1/tasks/{id}/events?limit=100&offset=0`: bounded task history page.
- `POST /v1/tasks/{id}/pause`, `/resume`, `/cancel`: owner lifecycle controls.
- `GET /v1/workers`: registered worker metadata; no credential disclosure.

Domain/authentication errors use a bounded JSON category; admission-capacity and
request-timeout failures use bounded HTTP error responses. Authentication failures do
not reveal expected tokens. A worker token cannot use owner endpoints. API authorization and domain
conflicts are separate checks. See [task lifecycle](TASK_LIFECYCLE.md) for idempotency.

## Outbound worker WebSocket

The worker connects to `/v1/worker` with its distinct bearer credential before upgrade.
The hub binds that credential to configured `worker_id`. The worker first sends `hello`
with version 2, worker ID, and capability list. The hub returns `welcome` after validation.
It can then send `assignment` containing a typed task and lease token/expiry.

Worker messages are `heartbeat` (optional active lease), `started` (task/lease), and
`completed` (task/lease/direct result), `completed_assisted` (task/lease/assisted result),
and `failed` (task/lease/typed category). The hub accepts `started` with `ready`; the worker
waits for that confirmation before journaling/executing the tool. `acknowledged` confirms accepted completion, `revoked`
revokes the current task, and `heartbeat` confirms liveness. `error` carries a bounded
category in addition to welcome/assignment. One active task per worker is the
initial concurrency contract. An assisted completion contains typed diagnostic evidence
plus bounded model usage/summary; failures carry categories rather than raw provider
errors. Incompatible protocol versions are rejected during hello/welcome negotiation.
The transport does not transmit credentials or raw shell commands. Remote URLs require
HTTPS/WSS, or literal-loopback HTTP/WS carried inside the verified encrypted SSH tunnel.

The production TCP listener admits at most 32 sockets before HTTP headers or
authentication and closes sockets after 30 seconds without read/write activity. Socket
permits remain held through WebSocket upgrade. HTTP requests are limited to 32 in flight
with a 10-second request deadline. The configured WebSocket connection limit defaults
to eight; hello and writes have 5-second deadlines. Private TLS proxies may add further
connection/header/idle limits.

Creation, owner control, and accepted completion notify dispatch. A 5-second fallback
consults durable state and performs bounded lease recovery. Notifications improve
latency but are not the queue or the recovery source of truth.

Connection loss triggers capped reconnect backoff. It does not prove execution did not
happen. The hub persists task state; the worker persists its own intent/result journal.
Re-registration, lease validation, stable operation identity, and journal reconciliation
prevent a new socket from implicitly authorizing stale work. There is no exactly-once
external-effect guarantee. Future protocol changes require version compatibility tests
and an explicit rollout/migration decision; do not silently repurpose versioned fields.
