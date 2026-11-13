# Add a task type

Read [lifecycle](../TASK_LIFECYCLE.md) and search for the diagnostic path. Define typed
input, capabilities, resource/deadline/retry budget, deterministic acceptance criteria,
and artifact/result contract in core. Do not create a state machine in an HTTP handler.

Decide whether repeating the operation is safe. Side-effecting tasks require an intent
ledger and reconciliation evidence before retry. Reuse create/claim/event persistence,
add a typed executor, register task/tool IDs, and update protocol compatibility deliberately.

Test capability mismatch, acceptance failure, stale assignment, cancellation, restart,
idempotent completion, and bounded retry. Update lifecycle, components, registry, and
threat model. Run focused tests and `cargo xtask check`. New task registration never
implies permission to perform its external effects.
