# 08 — AI provider and cost strategy

> Historical planning baseline at `eb6a0cb`. The [refined plan](../commerce-refined/ZATURO-COMMERCE-REFINED-PLAN.md) supersedes it where different; [current commerce contracts](../../COMMERCE.md) and [F1 evidence](../../COMMERCE_F1_REPORT.md) describe implemented behavior.

Status: proposed design only. Official public sources checked on 2026-11-14. No model request, subscription change, credential access, or budget reset was performed.

## Decision

Reuse `zaturo-llm`, the existing typed orchestrator ports and durable reservation mechanism. Add a separately authorized commerce purpose and policy profile in a future implementation. Do not create ad hoc HTTP clients in campaign, content, media, or communication modules. MeuAssistente retains its own service boundary, model permissions and consumption history.

Existing repository authorization is specific: Go is restricted to `PublicCoding`, and the separately configured Console route supports bounded `Personal` chat with the allowlisted DeepSeek V4.1 Flash model. Neither purpose grants unattended commercial drafting or customer support. A provider's public model list cannot extend the owner's allowlist. Until an approved commerce route exists, deterministic templates and synthetic model outputs support implementation and tests.

## Dated OpenCode evidence

Go expects typical coding-agent traffic, an identifying user agent and stable `x-opencode-session`; custom-client support does not prove marketing eligibility. Plans are $10/$40/$20-per-member monthly for Go/Plus/Teams; service accounts are not covered by Teams. For the owner's allowlisted `deepseek-v4.1-flash`, the documented endpoint is `https://opencode.ai/inference/go/openai/v1/chat/completions`. Go allowance is $100/month with five-hour/weekly windows of 20%/50%. Per-million input/output/cache-read allowance rates are $0.15/$0.60/$0.003 off-peak and $0.30/$1.20/$0.006 peak. These are not new project spending grants. The documented no-training/zero-day-retention DeepSeek arrangement requires Global regions and is dated through November 30, 2026. Reverify before expiry. No Extra Usage, fallback or model expansion is authorized. [OpenCode Go documentation](https://opencode.ai/docs/console/go/).

OpenCode's separate [Console inference API](https://opencode.ai/docs/console/api/inference/) documents `/inference/openai/v1/chat/completions`, Responses, Anthropic and Gemini families, bearer authentication and paid access from custom clients. Free-tier chat access is restricted to OpenCode clients. This proves a transport option, not a commerce grant or acceptable customer-data processing contract. [Hosted-service terms](https://opencode.ai/legal/terms-of-service) are effective November 8, 2026; review the complete terms with the applicable model provider conditions before selecting a commercial route. Do not infer an explicit prohibition or permission for every non-coding purpose merely from technical interoperability.

## Candidate routing policy

- **Current Go / `PublicCoding`:** preserve existing public-code diagnostic use. Commerce admission denied. No disguised coding prompt or generic agent label may bypass purpose checks.
- **Current Console / `Personal`:** preserve the existing bounded owner chat. Commerce admission denied until a separate grant and reviewed profile exist; do not reuse its budget under another name.
- **Proposed paid general-purpose Console profile:** candidate for public product facts and content proposals after purpose, commercial terms, privacy/region, retention, prices and spend caps are reviewed and approved. Use the existing provider adapter where compatible. All model IDs remain subject to the owner allowlist; stronger models are a later explicit choice, not an automatic fallback.
- **Direct first-party DeepSeek API:** optional future provider, not required for MVP. Official [API documentation](https://api-docs.deepseek.com/) exposes a distinct `https://api.deepseek.com` base URL and currently documents `deepseek-flash` served by V4.1-Flash-1017; that is not the Go model ID or account. Separate [prices](https://api-docs.deepseek.com/quick_start/pricing/) and API-specific contract review are necessary. The [consumer/privacy policy](https://cdn.deepseek.com/policies/en-US/deepseek-privacy-policy.html), updated February 10, 2026, explicitly does not cover processing rules for end users in downstream developer applications; it cannot substitute for an API data-processing review. No new transport or credentials are installed in this phase.
- **Local inference:** candidate on a notebook worker only; no model hosting on the constrained VPS. A future loopback adapter can target a pinned [llama.cpp server](https://github.com/ggml-org/llama.cpp), with an independently reviewed model and weights license, checksum, RAM/CPU latency measurements and bounded prompt/output. [Qwen3 upstream](https://github.com/QwenLM/Qwen3) is a model-family research reference, not a selected or downloaded model. Local inference still requires approval as a new capability. License, data protection, CPU/energy and quality costs remain real.
- **No provider:** deterministic, evidence-backed templates remain a supported mode. Provider outage or budget exhaustion pauses model-required work and reports an exception; it does not silently route to an unapproved model.

## Proposed request contract

A commerce inference request carries immutable task/operation/request IDs, purpose (`CommerceDraft`, `CommercePlan` or a later separately approved support purpose), account-free product evidence references, data classification, permitted regions, training/retention ceiling, approved provider/model/profile revision, max input/output tokens, timeout, maximum calls and reserved cost. These are proposed types and are not added to serialized contracts by this plan.

Order of admission is deterministic: validate task and grant revisions; minimize/redact data; check model/purpose/privacy; reserve cost durably; construct bounded request; send once under its request ID; validate response schema and evidence references; settle provider usage; store result before acknowledgement. Cancellation after transmission may leave an uncertain billed outcome. Retain that reservation and reconcile before another call. A fresh provider on timeout does not cure uncertain spend.

LLMs propose a campaign brief, content draft or bounded next step. They never approve their own budget, choose a new destination, invent a price, modify an affiliate tag, or execute a publish/send operation. Only deterministic code commits accepted proposals. Do not reveal private chain-of-thought in owner reports.

## Cost model and containment

Use fixed-point money in one recorded billing currency and checked integer arithmetic. Do not use floating-point totals for financial admission. Keep provider-invoiced usage separate from plan allowance usage and an owner's commercial budget. Reject a missing, expired or ambiguous price schedule.

Proposed formula: rate variables are integer micro-USD per million tokens; token counts are integers. The result is rounded upward to one micro-USD, with checked multiplication/addition and no floating-point conversion:

`reservation = ceil((max_uncached_input * input_rate + max_cached_read * cache_rate + max_cached_write * write_rate + max_output * output_rate) / 1_000_000) + known_fixed_fees_microusd`

Reserve at the highest rate that could apply during the request, assume no cache discount until provable, include billed reasoning in output where the provider does so, and reserve any media/tool charge separately. Admission takes the minimum remaining request, task, campaign, UTC-day, UTC-month, provider and owner cumulative budget. Token limits alone do not cap media costs.

A synthetic planning example, not a quote or authorized spend: 5,000 uncached input tokens and 1,000 output tokens at fictional $0.20/$1.00 per million cost $0.0020 before fees. The corresponding token reservation is 2,000 micro-USD. Two hundred such requests cost $0.40 before subscriptions, media, storage, taxes and currency exchange. Replace fictional values with a dated approved price revision before live admission. The project has authorized **zero new commerce paid calls** in this planning execution.

Maintain separate ledgers for policy admission and reporting only when responsibilities require them; there is one authoritative spend record per payer/provider account. If both projects consume one account, either centralize reservation behind a typed authenticated service or partition preallocated envelopes whose sum is bounded. Do not independently spend the full shared account limit in each repository.

MeuAssistente's existing **USD 10 cumulative validation reserve and historical consumption remain intact**. Pending/uncertain usage consumes the remaining reserve; reusing a model, new namespace, new day, fresh database or new checkout must never reset it. A commerce envelope is additional proposed policy requiring explicit authorization, not a reinterpretation of that USD 10. Existing ZaturoBot personal/coding balances and debt also remain intact.

## Economy without hidden escalation

Use deterministic parsers for ingestion, price validity, ranking arithmetic, disclosures and analytics. One inexpensive approved model may draft a bounded bundle for one campaign from clipped public evidence. Cache by evidence revision, brand revision, prompt/schema version and policy profile, invalidating on material change. Do not copy the entire feed or conversation history into every request.

A stronger model is justified only by recorded failure on a meaningful evaluation set and an explicitly approved incremental budget. Default maximum repair attempts should be zero or one within the same task reservation, never an open-ended debate among agents. A quality failure yields a draft exception rather than fabricated completeness. Generate expensive media only after text/evidence validation and enforce a distinct media job cap.

## Dependencies and acceptance

Dependencies: [audit](01-REPOSITORY-AUDIT.md), [workflow design](05-AGENT-AND-WORKFLOW-DESIGN.md), [autonomy policy](06-AUTONOMY-AND-POLICY-MATRIX.md), [acceptance plan](10-TEST-AND-ACCEPTANCE-PLAN.md).

Before any provider activation, record approved purpose/model/region, account-local secret reference, current terms/privacy and price evidence, per-call/task/day/month/cumulative limits, quality threshold, cancellation/reconciliation method and a rollback to provider-disabled mode. Revoking a provider stops new admission immediately, preserves uncertain reservations and cached evidence, and does not erase invoices/history.

Synthetic acceptance must prove cross-purpose rejection, unchanged existing allowlists, preserved USD 10 history, concurrent double-reservation prevention, fixed-point overflow rejection, cache-rate conservatism, unknown-billing quarantine across restart, no unapproved fallback, invalid JSON/evidence rejection and PII minimization. A local inference benchmark must state actual hardware, model hash/license, measured peak RSS, throughput, timeout and acceptance scores before claiming viability. Provider documentation and fake responses alone do not verify commercial permission, production quality or billing settlement.
