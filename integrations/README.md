# Integration boundary

There are no executable personal-service adapters in the foundation. The machine-readable
registry lists planned `github`, `video`, `price-monitoring`, and `gmail` integrations.
Read [integration contracts](../docs/INTEGRATIONS.md) before implementing any of them.

An adapter owns external API/protocol translation and typed bounded results. It must not
invent task policy, obtain credentials directly from model text, or assume a retry is safe.
Keep provider-specific credentials behind the future broker, operations journaled, and
fixture tests independent of real accounts. Use
[add-integration](../docs/recipes/add-integration.md) when adding the first adapter.
