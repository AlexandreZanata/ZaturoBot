# Transfer infrastructure

Own bounded spool I/O, streaming age encryption/decryption, and the device HTTP client.
SQL and quota/ownership rules belong to db/services. Never read a model credential,
invoke a shell, execute received files, or send file bytes to an LLM. Validate UUID
object paths, reject symlinks, and publish downloaded plaintext without overwriting.
Encrypt on the sender before upload; the hub never receives decryption identities.
Use temporary roots and generated age identities in tests. See
[file transfers](../../docs/FILE_TRANSFERS.md) for recovery and limits.
