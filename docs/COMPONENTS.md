# Component catalog

Search this catalog and `rg` before introducing a component. Package names below are
stable discovery keys; [architecture.toml](../architecture.toml) is the dependency source.

- `zaturo-core` — `crates/core`: explicit chat intents, inert shared-task contracts,
  task IDs, task states, legal transitions, execution
  budgets, diagnostic contracts, capabilities, and deterministic validation.
- `zaturo-protocol` — `crates/protocol`: versioned hub/worker messages and serialization.
- `zaturo-db` — `crates/db`: SQLite migrations, task/event persistence, assignments,
  worker records, shared coordination/public viewer recipients, and separate durable
  local execution and personal/coding budget journals.
- `zaturo-services` — `crates/services`: task creation, claims, lease maintenance,
  progress/completion, owner control, recovery, and pagination.
- `zaturo-security` — `crates/security`: redacted service credentials, authentication,
  shared secure-origin/private-bind policy, and capability policy independent of a model.
  App configuration structs validate operational settings at their composition roots.
- `zaturo-tools` — `crates/tools`: the registered deterministic diagnostic executor.
- `zaturo-llm` — `crates/llm`: provider-independent requests, strict model/privacy
  routing, bounded OpenCode Go transport, explicit Personal opt-in and typed subscription
  billing proof, cancellation, and synthetic fake client. Go diagnostics and Personal
  chat/planning share durable aggregate budgets while retaining distinct classifications.
- `zaturo-orchestrator` — `crates/orchestrator`: strict structured proposals, bounded
  diagnostic planning, policy/executor ports, evidence-based completion, and no shell.
- `zaturo-transfer` — `crates/transfer`: streaming age encryption, bounded opaque spool,
  durable resumable client caches, per-transfer inbox publication and checksum verification.
- `zaturo-android` — `crates/android`: fixed serial-bound ADB discovery/status/capture,
  revocable owner leases, process-group limits and opaque zeroizing screen frames.
- `zaturo-computer` — `crates/computer`: fixed reviewed semantic MCP/service helpers,
  bounded local subprocesses, local grants, exact plan digests, observed price formatting
  and separate zeroizing desktop PNG transport. `deploy/computer` owns reviewed upstream
  stdio/API transport, external read-only BitLupa compatibility and private setup.
- `zaturo-opencode` — `crates/opencode`: hash-pinned installed-program execution, a
  one-attempt budgeted Personal Go gateway, exact job MCP broker and private process,
  cancellation and media receipts. See [execution contract](OPENCODE_EXECUTION.md).
- `zaturo-youtube` — `crates/youtube`: fixed JSON CLI adapter to the existing Python
  YoutubeBot, explicit scheduling grants, result curation and uncertain-job reconciliation.
- `zaturo-desktop` — `apps/zaturo-desktop`: Linux Rust/Slint owner hub, daily activity,
  bounded chronological chat with explicit viewport intent, native Material controls,
  service actions and an ephemeral memory-only private-screen panel. Android's Kotlin
  client shares the presentation/navigation contract in `docs/NATIVE_CLIENTS.md`.
- `zaturo-hub` — `apps/zaturo-hub`: HTTP/WebSocket composition, pre-HTTP bounded TCP listener,
  notification-driven dispatch with durable fallback, and owner CLI.
- `zaturo-worker` — `apps/zaturo-worker`: outbound connection, journal reconciliation,
  authorized execution, result reporting, and reconnect.
- Local MCP — `deploy/mcp`: four compact stdio tools over scoped authenticated hub APIs,
  private profile installation and inert shared coordination; no automatic executor.
- `xtask` — `xtask`: the shared CI/local quality entry point, architecture/documentation
  contracts, and bounded Git-visible secret/history publication checks.

## Registrations and extension points

`diagnostic` is the direct task kind. Phase 2 adds `assisted_diagnostic` with capability
`agent.diagnostic.v1`; both use the one executable tool `diagnostic.v1`.
`github`, `price-monitoring`, and `gmail` remain planned; the existing YoutubeBot/video
adapter is implemented. See registrations in
[registry](../config/registry.toml); live account actions require their own explicit grant.
The `diagnostic-planner` prompt and OpenCode Go adapter form the bounded Phase 2 route.
Read the [prompt contract](../prompts/README.md) and
[model policy](LLM_PROVIDERS.md) before changing them.

[Direct dependency rationale](DEPENDENCIES.md) explains external packages.
[Engineering report](ENGINEERING_REPORT.md) records validation and measured local results.
Reusable interfaces are in the owning crate, and crate `AGENTS.md` files identify
local files and tests. [Recipes](recipes/README.md) explain change paths. Add an item
here whenever a new reusable component or workspace crate is introduced.
