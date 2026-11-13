# Persistence instructions

Read the root instructions first. All SQL belongs here. HubStore and WorkerJournal use separate SQLite files, WAL, foreign keys, bounded queries, and explicit schema versions. Repository transactions persist caller-decided state transitions atomically with events; never invent business rules. Add migrations instead of editing deployed schemas. Test rollback, reopening, schema version rejection, and journal operation ownership. See docs/MIGRATIONS.md.
