# Native Linux owner client

Own Slint presentation, bounded owner HTTP requests, private client settings and local
verified PNG previews. All task, account and device authorization stays on the hub/worker.
Never put bearer credentials in URLs, diagnostics, argv or the feed. Only loopback HTTP
through the private tunnel, or an HTTPS origin, is permitted. Free text saves a request;
only explicit supported actions dispatch tasks. Do not invent service/worker availability.
Sensitive screenshots are excluded from ordinary feed and local image previews.

`ui/app-window.slint` is the native layout; `src/api.rs` contains bounded wire adapters;
`src/config.rs` owns private owner configuration; `src/main.rs` binds UI callbacks to a
single bounded background request queue. App user-facing copy is Brazilian Portuguese.
Tests use synthetic credentials and temporary private roots. Do not access production
accounts or persist sensitive image content in a test. Source remains English.

`private_view.rs` is a separate ephemeral owner viewer. Explicit user confirmation,
registered device and finite visual watch are required. Reuse standard age in-memory
helpers from transfer; do not write plaintext, OCR, log, cache or add a normal feed row.
Clear at the earlier server/local deadline and on focus loss/hide/tab-close. Cancel
requests best-effort and retain relay expiry as the disconnected fallback. The manual
`examples/private_view_smoke.rs` uses only a generated identity and synthetic pixels.
Do not promise complete erasure of OS/renderer copies; owned buffers zeroize.
