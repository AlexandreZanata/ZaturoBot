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

`activity.rs` atomically records chat/idempotent task creation and canonical day/search
filters; terminal tasks publish stable notices. `service_tasks.rs` validates typed service
evidence. Accepted video jobs produce an initiation notice; only actual job observations
produce final completion. Private results require the exact metadata-only core contract.

`shared.rs` owns inert shared-task capacity/idempotency/optimistic revisions. Authenticated
provenance is a transport input, never request content. `chat_as` atomically records the
owner message and bounded command/handoff; only explicit assistant mode invokes personal
chat. Known commands skip inference. Recent conversation context excludes acknowledgements,
service data and every sensitive frame. Unsupported worker capabilities produce an honest
reply rather than a fabricated execution. Generation does not imply publication.


`computer.rs` binds up to8 sequential child steps to a live planner parent. Input review uses the prior observed node/ref; social execution binds the same parent's exact prepared hash/preview. Current executor grants, explicit native approval and parent cancellation are independently enforced.

Registered browser research retains its durable routing origin. Hub validates that origin
on both child creation and executor grant, and permits only public browser reads; neither
model output nor native approval expands it into desktop observation, input or social
actions. Explicit general computer tasks retain their separate bounded native grants.

`mercadolivre.rs` owns `Engine::ingest_mercadolivre_batch` (idempotency-key + SHA-256
conflict detection and non-sensitive activity notice emission), `mercadolivre_site_catalog`,
`mercadolivre_reference_catalog` (precompiled ETag-cached `CommerceReferenceCatalog` with
30d price trends and Telegram stats), `mercadolivre_telegram_queue`, and
`ack_mercadolivre_telegram_delivery` (invalidating warm cache on delivery ack).
