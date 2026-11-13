# Add an LLM provider

Read [provider contract](../LLM_PROVIDERS.md). Verify official current endpoints, auth,
headers, model IDs, capabilities, usage, rate limits, terms, retention/training, and
workload eligibility. Record a dated source note; never fill gaps from memory.

Define adapter-only transport behind the provider-independent request/response interface.
Keep selection, policy, budgets, and orchestration outside it. Classify errors, cap
retries/timeouts, and sanitize messages. Register model allowlists and data categories;
blocked privacy settings remain blocked until explicitly authorized.

Use synthetic fake-client/local-server tests for serialization, malformed output,
timeout, rate limiting, usage reconciliation, and secret-free errors. Update components,
provider document, roadmap, and registry. Run `cargo xtask check`. Real paid calls are an
optional explicitly authorized smoke test, never part of CI or this foundation phase.
