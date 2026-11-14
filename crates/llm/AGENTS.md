# Model adapters and routing

Own provider-independent model messages, routing, bounded transport, cancellation,
and the durable budget port. SQL belongs to `zaturo-db`; reasoning and tools do not
belong here. All provider calls must go through `Router`, which applies explicit
classification policy, model allowlists, deadlines, concurrency, and reservation checks.

Only `deepseek-v4.1-flash` is allowed by the current owner instruction. Verify official
Go endpoints and privacy terms before changing the adapter; source notes belong in
[the provider contract](../../docs/LLM_PROVIDERS.md). Never log request text, response
text, authorization headers, or provider error bodies. Unknown usage or interrupted
requests retain their durable reservation. No real provider call belongs in a test.

Run `cargo test -p zaturo-llm` with synthetic clients and local HTTP fixtures. Actual
production credentials are composition-root inputs, never configuration defaults.

The owner's latest explicit Personal route uses opencode-go with allow_personal_go=true,
allowed_privacy=[personal], Global consent and max_attempts=1. The opt-in defaults false;
legacy coding profiles remain PublicCoding and two-step. Never label personal chat as
public coding, impersonate OpenTatu or fall back to Console/another model. Only DeepSeek
is eligible. Both personal use cases share the existing durable aggregate token/call ledger.

The semantic computer planner admits at most min(max_task_model_calls,8) steps; ordinary
chat remains one call. Preserve the smaller configured budget and all deadline/cancellation
checks. No screenshots, cookies, automatic file context or reasoning enter inference.

Go Personal accepts only typed billing_source go/go-plus with a reported zero balance
charge. Optional allowance consumption is separate from actual charge; missing data is not
invented. The Router settles known parsed usage durably before rejecting an unproved/
charged reply. Malformed or missing usage/transport evidence keeps the reservation
uncertain. Neither path permits automatic retry or Console fallback.
Provider Extra Usage must remain off to prevent server-side credit fallback; a response
check cannot undo an already incurred charge. No paid provider request belongs in tests.
