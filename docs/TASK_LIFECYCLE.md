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

Not every legal domain transition has an API in the current implementation. In particular, approval records
and approval resolution are planned. The diagnostic follows creation, queue, assignment,
running, and validated completion; it never requests a human approval.

## Durable task record

A task has task/operation UUIDs, original request, kind, target worker, required
capabilities, state, priority, tool/output budget, retry count/cap, created/updated/deadline
UTC times, assigned worker, lease token/expiry, optional diagnostic result, optional
assisted orchestration evidence, optional data policy, and failure reason.
The diagnostic permits one tool call with a bounded result. `NewTask` limits request
size, priority, attempts, timeout, worker identifier, and idempotency-key length. The
foundation timeout is a total creation-to-completion deadline of 1–300 seconds, not a
fresh per-attempt allowance. Attempts are capped at three. Long-running checkpointed
jobs require a later deadline/budget contract extension.
Progress is append-only state-event evidence. Artifact and approval records are future
extensions; the diagnostic produces an inline bounded result rather than a file artifact.

Equivalent repeated creation with an idempotency key returns the existing task; a
conflicting payload fails. Assignment increments attempts and creates a fencing lease.
A worker must register the required task capability before claim: `diagnostic.v1` for
a direct diagnostic, or `agent.diagnostic.v1` for an assisted diagnostic. Heartbeats
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

## Assisted diagnostic and spend recovery

`assisted_diagnostic` requires the owner's explicit `public_coding` data policy. The
worker advertises its assisted capability only when a validated real model route is
configured; a fake client is not a production substitute. A successful assisted outcome
retains the direct typed diagnostic plus the selected model, bounded summary, step/tool
counts, and validated usage. Typed worker failures are fenced by the same worker/task/
lease checks; a stale failure cannot terminate newer work.

The separate worker ledger reserves each paid attempt before transmission. Token and
call admission are atomic across request/task/UTC-day/UTC-month scopes. Pending and
uncertain requests remain charged after restart. A response that exceeds its reservation
records debt and prevents further affected model dispatch. Final assisted outcomes are
journaled before reporting; reconnect can resend that evidence without another model
request. An unresolved paid request without a cached final outcome fails safely for
reconciliation rather than claiming zero spend or blindly restarting the model loop.

Pause/cancel prevents subsequent model/tool dispatch. It cannot revoke a request already
accepted by the provider, so cancellation preserves the corresponding reservation.
Long-running jobs, approval resolution, richer tools, and arbitrary project work remain
later capabilities, not consequences of the new task kind.
