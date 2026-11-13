# Model providers and routing

Phase 2 introduces a provider-independent client, a bounded router, and an OpenCode Go
adapter. The owner's current model policy allows only `deepseek-v4.1-flash`; there is
no model fallback. [Official-source research](OPENCODE_GO_RESEARCH.md) owns the dated
endpoint, authentication, usage, workload, and privacy evidence.

`zaturo-llm` owns transport and model routing. `zaturo-orchestrator` owns the structured
proposal loop. Typed tool execution and policy authorization remain separate. Tests
use a fake model or synthetic loopback HTTP responses. A fake client is not a production
provider, and normal CI never uses the owner's paid account.

## Configuration and activation

`ModelsConfig` is canonical in `crates/llm/src/config.rs`; the safe
[example](../config/models.example.toml) is disabled. The worker optionally loads a
model-config path and receives the API key through its protected local environment.
The hub never receives the provider key. Enabling the example alone is insufficient:
the exact model allowlist, coding purpose, explicit public-coding classification,
owner-authorized Global processing, request limits, and durable budget admission must
all pass before an outbound request. Training models and sensitive data are denied.

The only active privacy class is `public_coding`. This is an explicit owner assertion,
not a content detector. Do not submit secrets merely because a request has that label.
The API cannot infer privacy from filenames, source repositories, or model names.

## Bounded request contract

A request carries operation/session/request identity, coding purpose, classified
messages, a configured output limit, and a reservation. Context bytes, response bytes,
request duration, attempts, and concurrent requests are bounded. Secret-safe errors
expose categories, not arbitrary provider response bodies or authenticated headers.
The stable provider session identifies the task conversation across its two planning
steps; every attempt has a distinct durable reservation identity.

A response carries bounded text, the selected model, token usage, and optional provider
charge data. The router rejects an unexpected model or malformed usage. The integer
micro-dollar field records only provider-reported `usage.cost`, rounded upward. It never
substitutes Go allowance consumption for a monetary charge. Unavailable dollar data
stays absent rather than becoming an invented price or zero. The current budget
admission is token/call based; a separate dollar-denominated spending cap is not claimed.

## Budgeting, retry, and interruption

The worker's SQLite journal reserves tokens and model-call capacity before dispatch.
Request, task, UTC-day, and UTC-month limits are checked transactionally. Settle validated
usage against the reservation. Pending or uncertain attempts remain charged across
restart. A provider overrun is retained as debt and blocks further affected calls.

Retries are limited to explicitly classified retryable responses and capped backoff;
timeouts, lost responses, cancellation after send, or an interrupted paid request never
prove that nothing was charged. A repeated reservation cannot silently send the same
request again. Uncertain execution stops for reconciliation. Stored complete assisted
results can be replayed to the hub without another model call.

Actual verification evidence belongs to [the Phase 2 report](PHASE2_REPORT.md).
Future provider adapters, data categories, native tool calls, vision, and general agent
workloads require new contracts and tests; they are not enabled by this implementation.
