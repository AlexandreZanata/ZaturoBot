# Application service instructions

Read the root instructions first. Engine is the single entry point for task and worker
use cases. Task transitions, lease checks, acceptance, retry limits and queue limits
belong here or in pure core rules. Use HubStore transactions; never add SQL. Every state
transition and its event must commit together. Inject UTC milliseconds into all mutations.

Plain diagnostic completion and assisted completion are separate kind-gated contracts.
Assisted tasks require the owner's explicit public-coding policy and bounded model
evidence. Worker failures are typed categories, never untrusted provider text. Preserve
operation IDs across retries; the worker's budget journal must reconcile uncertain
model requests instead of repeating them. Claims must skip unsupported task capabilities
without starving eligible queued tasks. Test stale messages, migration compatibility,
failure fencing, disclosure policy and restart behavior before changing recovery.

`transfers.rs` owns file admission/idempotency, ownership, quota reservations, progress,
delivery and expiry. Infrastructure verifies/writes/fsyncs bytes before checkpointing;
remove expired bytes before releasing quota. File payloads never become executable tasks.
