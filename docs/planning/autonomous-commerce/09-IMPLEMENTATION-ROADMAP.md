# 09 — Implementation roadmap

> Historical planning baseline at `eb6a0cb`. The [refined plan](../commerce-refined/ZATURO-COMMERCE-REFINED-PLAN.md) supersedes it where different; [current commerce contracts](../../COMMERCE.md) and [F1 evidence](../../COMMERCE_F1_REPORT.md) describe implemented behavior.

Status: proposed execution phases; **no feature is implemented by this plan**. Dependencies: [audit](01-REPOSITORY-AUDIT.md), [architecture](03-SYSTEM-ARCHITECTURE.md), [policy](06-AUTONOMY-AND-POLICY-MATRIX.md), [platform evidence](07-PLATFORM-COMPLIANCE.md), [cost strategy](08-AI-PROVIDER-AND-COST-STRATEGY.md) and [acceptance](10-TEST-AND-ACCEPTANCE-PLAN.md).

## Ordering and shortest path

The present planning milestone creates the isolated branch and these reviewable documents. Implementation starts only after a new implementation request. At that point re-read actual HEAD and concurrent work, agree the slice, and allocate schema/protocol changes from the current inventory. Do not copy uncommitted main changes, reserve migration 0009, or interpret this roadmap as production permission.

The shortest path is C1 → C2. C1 proves an offline agency loop (MVP-D); C2 completes a real owner-assisted commercial pilot (MVP-P), provided a channel/program is eligible. Neither requires paid inference, WhatsApp, automated discovery, a scheduler, video production or multi-agent infrastructure. C3 → C4 adds narrowly authorized unattended publication (MVP-A). C5–C9 are optional independent expansions after their prerequisites. If affiliate enrollment fails, C2 may publish useful original non-affiliate content, but must not report that as affiliate revenue acceptance.

Each future phase updates its owning contracts, component catalog, crate instructions, registry/roadmap and ADR when affected. Run focused checks and then `cargo xtask check`; run `cargo xtask secrets` before any authorized publication of code. Do not activate a planned registry entry merely to make documentation checks pass.

## C1 — Deterministic draft agency (MVP-D)

- **Objective:** produce one complete brand-and-curation packet and one factual Hub report with all external effects disabled.
- **Repositories and reuse:** ZaturoBot only; reuse core validation, Engine/task IDs, db transactions, worker intent/result, existing activity feed, native report rendering and trusted local artifacts. MeuAssistente remains read-only.
- **Changes:** add focused commerce domain/service/persistence modules, disabled configuration/capability, AgencyProfile/BrandKit revisions, evidence-backed product import, deterministic exclusions/ranking, licensed local brand templates, content manifest and draft result acceptance. Avoid new crates unless a concrete adapter boundary requires one. Preserve existing task/protocol compatibility. Use artifact references instead of embedding images or oversized JSON in messages.
- **Dependencies:** an implementation request; reviewed schema/contract design against current HEAD; fixture media with explicit rights; no production account/provider requirement.
- **Acceptance:** five synthetic products yield two valid candidates and three distinct branded draft assets; stale/unverifiable candidates are rejected with reasons; every claim/price/link references evidence; every asset pins brand revision and rights; one stable report records created drafts and blocked actions after reopen. Commerce disabled leaves existing diagnostics/files/chat/YouTube behavior intact. Draft output never says published.
- **Tests:** T01–T04, T06 and T15 in the acceptance plan, plus the existing affected crate tests. Synthetic template rendering must be visually inspected for readable disclosure, contrast and platform safe areas; a renderer exit code alone is insufficient.
- **Risks:** overeager schema growth, source-text prompt injection even without inference, unsafe local file references and identity rights. Reject arbitrary paths and unsupported facts.
- **Costs:** no new paid calls/subscriptions; local CPU/storage measured. This is a cost class, not a claim that hardware/energy is free.
- **Rollback:** disable the commerce feature and stop its future admissions; retain draft/evidence/audit records. Restore a consistent pre-upgrade database only with the compatible binary and reconciliation of newer tasks; never delete shared journals or roll back unrelated work.

## C2 — First real assisted commercial pilot (MVP-P)

- **Objective:** turn an approved content packet into one observed, permitted public operation without requiring an automatic publisher.
- **Repositories and reuse:** ZaturoBot; reuse C1, existing Hub input/reporting and immutable content artifacts. No MeuAssistente mutation or browser-session reuse.
- **Changes:** accept bounded owner-imported authorized evidence and official affiliate links; record applicable program/medium approval, validity, disclosure and link constraints; export a packet; let the owner record the actual external post reference. Add a small verified report import if the affiliate program provides an authorized export. Label manual publication explicitly.
- **Dependencies:** owner chooses niche and BrandKit; actual eligible account and approved distribution medium; current official terms review; licensed media; explicit publication by the owner. If those are absent, remain in C1.
- **Acceptance:** one owner-approved packet is published by the owner in a permitted medium; the Hub shows its recorded external reference with provenance and confirmed/owner-attested distinction. A missing or stale source blocks the commercial packet. Official affiliate links remain intact. Missing click/conversion/commission data is unknown, not zero or inferred revenue. Fixture evidence cannot satisfy this real pilot gate.
- **Tests:** T03, T04, T16 and T18 offline; separately record a narrowly authorized human publication observation. No automated test connects to a real account.
- **Risks:** confusing self-reported publication with API-confirmed delivery, incompatible affiliate/channel rules, expired prices and missing disclosures. Preserve evidence level in the report.
- **Costs:** no software-required model spend; owner accounts, program eligibility and any hosting cost require separate choices. No paid advertising, purchase or subscription is included.
- **Rollback:** disable further packet approval/export for the campaign; retain external references and commercial records. Removing or editing an already published post is a separate authorized external operation, never an automatic rollback promise.

## C3 — Standing grants, editorial occurrences and cost admission

- **Objective:** prepare a restart-safe autonomous schedule while external publication remains simulated.
- **Repositories and reuse:** ZaturoBot; reuse Engine, db atomic transactions, task operation IDs, worker leases, existing budget/journal patterns and future scheduler contract.
- **Changes:** implement explicit versioned standing grants, emergency-stop revision, account/category/content/cadence scopes, revocation, deterministic occurrence admission, IANA timezone scheduling, bounded catch-up and an outbox-style operation intent. Add fixed-point commerce monetary reservations when a paid adapter is introduced; preserve existing token ledgers. Maintain one authoritative payer budget, either central reservation or bounded nonoverlapping allocations.
- **Dependencies:** C1; approved meaning of each grant and money currency; recurrence library/dependency review; retained legacy schema tests. Proposed default is one campaign/account, one post/day, one publisher in flight, at most 30 future occurrences. These defaults are not permission.
- **Acceptance:** a fake-clock month produces exactly one admitted task per occurrence key despite two schedulers/restarts; cancelled/missed windows do not accumulate an unlimited backlog; stale/revoked grants stop admission and worker pre-send checks. Unknown/overrun charges survive restart. No commerce purpose can use the existing PublicCoding or Personal permission. All runs remain dry-run until C4's external gate.
- **Tests:** T05, T07–T12, T15 and T19; concurrent SQLite admission races, time rollback/forward, DST ambiguity and money overflow included.
- **Risks:** treating task deadline as a schedule, stale offline grants, duplicate admissions, budget double allocation and restoring an obsolete backup that recreates allowance.
- **Costs:** local scheduling has no model requirement. New paid budgets default to zero; do not borrow MeuAssistente's cumulative USD10 validation reserve.
- **Rollback:** disable schedule admission and revoke commerce grants, retain occurrence/intents and reservations. Do not rewind next-occurrence state or recreate spent allowances. Restore schemas only under the operations procedure.

## C4 — One official publisher (MVP-A)

- **Objective:** publish one small campaign unattended inside a specific standing Level3 grant and report confirmed outcomes.
- **Repositories and reuse:** ZaturoBot only; reuse C1/C3 and the worker's fixed-adapter and durable service pattern. A concrete transport crate is justified only now if needed.
- **Changes:** implement one official channel adapter, dedicated endpoint-local secret reference, capability/identity binding, quotas, per-channel breaker, receipt validation, bounded status observation and uncertainty quarantine. Candidate: an owned Telegram channel only if the affiliate program explicitly permits that medium; otherwise an authorized owned site or continued assisted mode. Do not add several publishers at once.
- **Dependencies:** C3; current official API/platform/program evidence, eligible account and approved medium; explicit owner standing publication grant; actual channel identity; scoped credentials installed through an approved process. Telegram has no general idempotent-send guarantee; lack of a reliable reconciliation method means ambiguous sends require owner review, not retries.
- **Acceptance:** an authorized small live pilot yields a verified public post ID/URL and content/account binding; a lost acknowledgement fixture yields Uncertain and zero second sends. Revocation stops unsent occurrences. Hub summary separates prepared, admitted, published, failed and uncertain counts. Run observation does not imply conversions or revenue.
- **Tests:** T07, T09, T12–T15 and T18 offline; an official sandbox only establishes transport behavior. Any real publication smoke is separately authorized and recorded with exact scope and actual receipt.
- **Risks:** channel permission change, rejected affiliate medium, partial publication, expired credentials and duplicate sends after timeout. Release remains blocked if these cannot be fenced safely.
- **Costs:** no paid broadcasts or ads; use conservative free API limits. Hosting/paid service costs need an approved cap. Measure worker/hub RSS and storage under the bounded pilot.
- **Rollback:** emergency-stop/revoke the publisher, leave receipt/uncertain ledger intact and return to packet export. Disabling never retracts an already delivered post; deletion needs separate authorization.

## C5 — Optional eligible commerce inference

- **Objective:** improve strategy/copy quality with bounded structured proposals without changing publication authority.
- **Repositories and reuse:** ZaturoBot `llm`/`orchestrator`/worker/db/security; reuse existing Router/LlmClient/BudgetPort rather than parallel clients. MeuAssistente's model allowance and ledger remain unchanged.
- **Changes:** new explicit commerce purpose/data policy/profile, strict proposal schema and evidence references, price-revision cost reservation/settlement, minimized prompt context and deterministic acceptance. Add one eligible approved provider or qualified local adapter, not both by default. Stronger models require their own justified allowlist/budget change.
- **Dependencies:** terms/commercial purpose/privacy/region/retention review from [provider strategy](08-AI-PROVIDER-AND-COST-STRATEGY.md); explicit owner model and spending authorization; validated quality set. C1 templates remain available. C3 monetary admission is required before any paid route.
- **Acceptance:** valid proposals improve a declared evaluation target while every fabricated price, changed affiliate link, invented test and unauthorized action is rejected. Unknown billing stays reserved and no fallback occurs. Provider-disabled operation still produces deterministic drafts. A local route records actual hardware, model/license/hash, peak memory, latency and quality before eligibility.
- **Tests:** T04, T08–T11 and T17; fake model/HTTP fixtures only in CI. A paid smoke needs separate explicit authorization and cannot be funded by relabeling validation.
- **Risks:** upstream purpose/retention changes, model hallucination, hidden reasoning/media charges and cross-project double spend.
- **Costs:** owner-approved fixed-point request/task/campaign/day/month/cumulative caps; zero new paid calls until approved. Record subscriptions, media, taxes and currency separately from token estimates.
- **Rollback:** disable the commerce route, preserve settled/unknown charges and cached accepted drafts, revert to deterministic templates; no automatic provider substitution.

## C6 — MeuAssistente verified-facts bridge in synthetic mode

- **Objective:** prove fact-grounded conversation integration and takeover/consent fencing without commercial WhatsApp use.
- **Repositories and reuse:** ZaturoBot plus a **new explicit authorization to edit MeuAssistente**. Reuse its centralized Llm, actual MCP/service ports, grounded renderer, incoming dedupe, outbox uncertainty, operator CAS, account/chat filters and Tauri controls. Current access is read-only.
- **Changes:** separately authenticated versioned facts/draft/status contract, dedicated scoped service identity, immutable offer snapshots, expiry and callback replay fences; add typed commerce tools/renderer through reviewed static registration. Keep database ownership separate and restaurant semantics intact. Use a fake/synthetic channel.
- **Dependencies:** C1 contracts; receiving-repository authorization; consent/retention and ownership design. Do not remove `synthetic_only=1`, bypass fake-only constructors, select a new phone/profile or activate a Meta API.
- **Acceptance:** replayed commands yield one recorded draft; changed payload under an old key fails; stale facts produce an unavailable response; takeover, block or opt-out between draft and dispatch stops it; metadata-only result reaches Hub once. No production send is claimed.
- **Tests:** T11–T14 and T20, plus MeuAssistente's existing ownership/filter/outbox/provider tests and full quality gate in its authorized checkout. Its cumulative USD10 historical/pending charges remain byte-for-byte or logically preserved through additive migration.
- **Risks:** mistaking synthetic technical admission for commercial consent; leaked contact identifiers; remotely forging a trusted operator. A service capability cannot impersonate an operator or release a human handoff.
- **Costs:** synthetic fixtures require no paid model or messaging account calls. Production provider/channel costs and consent remain a later decision.
- **Rollback:** revoke bridge capability and stop new bridge work; preserve both histories/outboxes and current browser profile/session. Production communication is a separate future phase after explicit eligibility decisions, not an automatic continuation of C6.

## C7 — Optional branded media production

- **Objective:** add a bounded real media-generation path while keeping its publication permission separate.
- **Repositories and reuse:** ZaturoBot and the already authorized isolated YoutubeBot checkout only when its changes are requested; preserve its extension/recorder/generation/upload behavior and original local files. Reuse fixed JSON verbs, independent heavy-job units and matching-operation status.
- **Changes:** commerce media manifest references accepted script, brand revision, licensed inputs, requested renditions and output hashes; adapt the fixed generation contract only as needed. Add one template/rendition at a time. Do not invoke scheduled upload to obtain a generated file.
- **Dependencies:** C1; hardware measurements; endpoint-local interactive Google/ChatGPT readiness; appropriate media/provider rights and budget. Generation and publication each require their own grant.
- **Acceptance:** requested artifacts exist and validate format/count/brand/disclosure before media completion; accepted background work alone is insufficient. Interrupted submit reconciles its original operation; missing login is distinguished from unknown status. Existing upload/extension behavior passes regression checks.
- **Tests:** T06, T09, T15 and T17 plus YoutubeBot's focused/gate commands in its authorized isolated checkout. No real account challenge, rendering charge or upload in ordinary tests.
- **Risks:** resource contention with ongoing work, expensive failed media, licensing and generated-media disclosures.
- **Costs:** separately capped jobs/storage/CPU and any media charges; no assumption that the notebook fits the configured heavy-unit maximum. Schedule heavy work only within approved local limits.
- **Rollback:** disable new media admission and return to static assets; cancel owned work where safe, retain unknown paid jobs and original files, never erase legacy upload/profile data.

## C8 — Verified performance and price intelligence

- **Objective:** report attributable results from authorized exports/APIs and observed prices without inventing revenue or historical lows.
- **Repositories and reuse:** ZaturoBot; reuse db bounded import cursors, idempotency, immutable evidence, report revision and feed summary. No direct affiliate database, contact scraping or generic tracking endpoint.
- **Changes:** one authorized export/API importer; stable source transaction IDs, return/cancellation revisions, observed price series, currency-safe aggregates and metric freshness. Keep gross/pending/approved/settled/reversed commissions distinct. Use aggregate analytics first.
- **Dependencies:** C2 for real source records; approved data/export license, retention and attribution rules. C4 not required for owner-assisted campaigns. Any public redirect/analytics service requires separate security/privacy review.
- **Acceptance:** duplicate imports do not double count; late reversal changes the correct report revision; unknown values remain unknown; mixed currencies are not summed without an approved recorded conversion method; a single price observation never becomes history. Hub totals trace to source records and campaign/link references.
- **Tests:** T03, T10, T16 and T19; malformed rows, partial imports, restart, duplicate page and correction cases included.
- **Risks:** stale or unavailable program exports, disputed attribution, PII retention and zero-versus-unknown confusion.
- **Costs:** no model needed for arithmetic; bounded local storage. Paid data access is disabled pending explicit approval.
- **Rollback:** disable importer, retain cursor/evidence/audit and clearly mark reports stale; do not rewrite past settled results or delete source corrections to improve reported performance.

## C9 — Bounded optimization (Level4)

- **Objective:** adjust explicitly preapproved editorial variables from verified evidence while maintaining hard scopes and stop controls.
- **Repositories and reuse:** ZaturoBot; reuse C3 grants/budget/occurrences, C8 aggregates and accepted content policy. Do not create free-running debating agents.
- **Changes:** deterministic experiment enrollment, one-variable variants, fixed audience/content constraints, minimum sample/freshness requirements, evaluation windows and reversible promotion rules. A model may propose a tested hypothesis within C5; it cannot raise limits, buy traffic or enroll new channels.
- **Dependencies:** C4 safe publication; C8 trustworthy metrics; separate Level4 owner grant. Until sufficient observations exist, report insufficient evidence and retain baseline.
- **Acceptance:** an experiment cannot change destination/category/disclosure/budget or exceed cadence; stopping or revoking it prevents new variant admission; the report states sample/window/uncertainty and does not claim causation from insufficient data. Baseline restoration creates future occurrences without replaying old ones.
- **Tests:** T05, T07, T10, T16 and T19, with deterministic synthetic outcomes and adverse-result stop conditions.
- **Risks:** optimizing commission over user value, selection bias, inadequate samples and accumulating spend. Keep evidence/ranking exclusions authoritative.
- **Costs:** no mandatory inference; any incremental requests stay within the existing commercial envelope. No autonomous financial action or paid acquisition.
- **Rollback:** disable experiment, pin the last approved baseline for future content, preserve experiment IDs/results and all sent-operation evidence.

## Release decision

A phase is complete only when its behavioral acceptance evidence exists, its full required checks pass, and unresolved production gates are explicitly recorded. Draft, fixture, sandbox, owner-attested and provider-confirmed evidence are distinct. No phase may borrow the success claim of another repository or older deployment. [Operations](11-OPERATIONS-AND-RECOVERY.md) governs rollback after any external effect.
