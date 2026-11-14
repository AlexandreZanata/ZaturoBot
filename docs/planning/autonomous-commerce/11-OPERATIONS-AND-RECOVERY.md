# 11 — Operations and recovery

Status: proposed runbook for a later implemented commerce slice. No service, account, schedule, credential or deployment is changed by this document. Dependencies: [architecture](03-SYSTEM-ARCHITECTURE.md), [integration contracts](04-INTEGRATION-CONTRACTS.md), [policy](06-AUTONOMY-AND-POLICY-MATRIX.md), [cost strategy](08-AI-PROVIDER-AND-COST-STRATEGY.md), [acceptance](10-TEST-AND-ACCEPTANCE-PLAN.md).

## Operational ownership

The Hub is authoritative for campaign/grant revisions, schedule occurrences, existing TaskId/OperationId, admitted intent, owner stop state and curated reports. Its database stays lightweight. A worker owns its durable execution/send boundaries, endpoint-local secrets, reserved/uncertain costs and cached evidence. MeuAssistente owns conversation authority, local consent/filters, human takeover and message outbox in its own database. YoutubeBot retains its media jobs and original account/channel/profile locks. There is no shared SQLite file or cross-service SQL repair command.

No new process is required for scheduling by default: a bounded Hub loop queries due commerce records and atomically admits ordinary Engine tasks. Model/media/network execution stays on the notebook. Existing diagnostics, encrypted files and native chat remain independently usable when commerce is paused. A commerce feature outage must not require stopping unrelated ongoing work.

The owner configures account identity, channel eligibility, brand/content/category scopes, mandatory editorial window, cadence, expiry, cost limits, retention and reporting frequency before activation. Missing configuration means draft-only. There is no default public destination or default production grant. Interface text defaults to pt-BR; persisted times are UTC.

## Durable editorial scheduling

Persist IANA timezone (`America/Cuiaba` initially), local window, cadence, schedule revision, next eligible UTC instant, content/BrandKit references, grant revision, missed-window policy and stop semantics. The current daily feed's fixed UTC-4 offset is a view convention, not a recurrence engine. Use a reviewed timezone library and record the rule/version needed to audit occurrence resolution. Never implement schedules by sleeping until a wall-clock deadline or by making a model continuously poll.

Proposed deterministic rules, to confirm before scheduling implementation:

- An occurrence key binds campaign, schedule revision, local calendar occurrence and selected UTC instant. Enqueue task + occurrence + admitted operation intent + scoped counters atomically in the Hub. A unique key prevents two schedulers/restarts admitting the same occurrence.
- No window is active until the owner selects it. Initial proposed scope is one post/day, one campaign/account, one publisher in flight and at most30 future occurrences; increasing scope requires a new grant revision.
- For a nonexistent local time during a DST gap, skip and record the reason. For an ambiguous fold, choose the earlier UTC instant once, recording the offset/fold decision; never produce two posts. Test other timezones even though current Cuiaba rules do not have seasonal transitions. A later timezone-rule change recalculates only unsent occurrences under a new schedule revision and preserves all original keys.
- If the worker was offline beyond the window, default to **skip and report**, not catch up with a burst. A specifically approved catch-up policy may admit at most one still-valid occurrence within the next authorized window, with current facts, permission and budget. No infinite backlog recovery.
- Before dispatch, revalidate facts, official link/medium eligibility, exact account/content/brand/grant revision, freshness, consent if relevant, current stop state and reserved budget. A scheduled draft becoming stale is blocked, not silently rewritten under its old approval. Material edits create a new revision and policy evaluation.
- Use monotonic elapsed time for timeouts and UTC wall time for persisted due/expiry evaluation. A backward clock jump cannot refund spending or recreate occurrences. A forward jump applies missed-window rules. Unreliable clock synchronization blocks time-sensitive publication and produces a diagnostic exception.

A schedule record is not itself a grant. Calendar text, incoming messages, imported content or absence of an owner response cannot authorize publication.

## External operation procedure

For every side effect, retain task/operation IDs, payload/content hash, account/channel, grant/brand/fact revisions, request/receipt correlation, dispatch-attempt boundary, reserved cost and status evidence. Use the proposed EffectReceipt states from [integration contracts](04-INTEGRATION-CONTRACTS.md): `prepared`, `accepted`, `dispatching`, `confirmed`, `rejected_no_effect`, `uncertain` and `cancelled_before_dispatch`. These track effect evidence for the existing operation, not another task queue or competing task lifecycle. Their eventual Rust definition remains authoritative.

1. Admit and persist the intention under the existing task engine. Reserve all relevant budget and retain the price revision before any paid/network send.
2. Worker checks assignment/lease/expiry/local capability and current authority. Persist the irreversible send boundary before starting external IO; use the provider's documented idempotency key only when it actually supports one.
3. Send the bounded request once for that attempt. Validate response account/content/operation binding and its actual guarantee. Accepted background work is not generated media; an observed message bubble is not delivery; a click is not commission.
4. Persist validated receipt/result before acknowledging Hub completion. Hub validates typed acceptance and stores one stable report. Replayed complete local evidence must not issue another remote request.
5. On lost response, timeout, process death, cancellation after send, malformed receipt or unknown billing, mark Uncertain and retain the full reservation. Reconcile by the same operation/provider ID where supported. A status saying not found is conclusive only if the platform contract guarantees it covers the relevant send/consistency window.
6. If reconciliation cannot establish outcome, quarantine that effect and prevent a replacement disguised by a new key, schedule revision, provider or campaign. Owner review may resolve it with documented evidence; it cannot infer that nothing happened merely because no receipt was saved.

Do not blindly retry an uncertain side effect. Safe read-only status queries may use bounded retry. A definite pre-send validation denial can be retried after the underlying condition changes using the same recorded intent as appropriate. A documented definite provider rejection may permit a new attempt only under its reviewed adapter contract, limit, fresh authority and budget. There is no generic HTTP5xx-means-safe-to-resend rule.

Telegram send APIs do not provide a general client idempotency key or universal post lookup by our operation ID. A lost successful acknowledgement therefore remains quarantined until supported evidence or owner inspection resolves it. Do not automatically resend with a new message, switch channels or claim exactly-once delivery. A pilot may need occasional exception review; this is preferable to an invented delivery guarantee.

## Outage and interruption scenarios

**Hub restart:** reopen supported schema, preserve stop/grant epochs, reconcile task leases and occurrence keys, then recover a bounded page. Never recreate schedules from scratch. Report feed state comes from committed facts; a notification lost in RAM is regenerated from a stable dedupe key, not a new publication.

**Worker restart or lost connection:** reopen the existing journal and compare the same task/operation identities. Completed cached evidence is safe to report; begun unknown sends/paid calls require status reconciliation. Offline work may prepare local drafts if explicitly allowed, but must not begin publication using stale authority. Reconnection never re-enrolls the device, replaces keys or deletes its journal.

**Provider or channel failure:** open its breaker and stop only that route's new effects. Proposed breaker: after3 consecutive eligible transport/rate failures, pause admission for5 minutes; permit one read-only health/status probe before reopening. These values are bounded configuration proposals, not platform limits. Honor longer Retry-After without shortening it. Unknown sends remain quarantined independently of breaker recovery. No unapproved fallback provider/account is selected.

**Media actor loss:** use YoutubeBot's original operation status/channel/count/mode to reconcile. Accepted actor launch is not completion. Do not resubmit generation/upload from scratch because a status observer restarted. Preserve original media/profile files and separate heavy-unit limits. If cancellation cannot determine billing or remote effect, report uncertainty.

**Authentication expiry or interactive challenge:** block that endpoint with a curated reason and ask for owner sign-in on the correct machine through its existing human flow. Do not copy cookies, access session storage, defeat a challenge, change the connected WhatsApp profile or automatically consent to OAuth.

**Stale product/price or policy evidence:** stop affected packet/publication, retain source and rejection reason, schedule only an authorized source recheck. Never invent a replacement price or change an official link. A policy review deadline is an activation gate, not permission for an LLM to accept terms.

**MeuAssistente unavailable or human-owned:** preserve its command/operation key and use authenticated bounded status. No alternate direct-message channel or restaurant operation is used as fallback. On takeover/opt-out, invalidate queued automated work; only an explicit operator act under its local policy can release human ownership. WhatsApp production stays blocked under current synthetic restrictions.

## Emergency stop and revocation

A visible Hub action should stop commerce admissions and revoke the relevant grant epoch: global commerce, campaign, account/channel or provider. Worker must check local capability/expiry and current stop/grant state immediately before each effect. Proposed dispatch authority is operation-bound, short-lived and single-use; exact maximum validity is a contract decision tested against outage latency, never an unlimited cached permission. If current authority cannot be checked or the authority lease expires, fail closed for publication/messaging.

A Hub stop first commits **StopRequested/Pending**, issues no new dispatch permits and notifies every affected executor. Each worker and communication endpoint must serialize its local durable fence against actual send admission, cancel reversible owned preparation, and acknowledge its fenced epoch plus quiescence/effect status. The Hub may report the stop effective only after every affected executor acknowledges that no new send can begin and all already-started calls are terminal or explicitly identified as unresolved. An offline executor leaves the stop pending until it acknowledges its fence; short-permit expiry, heartbeat loss or task cancellation alone is not evidence that an active call ended. Any unresolved remote outcome stays separately Uncertain, even when future local dispatch is fenced.

Distributed stop cannot recall a request already in flight or prove remote deletion. Record the actual pre-send/after-send boundary and notify the owner if an effect may have crossed it. Report StopRequested, endpoint acknowledgements, effective dispatch fencing and unknown remote effects as distinct facts. Measure propagation and its maximum tested delay; do not promise instantaneous revocation across a network partition. Local emergency disable/credential revocation remains available on the endpoint.

Pause/resume is deliberate: resuming performs fresh facts/permission/budget checks and does not clear uncertainty, handoffs, bans or debt. A new grant does not reset old occurrence keys. Revoking a publisher does not authorize deleting its existing public posts. Deletion/account changes require their own explicit scope and durable operation.

## Budgets and unknown charges

Maintain one authoritative reservation record per payer/provider account. Account-wide allocation must cover all consuming projects; a second project cannot independently spend the full cap. Use fixed-point values/currency and checked arithmetic. Outstanding and uncertain reservations count against request/task/campaign/day/month/cumulative caps; overruns remain debt. UTC day/month transitions affect period counters only, not lifetime ceilings.

Preserve Zaturo's existing coding/personal aggregate records and MeuAssistente's cumulative USD10 validation history, pending reservations and fixed ledger identity. Commerce starts with zero approved new paid calls. An approved future commerce envelope is separate authorization, not a new name for resetting that reserve. Never repair a billing blocker by deleting/copying/replacing a ledger or retrying from a new checkout. Reconcile with authoritative provider evidence and keep a correction audit; absence of a price/charge is unknown, never automatically zero.

## Health, reports and alerting

Health should distinguish Hub readiness/schema, last worker heartbeat, due/missed occurrence count, stale facts/policy, breaker state, pending/uncertain effects, unavailable credentials by category and budget debt/remaining admitted envelope. Do not expose account identifiers, tokens or raw customer data on public health endpoints. The existing private transport remains; no new unauthenticated control API.

Routine reports use the configured local reporting window and existing ordinary Hub feed, with stable dedupe keys and source/as-of time. Keep summaries within the current4096-byte feed contract; larger typed detail is paginated/referenced through a reviewed authenticated contract. Urgent events are unresolved side effects, emergency stops, budget debt, credential/eligibility loss and privacy incidents. Group repeated failures; do not flood the owner with unchanged status. Missing metrics are unknown and pending commissions remain separate from settled results.

Retain bounded metrics for failed/admitted/published/uncertain operations, queue age, reconciliation age, request latency, duplicate denial, stop propagation and resource peaks. Report what was observed; a running unit, green mock or old benchmark is not evidence of new public delivery, capacity or revenue.

## Secrets, privacy and retention

Provider/platform secrets remain endpoint-local under dedicated service identities and restrictive file permissions. The Hub stores references, public metadata and digests where needed, not browser credentials or raw secrets. Rotation/revocation must be scoped, tested and retain operation correlation. Existing account connections and private profiles are not modified by planning or ordinary tests.

Before audience ingestion, approve a retention profile with purpose/lawful-basis/notice records, subject-rights handling, deletion deadlines, backup expiry and minimal audit obligations. Until then, use synthetic audience data only. Proposed MVP stores aggregate analytics and pseudonymous endpoint references; it avoids individual behavioral profiling. Do not place private messages, phone pixels, cookies or provider reasoning in commerce reports or model prompts.

Deletion must stop future targeting first, revoke consent, remove unnecessary payloads and retain only a justified minimal suppression/dedupe tombstone for a defined period. Deleting an outbox row must not restore send permission. Backups obey documented expiry and restore suppression rules; a restore cannot resurrect withdrawn consent or audience payloads indefinitely. Actual retention durations and legal exceptions require owner/legal approval before production; this plan does not invent them.

## Backup, restore and rollback

Before a schema upgrade, quiesce the affected writer and take a consistent SQLite backup with its binary/schema/config revision and artifact-manifest inventory. Plain file-copying a live WAL database is insufficient. Preserve existing ignored private data and enrollment. Keep encrypted backups only at an explicitly approved destination with a tested recovery-key process; do not introduce a general vault or new cloud account in this slice.

Proposed pilot objectives: recover the control/reporting path within60 minutes after a supported failure, with at most24 hours of ordinary report metadata lost. These are unmeasured RTO/RPO targets to confirm and test. **They do not permit forgetting external effects, consent withdrawal or charged usage.** Every effect needs durable intent and receipt/journal evidence before acknowledgement, with independent surviving Hub/worker records where possible. Losing both stores or restoring backups behind a possibly sent effect fences the affected account/campaign until authoritative reconciliation; an old backup cannot safely create allowance or resend authority.

Recovery order:

1. Keep commerce stopped and isolate the affected scope; preserve surviving journals, external IDs and incident times. Do not overwrite the current evidence with the backup.
2. Verify backup integrity/schema and restore into a separate local staging path with the matching binary. Apply only supported additive migrations after a new consistent backup.
3. Reconcile Hub intentions/occurrences with surviving worker/provider/MeuAssistente/YoutubeBot evidence. Include all pending/unknown charges, old grant epochs and consent suppression. Missing evidence remains quarantined.
4. Verify source freshness, current scope, endpoint identity, output artifacts and cost totals; run offline restore acceptance. Measure elapsed recovery and actual data gap.
5. Resume only explicitly eligible future work under a fresh current grant. Leave unknown prior effects blocked and report them. Do not replay historical occurrences to make the dashboard look complete.

Feature rollback normally means disabling new commerce work and returning to draft/export mode while retaining new tables and evidence. Binary rollback requires a schema-compatible binary or consistent old snapshot plus reconciliation of all newer external work. Never edit an applied migration, clear the operation journal, discard validation consumption, automatically undo public posts or reset unrelated ongoing work.

## Incident record and acceptance

Record UTC and local display time, affected revisions/scope, last verified state, uncertain operations/cost, containment action, evidence source, actual recovery, unresolved owner decisions and next safe step. Redact secrets and minimize personal data. A security/privacy incident may require a separate legal/notification response; external communication follows explicit authorization and current law, not an automatic generic sender.

Before unattended activation, demonstrate T05/T07/T09/T10/T12/T15/T19 from the [acceptance plan](10-TEST-AND-ACCEPTANCE-PLAN.md): occurrence uniqueness, stop/revocation, unknown-send quarantine, preserved spend, fair observation, bounded resources and restore without replay. Run an actual authorized small pilot only after these pass and platform/account gates are complete. No simulated recovery establishes that a real uncertain send was safely undone.
