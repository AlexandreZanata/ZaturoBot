# Hub composition and transport

Own validated hub configuration, authenticated HTTP/CLI, WebSocket device transport,
and composition. All task decisions go through `zaturo-services`; no SQL or tool
execution belongs here. Bound requests, connections, pages, and messages. Never log
headers, task text, or secrets. Tests use local listeners and synthetic credentials.
See [protocol](../../docs/COMMUNICATION_PROTOCOL.md) and
[deployment](../../docs/DEPLOYMENT.md).

`devices.rs` loads only credential digests and public age recipients. Bind WS/HTTP to
the authenticated device, independent of claimed IDs. `files.rs` composes bounded
spool operations with services; keep the owned critical-section guard alive inside
blocking work even if its HTTP future is cancelled. Device credentials cannot use
owner APIs or submit paid tasks. See [files](../../docs/FILE_TRANSFERS.md).

`clients.rs` binds independent application digests/scopes and optional viewer endpoints.
`activity.rs` composes persisted ordinary chat/feed. `private_views.rs` is separate bounded
RAM-only recipient-encrypted delivery with one worker grant, expiry, consume-once and
no-store headers; no private pixels belong in SQL, ordinary files or task text.

`shared.rs` exposes inert task summaries/details and revision-fenced updates; device
credentials are confined to their own target. Derive author identity from authentication.
Native mobile private viewers require a separate app-registry allow_private_views grant;
only their public age key enters app_viewers SQL. Owner-confirmed captures remain RAM-only.


`computer.rs` authenticates device parent/child routes with a private lease header. Planners cannot create unrelated parents or approve their own actions. Keep lease tokens out of URLs/logs; native chat confirms exact pending steps.

Prepared USB capture is a native-confirmed fixed15-second one-shot under the original viewer and <=60-second expiry. Only opaque preparation metadata survives focus changes; no app background frame fetch. Never use an assistant/MCP request as native consent.

`mercadolivre.rs` exposes authenticated ingest routes (`POST /v1/device/commerce/mercadolivre/ingest`
for enrolled devices and `POST /v1/commerce/mercadolivre/ingest` for owner/apps) with an
explicit 4 MiB route body limit, plus `GET /v1/device/commerce/mercadolivre/reference-catalog`
(compact ETag-cached catalog/price-trend reference for enrolled local workers) and warm-cached
Cloobe-inspired endpoints under the dedicated `4096`-permit commerce concurrency pool: `GET /v1/commerce/mercadolivre/site-catalog` (`ETag`),
`GET /v1/commerce/mercadolivre/taxonomy`, `GET /v1/commerce/mercadolivre/catalog-search`
(faceted JSON and HTML fragment pagination), `GET /v1/commerce/mercadolivre/products/{id_or_slug}`
(PDP with 30d price intelligence, SVG sparkline, FAQ, and similar offers),
`GET /v1/commerce/mercadolivre/redirect/{id}`, `POST /v1/commerce/mercadolivre/telemetry`,
`GET`/`POST /v1/commerce/mercadolivre/price-alerts`,
`GET /v1/commerce/mercadolivre/telegram-queue`, `POST /v1/commerce/mercadolivre/telegram-queue/{id}/ack`,
and `commerce` CLI export commands.


The owner authorized0.8 remote private URL and SMS images plus source publication.
Follow docs/REMOTE_PRIVATE_CAPTURE.md (repository root): native URL grant is RAM-only,
private browser profile separate from semantic sessions; SMS verifies unlocked foreground
before/after capture and optional fixed companion return needs its own grant. Never
self-approve capture, copy account profiles or publish private evidence. Preserve the
main-PC administration/same-signature APK boundary.
