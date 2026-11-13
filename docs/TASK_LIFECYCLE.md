# Task lifecycle

<!-- task-states: CREATED QUEUED ASSIGNED RUNNING WAITING_APPROVAL PAUSED RETRY_PENDING COMPLETED FAILED CANCELLED -->

`zaturo-core::TaskState` and its transition function are the canonical state machine.
The marker above is checked against that definition. Persist timestamps as UTC Unix
milliseconds. Inject current time into service operations and tests.

## States and legal transitions

- `CREATED` → `QUEUED` or `CANCELLED`. Creation and enqueue are recorded transactionally.
- `QUEUED` → `ASSIGNED`, `PAUSED`, `CANCELLED`, or `FAILED`.
- `ASSIGNED` → `RUNNING`, `RETRY_PENDING`, `PAUSED`, `CANCELLED`, or `FAILED`.
- `RUNNING` → `WAITING_APPROVAL`, `PAUSED`, `RETRY_PENDING`, `COMPLETED`, `FAILED`, or `CANCELLED`.
- `WAITING_APPROVAL` → `QUEUED`, `PAUSED`, `CANCELLED`, or `FAILED`.
- `PAUSED` → `QUEUED`, `CANCELLED`, or `FAILED`.
- `RETRY_PENDING` → `QUEUED`, `PAUSED`, `CANCELLED`, or `FAILED`.
- `COMPLETED`, `FAILED`, and `CANCELLED` are terminal.

Not every legal domain transition has an API in Phase 1. In particular, approval records
and approval resolution are planned. The diagnostic follows creation, queue, assignment,
running, and validated completion; it never requests a human approval.

## Durable task record

A task has task/operation UUIDs, original request, kind, target worker, required
capabilities, state, priority, tool/output budget, retry count/cap, created/updated/deadline
UTC times, assigned worker, lease token/expiry, optional result, and failure reason.
The diagnostic permits one tool call with a bounded result. `NewTask` limits request
size, priority, attempts, timeout, worker identifier, and idempotency-key length. The
foundation timeout is a total creation-to-completion deadline of 1–300 seconds, not a
fresh per-attempt allowance. Attempts are capped at three. Long-running checkpointed
jobs require a later deadline/budget contract extension.
Progress is append-only state-event evidence. Artifact and approval records are future
extensions; the diagnostic produces an inline bounded result rather than a file artifact.

Equivalent repeated creation with an idempotency key returns the existing task; a
conflicting payload fails. Assignment increments attempts and creates a fencing lease.
A worker must register the required `diagnostic.v1` capability before claim. Heartbeats
renew only that worker's active valid lease. Only matching worker, task, lease, state,
and accepted typed result can complete work. Exact duplicate completed reports are
idempotent; stale or conflicting reports cannot overwrite durable completion.

## Recovery and owner control

The hub retains pending work across reopen. Expired active leases enter `RETRY_PENDING`
and can requeue only while attempt/deadline budget remains; exhausted work fails with a
reason. An operation ID stays stable across retries. The worker journal persists intent
before execution and result before report. A cached matching result can be reused; an
unresolved diagnostic intent may repeat because it has no external effects. This rule
must not be generalized to future uploads, messages, purchases, commits, or account changes.

Pause revokes current assignment and blocks dispatch until resume requeues it. A
subsequent assignment consumes the next attempt; pause/resume cannot bypass the attempt
budget or original deadline. Cancel
revokes assignment and is terminal. A late worker result cannot revive either state.
There is no owned subprocess in the diagnostic. Future tools must stop dispatch, terminate
owned processes safely, and retain uncertainty when an external effect cannot be undone.
Deadline, queue, pagination, and recovery batching limits remain enforced during reconnect.

Completion is acceptance-based: the diagnostic must identify its operation and tool,
report success on Linux, and provide bounded nonempty architecture/summary fields.
A transport ACK alone is never acceptance evidence. See
[recovery recipe](recipes/recover-interrupted-job.md) and [protocol](COMMUNICATION_PROTOCOL.md).
