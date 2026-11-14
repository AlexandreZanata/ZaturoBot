# 03 — System architecture

Status: proposed design, not an implemented registration. Depends on the [audited baseline](01-REPOSITORY-AUDIT.md) and [existing architecture](../../ARCHITECTURE.md).

## Control, execution and modularity

Zaturo Hub remains the sole source of truth for task IDs, dispatch, leases, campaign authorization, scheduled occurrence admission and owner-visible results. Its SQLite stores new commerce metadata through `zaturo-db`. The outbound notebook worker retains execution intent, bounded provider reservations and cached results in its own journal. It performs network/media work off the constrained VPS. MeuAssistente owns conversations, consent/ownership checks and message delivery evidence in its local database. Neither service opens the other's database.

Start as a feature slice, enabled by explicit configuration and capabilities, composed into the existing worker. Only introduce a `zaturo-commerce` transport adapter crate once one concrete official API and fixture test justify it. Its dependencies may be core/security and reviewed external HTTP libraries; it must not own SQL, model routing or hub logic. A separate service/process is justified later by independent failure/resource/credential isolation, not an arbitrary agent-per-function scheme. Heavy media already has a process boundary in YoutubeBot.

## Owning modules and candidate changes

- `zaturo-core::commerce` would own AgencyProfile, BrandKit metadata, immutable verified facts, campaign/post/report contracts, scope/budget rules and pure validation. Keep focused submodules; no I/O or SQL.
- `zaturo-db` would own additive commerce tables, atomic task/schedule/outbox admission, grant revisions, evidence metadata, scoped counters and journal extensions. Existing deployed migrations stay immutable.
- `zaturo-services::commerce` would orchestrate campaigns using Engine, validate proposals, admit tasks, require evidence before completion and append ordinary Hub summaries. No provider HTTP or SQL.
- `zaturo-protocol` would own negotiated capability-gated task/result extensions and integration envelopes. Unknown versions/fields/capabilities fail closed. An additive service variant may retain protocol v2 only after compatibility tests prove old workers never receive it; otherwise negotiate a new version.
- `zaturo-security` would evaluate authenticated scoped grants independently of models, external content and shared MCP instructions. Existing diagnostic permissions do not grant commerce.
- `zaturo-orchestrator` would generate strictly bounded campaign/content proposals against curated facts through the existing model port. No publication tool or free-form shell.
- `zaturo-llm` would add an explicitly authorized commerce purpose and eligible route. Current PublicCoding/Go and Personal/Console policy stays intact; new commerce purpose is denied until approved.
- Hub/worker apps would compose endpoint authentication, capability configuration, adapters and independent circuit breakers. Native apps would render compact ordinary reports using their existing feed.
- MeuAssistente would expose one reviewed, scoped integration boundary around existing use cases. Its experimental WhatsApp adapter stays fixture-only; communication production is a later gate.

Exact dependency edges belong in `architecture.toml` when implementation adds them. Update crate instructions, component catalog, registry, contracts and roadmap in that change. This plan adds no workspace member or empty crate.

## Responsibility map

Commerce Orchestrator belongs to services/Engine. Campaign Planner uses a model only for bounded strategy proposals. Product Discovery is an authorized provider adapter or owner import. Price Intelligence is deterministic timestamp/currency/variant/source normalization with expiry; it cannot infer price history from one observation. Affiliate Link Manager validates/stores official links and approved attribution, without rewriting prohibited link forms. Offer Ranking uses an explicit deterministic scoring profile; model summaries do not override exclusions.

Content Generator proposes copy grounded in facts and BrandKit. Media Production Adapter delegates to YoutubeBot or licensed deterministic templates. Editorial Scheduler is a bounded commerce slice of the future scheduling design, using the existing task queue rather than a second scheduler daemon. Channel Publisher is one typed adapter per permitted official channel. WhatsApp Communication Adapter is a future MeuAssistente contract, unavailable for commercial production under current restrictions.

Audience and Consent Registry stays authoritative at the communication endpoint for recipient/channel consent; the Hub stores grant references and aggregate summaries. Performance Analytics imports authorized reports and maintains revision-aware attribution. Experimentation Engine changes only preapproved parameters within a bounded grant and requires enough verified observations. Notification and Exception Center reuses Hub activity. Model Router reuses `zaturo-llm`. Policy and Approval Engine reuses security/core rules and durable grants. Most components are ordinary modules, not independent AI agents or deployables.

## Data ownership and transaction boundaries

Proposed Hub records: agency/brand revisions, campaign revisions, fact snapshots and rights references, affiliate link references, grant/revocation versions, schedule/occurrences, content manifests/hashes, external operation intents/receipts, report import cursors and aggregate results. These are business records, not alternative task lifecycle states. Every executable unit references one existing TaskId and OperationId. An external-effect ledger tracks that operation's receipt/uncertainty, not a second queue.

Worker-local records: reserved cost, intent/dispatch boundary, bounded output hashes, remote request/post IDs and reconciliation status. Do not delete this journal on reconnect. Proposed MeuAssistente records: authenticated request deduplication and existing conversation claims/outbox/consent. Provider keys, cookies, customer addresses and raw messages do not move to the Hub.

Admission commits task + operation intent + occurrence key + budget reference atomically where they share the Hub database. Dispatch is at least once; adapters enforce semantic idempotency where possible. There is no cross-database transaction and no exactly-once promise. Recovery uses authenticated status/reconciliation and retained intent. A worker persists its send boundary before network I/O and returns a receipt or Uncertain; it cannot claim a transaction with the remote platform.

## Proposed scale and bounds

First pilot: one niche, one brand, one active campaign, one publisher in flight, at most five candidates per research batch, three drafts per task, 30 future editorial occurrences and 100 audience members if that channel is authorized. Proposed integration HTTP envelope <=32 KiB; existing worker wire messages remain <=16 KiB and carry references when needed. Rendered text/report artifacts <=8 KiB; the ordinary feed summary stays <=4096 bytes. Media uses artifact references, never inline bytes. These are proposed caps to validate, not measured guarantees.

Use bounded concurrency and fair polling; isolate per-channel breakers and media resource units. No new Redis, PostgreSQL, Docker, broker or vector database. The hub target remains below 150 MB normal/300 MB peak from requirements; measure the new workload. Local inference/media runs only on a qualified worker, never the 1 GB hub. Existing memory evidence does not certify commerce.

## Simultaneous work

This branch excludes uncommitted `computer` modules on main. Commerce does not depend on full desktop automation, browser sessions or screenshots. At implementation start re-audit HEAD, capabilities, schemas, feed types and provider contracts, merge through normal owner-approved workflow, and allocate migrations after inspecting actual latest versions. Module opt-out must preserve diagnostic/transfer/YouTube/native acceptance.
