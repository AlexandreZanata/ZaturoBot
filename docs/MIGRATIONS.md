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


`crates/db/migrations/0005_hub_activity.sql` upgrades hub 4 to 5 (or composes 1 -> 4 -> 5)
with indexed feed items, chat idempotency, operation lookup and service-update deduplication.
Task/file rows are preserved; existing terminal tasks seed metadata-only notices once by
stable dedupe key. Sensitive frames are never SQL rows. Back up/quiesce before upgrading;
a 0.3 hub needs its matching schema-4 backup to roll back.

`crates/db/migrations/0006_worker_services.sql` upgrades worker 3 to 6 (or 2 -> 3 -> 6)
with a separate service intent/result journal. Existing diagnostic/model tables and charged
reservations remain unchanged. Unknown side-effecting service intents reconcile via status
rather than resubmit. Screenshot frames never enter this journal. Rollback requires the
consistent schema-3 snapshot and old worker binary, not merely a binary replacement.


`crates/db/migrations/0007_hub_shared.sql` upgrades hub 5 to 7 (or composes 1/4 upgrades)
with shared tasks, bounded revision notes, authenticated request dedupe and public native
viewer recipients. Existing tasks, feed, transfers and app identities remain unchanged.
`crates/db/migrations/0008_worker_assistant.sql` upgrades worker 6 to 8 (or composes 2/3)
with separate personal-call reservations. Both call ledgers count toward route admission's
global UTC-day/month totals. Pending/unknown/overrun charges remain retained. The existing
operation/service tables and their uncertainty fences are preserved. Schema numbers use
the workspace migration inventory, not interchangeable database identities.

Back up/quiesce each writer before deployment. Failed schema creation rolls back the entire
store upgrade. New binaries accept the documented old versions; old binaries do not accept
7/8. Rollback needs the matching pre-upgrade consistent database and compatible binary,
with reconciliation of any newer work; never clear a journal merely to downgrade. These
upgrades create local tables/indexes and do not bulk rewrite owner records or images.


## Hub 7 to 9: bounded computer coordination

`crates/db/migrations/0009_hub_computer.sql` adds `computer_steps` with a parent task foreign
key, unique parent/sequence and a1..8 bound. Bodies retain the exact proposed action,
owner-approval state and child identity; no screenshot bytes or provider credentials.
HubStore upgrades old1/4/5/7 transactionally to9. Schema7 public recipients/shared tasks
and previous task/feed/transfer data remain intact. Worker schema remains8, including
all old service/model uncertainty fences. Rollout must back up SQLite consistently before
starting the new hub; never restore an old binary over a migrated live DB automatically.
Tests cover old data preservation and full rollback on a conflicting table.


The installed executor adds typed service JSON variants and extends the existing schema8
service-observer whitelist; it introduces no competing SQL store or schema migration.
Private fsynced OpenCode intents/process/billing/tool/media receipts remain under the
explicit local0700 state root. Preserve them together with the worker journal for recovery.
