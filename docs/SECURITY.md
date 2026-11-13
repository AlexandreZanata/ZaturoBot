# Security contract

The owner policy is authoritative; model proposals and external content are not.
[Threat model](THREAT_MODEL.md) explains boundaries and residual risks.

## Implemented foundation controls

Owner API access and worker transport use separate bearer credentials. Worker identity
is bound to configured registration rather than trusted from a message. Application configuration
rejects unsafe values, and credential debug output is redacted. Only the typed diagnostic
capability is dispatchable; no shell, provider, browser, or credential broker exists.
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
prefer an owner-managed VPN such as Tailscale. Direct remote plaintext HTTP/WebSocket
is forbidden. A reverse proxy may terminate TLS while the hub stays loopback-bound.
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
identities and protect databases/backups. Persistent encrypted vault storage, device
credential rotation APIs, browser isolation, and provider-data controls remain planned.
Never dump environment variables or authenticated request headers while debugging.
