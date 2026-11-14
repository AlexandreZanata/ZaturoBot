# Model providers and routing

`zaturo-llm` owns provider transport, bounded requests, privacy/model admission and the
budget port; `zaturo-orchestrator` owns structured planning. Typed execution, local grants
and native owner approval stay independent. Only `deepseek-v4.1-flash` is eligible.
The owner's latest explicit instruction selects Go for personal chat/computer planning
(ADR-0021), superseding the earlier Console-only routing. No model or Console fallback.

Tests use fake models or synthetic localhost HTTP responses; no production credential or
paid provider request belongs in a test. A fake response does not prove live subscription,
billing, account login or actual destination-PC acceptance.

## Configuration and data classification

`ModelsConfig` in `crates/llm/src/config.rs` is canonical. Its `allow_personal_go` opt-in
defaults false. The [coding example](../config/models.example.toml) retains
`allowed_privacy=["public_coding"]` and a two-step diagnostic contract. Existing coding
profiles are not silently converted to personal traffic.

The separate [chat](../config/assistant-models.example.toml) and
[computer](../config/computer-models.example.toml) examples select `opencode-go`, explicitly
set `allow_personal_go=true`, and require exactly `allowed_privacy=["personal"]` with
`max_attempts=1`. They remain disabled and require explicit Global processing consent.
The opt-in records the owner's route decision; it does not label personal data as public
coding, change provider terms, or authorize screenshots/credentials. A missing/mismatched
classification, unapproved model, training consent, deadline or budget fails closed.

The home worker uses its existing protected `OPENCODE_GO_API_KEY`; the hub and work PC
receive no provider key. A worker profile alone does not configure provider Extra Usage
or establish actual plan eligibility. Legacy Console transport remains explicit
compatibility support, not an automatically selected backup or a requirement to buy credit.

## Fixed Go connection and reference scope

The endpoint is `https://opencode.ai/inference/go/openai/v1/chat/completions`, with Bearer
authentication, ZaturoBot's own product user agent and a stable `x-opencode-session` for
its conversation. The [official Go documentation](https://opencode.ai/docs/console/go/)
describes direct clients and coding-agent-style traffic. It does not expressly promise
arbitrary-personal workload eligibility. Record the owner's accepted local use honestly;
do not impersonate OpenCode or claim a new provider permission.

The read-only reference is [OpenTatu commit05e8bb3](https://github.com/AlexandreZanata/OpenTatu/tree/05e8bb33b9256eff35d5b6bc05ca8dee067a35c3),
particularly its provider implementation, configuration pointers and connection record.
Reuse only the verified endpoint/header/key-environment pattern. Do not copy its Muse
Spark roles, training permission, 900-second timeout, broad output handling or accounting
that treats allowance use as a monetary charge. ZaturoBot retains its bounded transport,
secret-safe errors and exclusive DeepSeek allowlist. The earlier
[Go research](OPENCODE_GO_RESEARCH.md) remains dated Phase2 evidence, read with ADR-0021.

## Request bounds and durable budgets

Every call carries task/operation/request identities, explicit classification, bounded
messages/output and a reservation. Personal chat is one tool-free call with at most four
clipped ordinary context turns. The semantic planner uses latest bounded observations and
compact receipts, at most the smaller of eight steps and configured
`max_task_model_calls`. Step zero or a higher step is rejected before reserve/send.
PublicCoding Go diagnostics remain limited to two steps. No reasoning, screen pixels,
cookies or automatic file context enters personal inference; thinking is disabled.

The worker journal reserves token/call capacity before dispatch and checks request/task/
UTC-day/UTC-month totals transactionally. Personal and coding journals share aggregate
admission; another profile/route never creates a fresh allowance or clears debt. Unknown
or interrupted requests retain their reservations across restart. The existing token/call
caps remain local limits, not a live reading of the subscription's remaining allowance.

Retries require an explicitly eligible error/backoff on their original bounded route.
Personal requests have one attempt. Malformed usage, cancellation or lost responses keep
the unknown reservation charged for reconciliation. When a response has known parsed
usage, the Router settles that usage durably before applying the subscription billing
acceptance rule. A known billing-source/charge mismatch blocks reply content and further
planning after accounting; it is not relabelled an unknown charge. Neither outcome
permits automatic retry or fallback. A matching cached complete result may be reported
without another model call.

## Subscription evidence and additional-credit boundary

Go Personal acceptance requires typed `billing_source` equal to `go` or `go-plus`, and
reported `usage.cost` zero. `ModelUsage.cost_microusd` continues to represent actual
balance charge. `ModelResponse.billing` contains optional `ModelBilling` evidence: source
and `allowance_microusd`. The latter is plan consumption, not another charge, refund,
remaining balance or local spending allowance. Missing billing-source/cost is not inferred as
subscription/free use. Unknown, balance, BYOK/free or positive-charge evidence cannot
satisfy the Personal Go acceptance rule. The
[official inference cost contract](https://opencode.ai/docs/console/api/inference/#cost)
defines those provider fields separately.

Billing metadata is parsed into normalized response/report data in RAM. Ordinary chat
receipts retain this safe metadata in the existing service-result JSON; no new SQL
allowance ledger is introduced, and historical results need not contain it. Existing actual usage,
reservation and uncertainty persistence remains intact. Receipt validation can stop later
calls and refuse a misleading success, but cannot undo a charge after a request was sent.

Keep provider **Extra Usage disabled** for the owner's no-additional-credit requirement.
The [provider's beyond-limit policy](https://opencode.ai/docs/console/go/#usage-beyond-limits)
allows balance fallback when that option is on. Endpoint selection and local response
checks are insufficient to prevent that server-side fallback if settings later change.
The supplied account screenshot showed Extra Usage off/$0 at that time; it is neither a
live account monitor nor proof of a future request. No top-up, credit purchase, balance
fallback, quota circumvention or alternative model is authorized.

## Historical evidence and live Go acceptance (2026-11-14)

The prior explicitly authorized Console attempt returned HTTP402. Keep that result as
historical evidence in [the assistant record](ASSISTANT_AND_SHARED_TASKS.md); Console
balance is no longer a prerequisite for the newly selected Personal Go route. The
[Phase2 report](PHASE2_REPORT.md) retains its separate earlier coding-route evidence.

One owner-authorized normal chat request completed through the deployed hub and updated
home worker using the fixed Go client. The provider returned `deepseek-v4.1-flash`,
`billing_source=go`, `usage.cost=0`, 337 input tokens and 6 output tokens in one model call.
The worker settled the durable reservation, retained subscription allowance separately,
and published the real reply in the ordinary feed consumed by both native clients.
The live result was "IA Go conectada." No Console fallback or credit purchase occurred.

The home-worker0.6.1 update preserved the existing key, coding profile and earlier ledger
rows under a consistent private backup. Both personal model profiles select Go; worker
schema8 and compatible hub schema9 remain unchanged. Provider routing is on the worker,
so this fix requires no Android or desktop UI update. This does not establish physical
phone/work-PC GUI acceptance or a real social publication. Beyond this explicitly
authorized slice, new models, data categories, vision and general agent workloads require
their own contracts and tests.
