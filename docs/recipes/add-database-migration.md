# Add a database migration

Read [migrations](../MIGRATIONS.md). Identify whether the hub or worker owns the data;
there is no shared synchronized SQLite file.

Add the next append-only SQL migration under `zaturo-db`, preserving prior released
files. Update schema-version application, registry/documentation as required, and model
conversion code. Keep related updates transactional and preserve audit-critical evidence.
Document preconditions, transformations, old/new compatibility, backup, and restore path.

Test clean creation, upgrade from the previous schema with meaningful rows, failed
migration rollback, foreign keys, reopen persistence, and repeated application. Use
temporary databases. Never run migrations against the owner's live databases in tests.
Update [migration inventory](../MIGRATIONS.md), run focused db tests and `cargo xtask check`.
