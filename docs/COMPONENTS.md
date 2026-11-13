# Component catalog

Search this catalog and `rg` before introducing a component. Package names below are
stable discovery keys; [architecture.toml](../architecture.toml) is the dependency source.

- `zaturo-core` — `crates/core`: task IDs, task states, legal transitions, execution
  budgets, diagnostic contracts, capabilities, and deterministic validation.
- `zaturo-protocol` — `crates/protocol`: versioned hub/worker messages and serialization.
- `zaturo-db` — `crates/db`: SQLite migrations, task/event persistence, assignments,
  worker records, and a separate durable local execution journal.
- `zaturo-services` — `crates/services`: task creation, claims, lease maintenance,
  progress/completion, owner control, recovery, and pagination.
- `zaturo-security` — `crates/security`: redacted service credentials, authentication,
  shared secure-origin/private-bind policy, and capability policy independent of a model.
  App configuration structs validate operational settings at their composition roots.
- `zaturo-tools` — `crates/tools`: the registered deterministic diagnostic executor.
- `zaturo-hub` — `apps/zaturo-hub`: HTTP/WebSocket composition, pre-HTTP bounded TCP listener,
  notification-driven dispatch with durable fallback, and owner CLI.
- `zaturo-worker` — `apps/zaturo-worker`: outbound connection, journal reconciliation,
  authorized execution, result reporting, and reconnect.
- `xtask` — `xtask`: the shared CI/local quality entry point and contract checks.

## Registrations and extension points

`diagnostic` is the only implemented task kind; `diagnostic.v1` is its tool/capability ID.
Future `github`, `video`,
`price-monitoring`, and `gmail` integrations are planned registrations in
[registry](../config/registry.toml); none contacts an external account today.
No executable AI prompts or provider adapters are installed. Read the
[prompt contract](../prompts/README.md) before adding one.

[Direct dependency rationale](DEPENDENCIES.md) explains external packages.
[Engineering report](ENGINEERING_REPORT.md) records validation and measured local results.
Reusable interfaces are in the owning crate, and crate `AGENTS.md` files identify
local files and tests. [Recipes](recipes/README.md) explain change paths. Add an item
here whenever a new reusable component or workspace crate is introduced.
