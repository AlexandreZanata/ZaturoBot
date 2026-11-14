# 10 — Test and acceptance plan

> Historical planning baseline at `eb6a0cb`. The [refined plan](../commerce-refined/ZATURO-COMMERCE-REFINED-PLAN.md) supersedes it where different; [current commerce contracts](../../COMMERCE.md) and [F1 evidence](../../COMMERCE_F1_REPORT.md) describe implemented behavior.

Status: future behavioral contract; not a claim that commerce tests exist or passed. Use the [audit](01-REPOSITORY-AUDIT.md) for current source/test evidence and [handoff](13-CLAUDE-HANDOFF.md) for commands actually run in this planning pass. Dependencies: [contracts](04-INTEGRATION-CONTRACTS.md), [policy](06-AUTONOMY-AND-POLICY-MATRIX.md), [roadmap](09-IMPLEMENTATION-ROADMAP.md), [operations](11-OPERATIONS-AND-RECOVERY.md).

## Harness and evidence boundary

Use temporary separate hub/worker/MeuAssistente databases, synthetic identities and fixture credentials, injected UTC time, fake model/budget ports, fixed fake media executables and loopback fake APIs. No production account, real credential, paid inference, provider subscription or live publication is a CI dependency. Do not run tests against the owner's connected WhatsApp profile, current services or private validation ledger. Make old-schema fixtures from synthetic records; do not copy owner databases into tests.

Test the observable invariant and failure path, not the exact implementation or a copied list of constants. Record admitted task IDs, operation IDs, provider-call counters, journal states, receipts, report revisions, artifact hashes and denial categories. This allows a reviewer to distinguish a stopped operation from a silent no-op. Never log raw secrets, audience identifiers, provider error bodies or browser state.

For side effects, the fake provider counts received sends durably outside the worker database. Kill the worker at defined barriers and verify the provider counter after restart. A Hub COMPLETED state, HTTP200, successful subprocess exit or assertion that a method was called is not enough to prove publication. The harness must verify the intended content/account/operation binding and receipt semantics.

## Acceptance cases

### T01 — Independent disable and legacy preservation

Open synthetic copies of every supported old schema, preserve diagnostic events, uncertain model reservations, accepted/uncertain service intents, shared records and feed rows, then enable and disable commerce. Existing direct/assisted diagnostics, transfers, chat, generation-only service and native feed still satisfy their current contracts. No commerce grant or route appears when disabled. Unsupported newer schema fails closed. Inspect actual current schema streams before naming migrations; baseline Hub7/worker8 is historical, not a reserved number.

### T02 — Complete offline agency packet (C1)

Submit one fixture campaign containing five product records, including stale, unsupported and invalid-link candidates. Produce the two accepted candidates and three planned branded drafts specified by C1, with deterministic reasons for rejected records, asset manifests and one deduplicated factual report. Reopen both stores and repeat the request with its original key: same IDs/results, zero external calls and no duplicate report. Change the body under that key: conflict, original evidence unchanged.

### T03 — Product, price, affiliate and source integrity

Reject unknown source permission, expired observation, wrong variant/currency, impossible amount, invented previous price, absent official link provenance, prohibited destination/shortener and an unapproved affiliate medium. Preserve the exact accepted official link. Every displayed price traces to source/time/variant/conditions. An observation without history cannot substantiate “lowest ever”; no simulated return value may create a price history or commission.

### T04 — Brand, claim and disclosure acceptance

Materially change BrandKit, source facts or content after a supervised approval: the old approval no longer authorizes that content. A draft claiming a physical test without evidence, fabricated discount/scarcity, hidden advertising label, unsupported specification or marketplace endorsement is rejected. Render fixture assets at target sizes, visually inspect legibility/safe areas, test text contrast and alt text. Record brand/rights hashes. Deterministic templates and optional model drafts face the same validators.

### T05 — Editorial time and bounded occurrences

Use `America/Cuiaba` local windows and persist UTC. Race two occurrence-admission transactions at the same boundary: exactly one task per campaign/schedule-revision/occurrence key. Reopen before and after admission; no duplicate. Cover local midnight, month end, leap day, clock backward/forward, outage beyond the window, schedule change and revoked/expired grant. Exercise another IANA zone with a gap/fold to prove explicit DST rules; current fixed-offset feed behavior does not prove recurrence correctness. Bound catch-up and horizon; a long outage cannot enqueue an unbounded backlog.

### T06 — Artifact and transport bounds

Round-trip supported envelopes, reject unknown fields/versions and unsupported capabilities, and test empty/oversized/truncated payloads and archive/path traversal. Baseline task transport is16KiB, service result4096 bytes, foreground task timeout<=300 seconds and ordinary feed text<=4096 bytes. Do not send a proposed32KiB commerce bundle or8KiB report through these existing limits: use bounded references/pagination and compact summaries or explicitly review a new contract with compatibility tests. Media bytes never enter task text/feed. Long jobs complete through validated status evidence, not a larger unbounded foreground timeout.

### T07 — Standing authority and emergency stop

For each autonomy level, test an eligible in-scope request and a denial for new account/channel/category, missing approval, stale content/grant revision, expired grant, exceeded cadence or budget. A model/shared note/customer message cannot create or extend authority. Race stop/revocation against schedule admission, assignment and actual worker/communication-endpoint send admission. Hub StopRequested/Pending blocks new permits; every affected executor commits a local durable send fence and acknowledges quiescence/status before the Hub reports effective dispatch stop. An offline executor leaves stop pending; permit expiry alone cannot prove a previously active call ended. After an acknowledged local fence, verify no new send begins. Document all calls already sent and all unresolved remote effects separately; do not claim they were recalled.

### T08 — Provider purpose and model isolation

Attempt commerce through current Go/PublicCoding and Console/Personal profiles: zero provider calls and explicit denial. Unknown model, new region, excessive retention/training, missing terms review or absent commercial budget also deny. Existing DeepSeek allowlist remains unchanged. A failed/budget-exhausted route cannot silently select another model/provider. A local route requires a separately approved capability and model/license/hash; loopback reachability alone is insufficient.

### T09 — Every external send boundary

Crash before intent, after intent/before send, after send/before acknowledgement, after receipt/before journal commit, after journal/before Hub acknowledgement and after completion/before report. Reopen and reconcile using the same operation and payload hash. A committed complete receipt replays locally without another send. A timeout/lost acknowledgement becomes Uncertain; no new operation ID or fallback provider is used to hide it. For Telegram, assert one received send and no second send after ambiguous success because no general client-idempotency key is available. A malformed or wrong-account receipt must not complete the task.

### T10 — Monetary admission and preserved history

Use integer fixed-point amounts and dated price revisions. Concurrent requests compete for request/task/campaign/day/month/provider/cumulative caps: total settled+reserved+uncertain allocation never exceeds admitted authority, except an overrun retained as debt and blocked thereafter. Reject arithmetic overflow, missing currency, unproved cache discount and unsupported conversion. Day/month rollover does not reset task/cumulative spend. Price changes cannot make an already uncertain request free. Shared payer envelopes must sum to at most the parent cap.

Preserve synthetic representations of MeuAssistente's cumulative USD10 ceiling and all reserved/settled/uncertain historical consumption across reconnect, new task namespace, checkout/data-directory change and additive migrations. Existing Zaturo coding/personal totals remain included in their aggregate policy. Never inspect or reset the real ledger for testing. A commerce budget is independently approved; it cannot reinterpret the validation reserve as commercial spending.

### T11 — Authentication, minimization and untrusted input

Reject missing/wrong/expired/revoked service credentials, role confusion, unsigned/unbound account assertions, wrong audience/scope and insecure origins. Dedicated integration credentials cannot administer accounts, release human takeover or call arbitrary Hub owner routes. Verify error/log redaction. Inject malicious repository/product text, callbacks, HTML and model output attempting a new tool, secret read or destination change: no authority change. Test that prompts contain only minimized public evidence; customer names/numbers, private messages, cookies, screenshots and feed history do not appear.

### T12 — Contract replay and ordering

Exact repeat returns the recorded result, not fresh execution; changed payload/key binding fails. Callback replay/out-of-order sequence cannot regress Completed/Uncertain, erase a newer exception or double-count a report. Unknown operation and mismatched channel/account/hash fail. Simulate dropped response and status lookup before any retry. Prove retention of dedupe tombstones for the agreed replay horizon; a deleted delivery record cannot reopen send authority.

### T13 — Audience withdrawal and human ownership

In synthetic MeuAssistente fixtures, deny unknown audience permission, empty allow-only sets, explicit blocks and unsupported group identity. Revoke consent or claim human ownership between fact fetch, draft save, enqueue and actual send: every later automated transition is fenced. Re-activation must not release a human pause or drain old history. Opt-out is actionable without requiring an LLM and blocks queued future messages. Existing filters are technical controls, not evidence of production consent.

### T14 — Production boundaries remain closed

Fixture-only WhatsApp constructors and `synthetic_only=1` remain enforced. Neither bridge capability nor a valid offer activates production messaging, the Meta API or a new phone/profile. TikTok intended-use restrictions and missing affiliate medium approval block autonomous publication. Provider sandbox/Trial success is labeled sandbox; local generation is labeled generated/draft. Test that no simulated send is rendered as delivered or real commerce success.

### T15 — Fairness, cancellation and resource containment

A batch of20 unchanged/unreachable jobs cannot starve the next eligible job; observing status must not block heartbeat/control. Cancellation/stop terminates owned reversible subprocess groups where supported and marks sent/paid uncertainty correctly. Bound queues/pages/HTTP bytes/timeouts/concurrency, rendered asset decode and disk/artifact quotas. Measure hub idle and bounded-pilot peak RSS, worker/media peak resources and reconnect behavior on the actual eligible hardware. Requirements'150MB normal/300MB peak targets remain targets until measured; old diagnostic benchmarks are not commerce capacity evidence.

### T16 — Analytics truth and corrections

Import the same synthetic report/page twice and restart between pages: no duplicate clicks/conversions/commissions. Apply cancellation, return, late settlement and corrected source versions to the right transaction/campaign. Keep source-unavailable distinct from measured zero; do not add unrelated currencies or treat a self-reported purchase as confirmed commission. Reports expose as-of time, source/import revision and pending/settled/reversed amounts. No statistical conclusion from insufficient samples.

### T17 — Model/media quality and bounded repairs

Evaluate structured proposals on fixture facts with fabricated-evidence adversaries, malformed JSON, wrong IDs and incomplete outputs. Each accepted claim resolves to permitted evidence; errors yield a draft exception. Count model calls/repair attempts against one admitted task budget. A YoutubeGenerate job cannot select scheduled upload; missing login, unknown readiness and unavailable service remain distinct. Completion requires requested ready artifacts and matching operation/mode/count, not actor acceptance or guessed file names. Preserve YoutubeBot's existing extension/upload tests when an authorized change touches it.

### T18 — Separate real pilot evidence

Offline fixture acceptance qualifies code, not platform/account eligibility or delivery. A later real smoke needs explicit owner scope, eligible account/channel, allowed affiliate medium, exact bounded content, cost limit, expiry and recovery plan. Observe the real external reference and verified account/content where the API permits, save curated receipt metadata, and reconcile spend. For owner-assisted publication, label the evidence owner-attested unless independently verified. Do not run this step during the planning task or normal tests.

### T19 — Backup, restore and experiment safety

Restore consistent synthetic backups with matching binaries and retain unknown-operation tombstones, grant epochs, occurrence keys, import cursors and reserved/debt costs. Simulate remote effects newer than the backup: startup is paused until reconciled; an old database cannot authorize sending again. Measure restore duration against the proposed objectives. Experiments cannot change unapproved destination/category/disclosure/cadence/budget; absent adequate fresh metrics they retain baseline. Stopping/reverting a variant changes only future occurrences.

### T20 — Two-repository boundary

Run both libraries/services against separate temporary stores over the proposed authenticated loopback contract. Filesystem access to the peer database is unnecessary and denied in the harness. Verified snapshots expire, authenticated principal/payload bindings and schema versions are checked, and MeuAssistente grounded rendering cannot alter price/link/rules. Replay one command and callback across both restarts: one draft/intention and one Hub result; takeover prevents the synthetic send. Existing restaurant order/payment behavior and historical validation caps remain intact. Receiving-repository changes require separate explicit authorization first.

## Commands and release evidence

Future focused commands should follow the touched owners, for example `cargo test -p zaturo-core`, `cargo test -p zaturo-db`, `cargo test -p zaturo-services`, `cargo test -p zaturo-llm`, `cargo test -p zaturo-orchestrator`, `cargo test -p zaturo-youtube`, or the relevant worker integration target. These are suggested commands, not execution claims. Run the project's documented `cargo xtask check` after focused acceptance, and `cargo xtask secrets` before any authorized code publication. In an authorized MeuAssistente implementation checkout, run its corresponding focused suites and `cargo xtask check`; do not mutate its reference checkout or validation ledger.

A phase evidence record contains revision/branch, exact commands and exit/results, fixture definition, observed invariants, artifact/receipt references, resources when relevant, skipped/blocked checks, production gates and rollback verification. Preserve a failing check honestly; never weaken a policy, change expected reality or mark an incomplete phase implemented just to obtain a green command.
