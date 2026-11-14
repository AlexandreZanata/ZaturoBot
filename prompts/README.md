# Prompt declarations

The Phase 2 `diagnostic-planner` prompt is the static trusted instruction for the bounded
coding diagnostic. Its strict action schema and declaration live beside it. The prompt
has no template variables: owner requests and observations are separate JSON messages.

When adding prompts, register each path, stable ID, purpose, version, permitted
variables, output schema, and owning component. Separate trusted system instructions
from fenced untrusted evidence. Render only declared variables; reject missing/unknown
variables. Treat provider/model configuration as data and never embed credentials.

A prompt change needs synthetic rendering/response fixtures and a behavioral evaluation
for policy boundary regressions. Keep expected structured output explicit, bound context,
and store concise evidence summaries rather than private chain-of-thought. Follow
[change-agent-prompt](../docs/recipes/change-agent-prompt.md).

`assistant-chat` is the separate tool-free Personal/Console conversational prompt owned by
`zaturo-orchestrator::conversation`. Its plain text final answer is capped at1500 UTF-8
bytes; the schema records the string form, while the source enforces the byte limit.
History is separately encoded ordinary data; no reasoning, screen or automatic file context.

`computer-planner` uses at most eight structured Personal/Console calls, one clipped
semantic observation and compact verified receipts. Exact input/social draft approvals
remain hub/local policy; model output cannot approve itself. Screenshots, credentials
and private reasoning are excluded. See [computer use](../docs/COMPUTER_USE.md).
