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

The Go route accepts only `public_coding`. This is an explicit owner assertion, not a
content detector. The separately authorized Console exception below accepts `personal`.
Do not submit secrets merely because a request has a label: the API cannot infer privacy
from filenames, source repositories, or model names.

## Bounded request contract

A request carries operation/session/request identity, an explicitly authorized purpose, classified
messages, a configured output limit, and a reservation. Context bytes, response bytes,
request duration, attempts, and concurrent requests are bounded. Secret-safe errors
expose categories, not arbitrary provider response bodies or authenticated headers.
The stable provider session identifies the task conversation across its bounded planning
steps; every attempt has a distinct durable reservation identity. Go coding remains
limited to two steps. Console admits at most the smaller of eight steps and its configured
`max_task_model_calls`; step zero and a higher step are rejected before reserving or sending.

A response carries bounded text, the selected model, token usage, and optional provider
charge data. The router rejects an unexpected model or malformed usage. The integer
micro-dollar field records only provider-reported `usage.cost`, rounded upward. It never
substitutes Go allowance consumption for a monetary charge. Unavailable dollar data
stays absent rather than becoming an invented price or zero. The current budget
admission is token/call based; a separate dollar-denominated spending cap is not claimed.

## Bounded Personal Console exception

The owner additionally authorized ordinary text chat and semantic computer planning through
`opencode-console`, classified `personal`, using the same exclusive
`deepseek-v4.1-flash` allowlist. They have separate disabled-by-default profiles and retain
explicit Global processing authorization. There is no fallback between Console and the
coding-only Go route. Console has one attempt per call, disables thinking and never sends
screenshots, cookies, automatic file context or provider reasoning.

Ordinary [assistant chat](ASSISTANT_AND_SHARED_TASKS.md) remains a single text-only call.
The separate [computer profile](../config/computer-models.example.toml) supports up to eight
model calls/semantic actions, with latest bounded observations and compact receipts. A
smaller configured call/token budget or an earlier deadline stops it sooner. Planning
runs on the home worker; executor grants, native owner confirmation and independent
post-action verification stay outside model authority. A model completion cannot create
permission or replace actual execution evidence.

Both Personal use cases reserve and settle through the existing worker journal and
aggregate UTC-day/month token/call limits. Changing a route or profile does not create a
new allowance, clear uncertain usage, or permit a model switch. The last bounded live
Console check returned HTTP402; this external balance/billing prerequisite is not covered
by fake-provider acceptance tests or silently bypassed.

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
Beyond the explicitly authorized Personal/Console slice, future provider adapters, data
categories, vision and general agent workloads still require new contracts and tests.
