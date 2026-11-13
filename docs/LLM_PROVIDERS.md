# Model providers

Status: planned for Phase 2. No adapter, provider API key, paid call, or model prompt is
active. The diagnostic demonstration does not need a model.

OpenCode Go is the preferred candidate. `deepseek-v4.1-flash` and
`muse-spark-1.3-contributor` are configuration candidates, not immutable guarantees.
The official [Go documentation](https://opencode.ai/docs/console/go/) is the authority;
verify it again immediately before implementation. The older `/docs/go/` URL redirects.

The 2026-11-13 reference inspection found coding-agent traffic restrictions, product
User-Agent and stable `x-opencode-session` expectations for direct clients. DeepSeek
V4.1 Flash requires the Global privacy region; Muse Spark 1.3 requires enabling paid
endpoints that may train on submissions. Do not enable either policy for sensitive data
without explicit authorization. Presence in a model catalog does not grant workload
eligibility, especially for general research, email, or video automation.

Before writing a real adapter, record exact endpoints, auth, API styles, headers, session
semantics, current IDs, vision/tool/structured-output capabilities, token/rate limits,
usage fields, terms, retention, training, and permitted use cases from official sources.
Unverified values remain disabled. OpenTatu's adapter suggests isolating transport,
router, retry policy, and a fake client; its current values are not copied as guarantees.

## Provider-independent contract

A model request carries purpose, model allowlist, bounded messages, output schema,
privacy category, session/operation ID, timeout, and reservation. A response carries
validated content, finish reason, usage, optional price data, and provider metadata.
Capabilities are explicit metadata, not inferred from an identifier. Route by purpose;
keep tools and orchestration independent of adapter wire formats.

Use capped exponential backoff only for classified retryable failures. A timeout may
have consumed tokens; reconcile accounting before retry. Reserve task/request/daily/
monthly limits locally and reconcile actual usage. Apply privacy authorization before
sending code, email, documents, or screenshots; training-eligible models need a specific
data-category grant. Keep targeted evidence and summaries rather than entire histories.

Acceptance: synthetic fake-client tests for malformed output, denied model/data routes,
rate limits, timeouts, usage races, and budget exhaustion. Real-provider smoke tests are
explicit opt-in and require workload eligibility; they never run in normal CI.
