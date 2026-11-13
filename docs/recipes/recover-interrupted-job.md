# Recover an interrupted job

Read [lifecycle](../TASK_LIFECYCLE.md). Before retry, establish whether the operation is
safe to repeat and whether durable intent/result evidence exists.

The hub reopens its own database, preserves pending tasks, and reconciles leases against
current UTC time. The worker reopens its separate journal. A completed matching operation
can report its persisted result under the current assignment rules. An intent without a
result is uncertain. Only the foundation's read-only deterministic diagnostic is safely
repeatable; future external effects need tool-specific lookup or owner reconciliation.

Reject stale lease tokens and mismatched operation IDs. A new connection alone does not
transfer authorization. Paused/cancelled/terminal tasks cannot become running because an
old message arrives. Record any accepted recovery transition in durable history.

Use restart tests with temporary files to reproduce the exact boundary: before intent,
after intent, after result, before acknowledgment, or after hub commit. Never edit the
database to conceal an unresolved effect. Report the remaining uncertainty explicitly.
