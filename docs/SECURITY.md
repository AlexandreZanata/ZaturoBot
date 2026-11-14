# Security contract

The owner policy is authoritative; model proposals and external content are not.
[Threat model](THREAT_MODEL.md) explains boundaries and residual risks.

## Implemented foundation controls

Owner API access and worker transport use separate bearer credentials. Worker identity
is bound to configured registration rather than trusted from a message. Application configuration
rejects unsafe values, and credential debug output is redacted. The typed diagnostic is the only executable tool. A bounded model planner can propose
it through the assisted task route; no shell, browser, or credential broker exists.
Tasks, claims, messages, history pages, connections, and retries have explicit limits.
The production TCP listener admits only 32 sockets, including unauthenticated/partial
HTTP requests and upgraded sockets, with a 30-second I/O idle timeout. Separate limits
bound HTTP requests and authenticated WebSockets. Leases and operation identities
reject stale or mismatched reports.

Hub configuration rejects public/unspecified binds and permits loopback, RFC1918,
CGNAT/Tailscale-range IPv4, or IPv6 unique-local addresses. An address range alone does
not authenticate network membership: firewall/VPN ACLs remain required. Both apps use
one shared endpoint validator requiring an HTTPS origin or literal-loopback HTTP origin,
without user information, path, query, or fragment.

Bind loopback for local development. Remote operation must use a secure private path;
the installed private topology uses an authenticated SSH tunnel to a loopback hub.
A VPN such as Tailscale remains a later alternative. Direct remote plaintext
HTTP/WebSocket is forbidden. A reverse proxy may terminate TLS while the hub stays loopback-bound.
No external account keys belong on the VPS. Hub/worker authentication tokens are service
credentials and must be protected with owner-only file access and rotation.

## Policy classes

Read-only, reversible local writes, Git branch/commit, external communication, account
modification, destructive, financial, and privileged OS actions are distinct policy
classes. The foundation grants read-only diagnostic only and denies unimplemented tools.
Future grants must bind task, capability, resource, scope, expiration, and approval evidence.
Scheduling and model output cannot broaden them.

Destructive actions, privileged modifications, out-of-scope credentials, and high-impact
external effects require explicit owner approval. An approval must identify the concrete
operation and scope; expired or mismatched approvals fail closed.

## Operational limits

Redaction reduces accidental exposure; it does not encrypt process memory or environment
variables. The foundation is not a sandbox for hostile local code. Run separate Unix
identities and protect databases/backups. Persistent encrypted vault storage, device credential rotation APIs, and browser
isolation remain planned. Phase 2 provider routing admits only explicit public-coding
requests, the owner-selected model, and approved Global processing; it is not a general
sensitive-data classifier.
Never dump environment variables or authenticated request headers while debugging.

## Model and publication boundaries

The provider key stays in the worker's protected local environment and never travels to
the hub, model prompts, task results, Git, or application logs. The router and policy
checks are deterministic; model output cannot bypass them. Paid requests reserve budget
before dispatch, and uncertain outcomes remain charged across restart. A model summary
alone is not diagnostic acceptance evidence.

Ignored `.secrets/` contains owner-controlled operational material outside the source
artifact. Repository documentation walks skip it. The publication gate checks Git-visible
working files, staged content, and reachable history with redacted failures. Passing
that heuristic is not a substitute for reviewing the actual staged diff before push.

## Device file boundary

The authorized relay uses independent device tokens, SHA-256 registry entries and a fixed
identity at WS hello. Native age identities and plaintext remain on endpoints; the hub
sees filenames, encrypted sizes/hashes and delivery metadata. Transport SSH uses new
per-device keys restricted to local forwarding to the loopback hub. Owner/admin/provider
credentials are excluded from work-PC enrollment. Incoming content is inert private data,
never a task, shell input or LLM context. Quotas, fixed chunks, strict UUID object paths,
no-follow file opens, local process locks and no-overwrite publication bound the file path.
See [file contracts and recovery](FILE_TRANSFERS.md); this is not the future general vault.


Native app credentials are independently revocable; they cannot control owner-admin APIs.
Google/YouTube credentials stay inside the worker-local profile, distinct from hub tokens.
Sensitive screens bypass normal feed/artifact persistence: standard age ciphertext exists
only in bounded hub RAM for one retrieval/short TTL; fixed metadata acceptance blocks raw
pixels/codes/captions entering task SQL. Literal-loopback app/device clients bypass HTTP
proxy routing and private responses are no-store. See [private screens](ANDROID_SENSITIVE_SCREENS.md).
