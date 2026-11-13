# Agent architecture

Status: planned for Phase 2. The current worker executes a deterministic task directly;
it does not call a model, reason autonomously, or spawn specialist agents.

The future orchestrator consumes an owner task plus bounded context, asks a model for a
structured proposal, validates it, requests policy authorization, dispatches one typed
tool, records its result, and evaluates task acceptance. Models cannot dispatch tools or
access credentials directly. Tool observations are untrusted data with provenance.

A provider-independent request carries purpose, allowed models, response schema,
privacy category, deadline, and budget reservation. The provider adapter translates wire
formats; the orchestrator owns planning and task progress. The policy engine remains
outside both. Store concise decision summaries and evidence references, not private
chain-of-thought. Memory should separate owner preferences, project facts, and transient
task context, with provenance and retention.

Dependencies: [provider contract](LLM_PROVIDERS.md), typed tool registry, durable
operation journal, and policy grants. Threats: prompt injection, forged tool claims,
context disclosure, runaway loops, and budget races. Acceptance requires a fake model
fixture that proposes forbidden tools, bounded step/token/time limits, and recovery
without repeating uncertain external effects. No multi-agent team is needed before one
agent can satisfy those criteria reliably.
