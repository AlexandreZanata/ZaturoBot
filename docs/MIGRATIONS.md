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

Enable WAL, foreign keys, and bounded busy handling for file databases. Apply migrations
transactionally and record schema version. Migration tests create a new database, reopen
it, and verify previously committed tasks/events/journal entries remain usable.

Before any production schema upgrade, stop or quiesce writers, create a consistent
backup, and verify supported old-to-new migration. Do not edit a migration already used
in a released installation. Add an append-only migration and document data transformation,
compatibility, expected duration, and rollback/restore implications. Downgrading a binary
is not automatically a supported schema downgrade. See
[add-database-migration](recipes/add-database-migration.md).
