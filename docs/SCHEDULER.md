# Scheduler

Status: planned for Phase 6. The foundation's queue eligibility, deadlines, bounded
retries, and lease maintenance are task-engine behavior, not a recurring-job scheduler.
No cron, overnight window, dependency graph, or notification service is enabled.

A future schedule stores an IANA timezone (`America/Cuiaba` initially), local execution
window, next eligible UTC instant, task template, dependency IDs, concurrency cap,
budget scope, and interruption policy. Use explicit DST/ambiguous-time handling even
when the owner's current timezone has no seasonal transition. The scheduler performs
ordinary deterministic calculations and never needs continuous model inference.

At a window boundary, stop new dispatch and checkpoint/cancel owned work according to
its tool contract. Human approvals pause work safely; an overnight window grants no
additional permissions. Persist schedule advancement and enqueue idempotency in one
transaction. Reconnect must not create duplicate occurrences. Budget reservations must
prevent concurrent tasks from overspending a daily or monthly allowance.

Dependencies: durable task engine, recurrence-time library review, approvals, budget
accounting, and notifications. Threats: clock jumps, repeated occurrence creation, stale
grants, runaway backlogs, and impossible dependencies. Acceptance uses an injected clock
for timezone boundaries, missed windows, restart, owner interruption, and budget exhaustion.
