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
