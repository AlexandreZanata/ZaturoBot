# Observability

The hub exposes health information and persists task events with IDs and UTC timestamps.
The owner can list tasks and retrieve a bounded event history through authenticated
interfaces. Structured logs should identify connection/reconnect and error categories
without including bearer tokens or arbitrary request contents.

Use task ID, operation ID, worker ID, and assignment/lease identity as correlation keys.
Persist durable lifecycle evidence before acknowledging success. Task state and acceptance
evidence determine completion; a successful HTTP response or process exit alone does not.
Distinguish failed, cancelled, paused, waiting approval, retry pending, and active work.
An unreachable worker is device state, not automatic proof that its operation failed.

No dashboard, notification pipeline, token-usage telemetry, provider selection, or tool
process log collection is implemented for absent subsystems. When those features arrive,
add bounded fields for model ID, usage, approval decision, tool result, retry cause, and
artifact references, without private chain-of-thought or secrets.

## Retention and investigation

Do not silently delete audit-critical task transitions. The foundation retains durable
history; automated retention/archival is planned. Use pagination and monitor database/disk
growth. Configure journald rotation for transient service logs separately. Artifacts need
future per-task size/retention rules and deletion references that preserve audit metadata.

For failures, retrieve task and event pages, compare lease/operation identities, inspect
worker journal state, and classify the cause as input, authorization, unavailable worker,
transport, expired lease/deadline, execution, or persistence. Follow
[debug-failed-task](recipes/debug-failed-task.md); never print the environment for diagnosis.
