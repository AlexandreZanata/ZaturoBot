# Prompt declarations

No executable model prompts exist in Phases 0/1. This directory documents the declaration
contract; it does not provide a hidden agent loop. The registry's prompt list is empty.

When Phase 2 adds prompts, register each path, stable ID, purpose, version, permitted
variables, output schema, and owning component. Separate trusted system instructions
from fenced untrusted evidence. Render only declared variables; reject missing/unknown
variables. Treat provider/model configuration as data and never embed credentials.

A prompt change needs synthetic rendering/response fixtures and a behavioral evaluation
for policy boundary regressions. Keep expected structured output explicit, bound context,
and store concise evidence summaries rather than private chain-of-thought. Follow
[change-agent-prompt](../docs/recipes/change-agent-prompt.md).
