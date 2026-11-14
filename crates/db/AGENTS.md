# Persistence instructions

Read the root instructions first. All SQL belongs here. HubStore and WorkerJournal use
separate SQLite files, WAL, foreign keys, bounded queries, and explicit schema versions.
Repository transactions persist caller-decided state transitions atomically with events;
never invent business rules. Core owns quota evaluation; the budget repository supplies
transactionally read totals and commits its decision.

- `repository.rs`: hub task, event, worker and queue queries; schema version 5, upgraded from 1/4.
- `transfers.rs`: bounded file metadata, durable offsets, quota totals and transfer audit.
- `migrations/0004_hub_transfers.sql`: hub version 1 to 4, preserving task/worker evidence.
- `journal.rs`: local intents, one cached diagnostic, and complete assisted evidence.
- `model_budget.rs`: worker reservations, actual usage, uncertainty and cancellation.
- `migrations/0003_worker_model_budget.sql`: additive worker version 2 to 3 migration.

A repeated reservation never authorizes another network call. Unknown calls keep the
full reservation, including across restart and cancellation. Record overrun debt before
returning a budget failure. Cache a matching tool result before final assisted evidence.
Add migrations instead of editing deployed schemas. Test rollback, reopening, migration,
operation ownership, quota races and unknown-result recovery. See
[migrations](../../docs/MIGRATIONS.md).

Hub schema5 adds indexed activity/chat and service-state dedupe; worker schema6 adds a
separate service intent/result journal with fair polling timestamps. Preserve the model
ledger unchanged. Private screen frames never enter SQLite. Seed only metadata notices
with stable keys; do not treat job acceptance as video completion.
