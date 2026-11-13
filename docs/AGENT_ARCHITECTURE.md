# Agent architecture

Phase 2 implements one bounded coding-environment diagnostic loop. It does not provide
arbitrary project editing, a shell, browser/desktop control, external communications,
or specialist agents. Those capabilities remain in their later roadmap phases.

`zaturo-orchestrator` receives an owner-authorized public coding request and a provider-
independent model interface. Its trusted [planner prompt](../prompts/diagnostic-planner.md)
is static and versioned. Owner text and tool observations are separate JSON messages,
not substitutions into system instructions. Only declared structured actions can cross
the parser; additional fields, unknown tools, and proposed privilege changes fail closed.

The executable sequence is limited to two planning steps and one real tool:

1. The model proposes `diagnostic.v1` without arbitrary arguments.
2. The policy/executor authorizes the read-only tool and returns its typed observation.
3. The model returns a bounded summary grounded in that observation.
4. Independent result acceptance verifies tool identity, operation identity, diagnostic
   success, model policy, and bounded usage/evidence before the task can complete.

A finish-before-tool proposal, repeated tool request, unregistered action, or extra step
cannot report success. The model can propose, but it cannot dispatch tools, read secrets,
grant approval, or change policy. The worker executor journals intent/result around the
typed diagnostic and the final assisted outcome before hub reporting.

The model router enforces the [provider contract](LLM_PROVIDERS.md); durable worker
reservations enforce spend admission across tasks and restarts. Cancellation stops
further dispatch, while a model request that may already have reached the provider stays
accounted for. Unknown paid-request outcomes require reconciliation rather than blind
retries. A completed journaled outcome is safe to resend after reconnection.

Store concise summaries, selected model, usage, tool evidence, operation/request identity,
and error categories. Do not store private chain-of-thought, raw credentials, or complete
provider error bodies. Fake-model tests cover prompt-injection attempts, malformed
actions, denied routes, budgets, cancellation, and restart/uncertain-request handling.

The natural-language summary remains untrusted model output. Local validation proves the
recorded tool/operation contract and bounds; it does not prove every sentence in a model
summary is semantically true. The typed diagnostic is the authoritative task evidence.
