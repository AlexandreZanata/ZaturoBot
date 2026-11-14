# Database migrations

`zaturo-db` owns all SQL and schema changes. Hub and worker have separate databases;
never copy or synchronize one database into the other. The engine's migration files
and journal schema are the authoritative executable definitions.

`crates/db/migrations/0001_hub.sql` creates durable task/event, worker, and
idempotency storage for the hub. `crates/db/migrations/0002_worker.sql` initializes the
separate worker operation journal. These are independent schema streams, not two
migrations to apply to the same database. The worker journal records operation intent and
a completed diagnostic result before reporting it, so an acknowledged transport message
is not the only evidence of execution. Each store applies only its own schema migration.

`crates/db/migrations/0003_worker_model_budget.sql` upgrades the worker journal from
version 2 to version 3. It adds durable model reservations/usage and assisted outcome
metadata without altering the independent hub schema version 1. Existing direct
diagnostic results remain readable. Migration tests open a version-2 worker fixture,
upgrade it, and preserve its committed journal evidence.

Pending/uncertain reservations remain charged across restart; missing responses are not
free budget. Rolling back a version-3 worker requires restoring a consistent version-2
backup or a separately designed downgrade. Do not synchronize the worker database to
the hub or treat the two version numbers as one ordered schema stream.

Enable WAL, foreign keys, and bounded busy handling for file databases. Apply migrations
transactionally and record schema version. Migration tests create a new database, reopen
it, and verify previously committed tasks/events/journal entries remain usable.

Before any production schema upgrade, stop or quiesce writers, create a consistent
backup, and verify supported old-to-new migration. Do not edit a migration already used
in a released installation. Add an append-only migration and document data transformation,
compatibility, expected duration, and rollback/restore implications. Downgrading a binary
is not automatically a supported schema downgrade. See
[add-database-migration](recipes/add-database-migration.md).

`crates/db/migrations/0004_hub_transfers.sql` upgrades only hub schema 1 to 4, adding
file metadata/idempotency, checkpoint/quota indexes and a separate transfer event audit.
Task, worker and task-event rows are preserved. New databases apply 0001 and 0004 in
one transaction. Worker schema remains 3. The upgrade is local table/index creation
with no bulk task rewrite. Stop hub writers and take a verified SQLite backup first.
A 0.2 hub refuses schema 4; rollback requires restoring the matching pre-upgrade hub
snapshot and its binary, not merely replacing the executable. Ciphertext spool and
registry configuration must be reconciled separately if rolling back after new uploads.
