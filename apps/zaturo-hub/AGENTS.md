# Hub composition and transport

Own validated hub configuration, authenticated HTTP/CLI, WebSocket device transport,
and composition. All task decisions go through `zaturo-services`; no SQL or tool
execution belongs here. Bound requests, connections, pages, and messages. Never log
headers, task text, or secrets. Tests use local listeners and synthetic credentials.
See [protocol](../../docs/COMMUNICATION_PROTOCOL.md) and
[deployment](../../docs/DEPLOYMENT.md).
