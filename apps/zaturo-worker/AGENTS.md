# Worker composition and transport

Own outbound authenticated WebSocket connections, strict local runtime configuration,
permission checks, and durable execution-journal coordination. Tools live in
`zaturo-tools`; SQL lives in `zaturo-db`. Plain/model-assisted diagnostics and explicitly authorized fixed service requests are
the supported task kinds. `execution.rs` adapts the durable ledger and journal to the model router.
Never select a fake client from a production configuration or expose provider keys to the hub.
Wait for hub Ready before dispatch. Record intent before execution and result before
reporting. An unknown side effect must never be blindly retried. Read
[recovery](../../docs/TASK_LIFECYCLE.md) before editing the connection loop.
Keep the socket responsive while execution runs. Owned jobs must cancel on revocation
or disconnect; persisted reservations fence replay even after abrupt process death.

`files.rs` runs a separate private inbox loop; `zaturo-transfer` owns crypto and file
transport. Received bytes are inert and never enter task/model execution. Use a unique
local native age identity and private per-transfer inbox paths. Work-PC enrollment has
neither provider nor owner/admin credentials. See [files](../../docs/FILE_TRANSFERS.md).

Owner-authorized services now use `service_execution.rs` with separate durable fences.
`service_jobs.rs` observes actual heavy-job state independently of submission. Youtube
uses its existing Python project and separate fixed resource units. `private_views.rs`
requires an exact hub grant and registered owner USB lease, encrypts raw PNG in memory
and uploads only ephemeral ciphertext. It never stores screenshots in its journal or
file cache. Keep the public-coding diagnostic policy and its two-step limit intact.

`assistant.rs` composes the explicitly opted-in Go text-only orchestrator and Personal
budget ledger. Personal classification, one attempt and subscription billing proof remain
separate from public-coding diagnostics; never fall back to Console or copy the key to work.
Cache a complete reply; uncertain requests never repeat. `android_actions.rs`
opens only the messaging app of an explicitly registered/granted USB device and atomically
saves metadata-only recipes under workspace_root/android-instructions. These local recipes
are observations, not new permissions. Keep media generation separate from publication.


`computer.rs` keeps paid home planning separate from work execution; the work child fetches a fresh parent-bound grant before the local adapter. `desktop_private.rs` captures only the locally fixed window through the private RAM relay. No provider key reaches the work PC.

`opencode.rs` composes the real installed runner with the same durable Personal budget and service journal. `service_jobs.rs` requires full ready-media proof before installed-executor completion. Never replay a paid intent or detached actor. Separate portal capture requires explicit local/native grants; no pixels enter journals.

`mercadolivre.rs` composes `commerce ingest-ml`, `commerce watch-ml`, `commerce sync-reference`,
and `commerce query-reference`: reads bot JSON batches locally (skipping unchanged SHA-256
digests in watch mode), filters 9:16 vertical reel thumbnails and strips `.local/` filesystem paths,
uploads verified static images to MinIO, posts validated Hub ingest batches with `Idempotency-Key`,
and maintains an ETag-cached local VPS reference database (`If-None-Match` -> `304`) for sub-ms queries.

`telegram.rs` composes official Telegram Bot API publishing (`commerce telegram-publish` and
optional `ingest-ml --publish-telegram/--dry-run-telegram`), interactive `getUpdates`
command replies (`commerce telegram-poll`), and human-assisted WhatsApp Web draft exports
(`commerce whatsapp-drafts`) without Meta API or unofficial browser automation.


The owner authorized0.8 remote private URL and SMS images plus source publication.
Follow docs/REMOTE_PRIVATE_CAPTURE.md (repository root): native URL grant is RAM-only,
private browser profile separate from semantic sessions; SMS verifies unlocked foreground
before/after capture and optional fixed companion return needs its own grant. Never
self-approve capture, copy account profiles or publish private evidence. Preserve the
main-PC administration/same-signature APK boundary.
