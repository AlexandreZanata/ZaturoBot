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
