# Model adapters and routing

Own provider-independent model messages, routing, bounded transport, cancellation,
and the durable budget port. SQL belongs to `zaturo-db`; reasoning and tools do not
belong here. All provider calls must go through `Router`, which applies explicit
public-coding policy, model allowlists, deadlines, concurrency, and reservation checks.

Only `deepseek-v4.1-flash` is allowed by the current owner instruction. Verify official
Go endpoints and privacy terms before changing the adapter; source notes belong in
[the provider contract](../../docs/LLM_PROVIDERS.md). Never log request text, response
text, authorization headers, or provider error bodies. Unknown usage or interrupted
requests retain their durable reservation. No real provider call belongs in a test.

Run `cargo test -p zaturo-llm` with synthetic clients and local HTTP fixtures. Actual
production credentials are composition-root inputs, never configuration defaults.

The owner-authorized personal chat exception uses only `opencode-console` with Personal
classification, a fixed Console endpoint and max_attempts=1. Go remains PublicCoding only;
never fall back between routes. Both use the same exclusive DeepSeek allowlist and durable
aggregate budgets. Console requests disable thinking to bound ordinary-chat overhead.
No screenshots, cookies, automatic file context or provider reasoning in the conversation.

The owner-authorized semantic computer planner uses the separate Personal/Console profile
with at most eight sequential steps and max_attempts=1. Router step admission is further
bounded by that profile's max_task_model_calls; it never expands Go's two-step coding
route. Home-worker planning shares the existing durable aggregate token/call ledger,
exclusive DeepSeek model and cancellation/deadline checks. Typed executor grants and
native owner confirmations remain outside model authority. Preserve both the smaller
configured call limit and the absolute eight-step ceiling; no screenshots enter models.
