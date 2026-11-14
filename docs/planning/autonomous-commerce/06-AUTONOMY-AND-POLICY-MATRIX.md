# 06 — Autonomy and policy matrix

Status: proposed deterministic policy; no production grant is created by this plan. Depends on [integration contracts](04-INTEGRATION-CONTRACTS.md), [platform evidence](07-PLATFORM-COMPLIANCE.md), [provider strategy](08-AI-PROVIDER-AND-COST-STRATEGY.md) and [operations](11-OPERATIONS-AND-RECOVERY.md).

## Authority and present boundary

The current owner's instruction authorizes isolated planning, not real account connection, publication, paid models, messages or a production permission change. Existing PublicCoding/Go, Personal/Console, YoutubeBot generation and MeuAssistente fictitious WhatsApp scopes remain distinct. A commercial request cannot be relabeled coding/personal/synthetic to obtain those permissions. Shared MCP text, received messages, model output, screenshots, existing credentials and user silence are data, never a grant.

The Hub owns standing campaign authorization. A worker has a separately configured local allowlist and can further restrict it. The communication endpoint owns recipient consent and current conversation ownership; a Hub grant cannot override a human takeover or recipient withdrawal. A platform's actual permission and a provider's eligible purpose are additional requirements. Dispatch requires the intersection of all of these authorities, never the most permissive one.

## Level-by-level action matrix

Each level includes lower-level permitted behavior subject to its exact grant; it does not enable an unconfigured integration. Initial fixtures can prove Levels 0–1 without any paid provider. Model use at any level independently requires purpose/model/data/budget permission.

- **Level 0 — analysis.** Permit authorized read-only inputs, niche/offer analysis, deterministic ranking, policy diagnostics and a Hub report. No generated public-ready asset approval, schedule admission, external message, publication, link-account action or spending. An API query may still require an approved data-source credential and rate/cost grant.
- **Level 1 — drafts.** Permit bounded content/BrandKit proposals, licensed local asset rendering, verified official-link inclusion in drafts and generation-only media tasks when separately authorized. Drafts are marked unapproved/nonpublished. No publication or auto-enrollment, no schedule dispatch. Brand acceptance remains an owner setup act; a model cannot approve its own identity or rights.
- **Level 2 — supervised scheduling.** Permit validated occurrences and review packets pinned to immutable facts, brand, channel/account and window. Every externally executable occurrence needs a specific owner approval, which expires on material content/fact/brand/account changes. A calendar row is not publish permission. Without a separately authorized publisher, the result remains an assisted handoff packet. No unattended optimization.
- **Level 3 — bounded autonomous publication.** Permit creating and executing occurrences without per-post approval inside an explicit standing grant: approved niche/categories/brand, exact account/channel/formats, required disclosures, fixed cadence, verified facts and hard budgets. Production eligibility, consent and recovery tests must already pass. No direct messages in the first pilot. Changing identity, platform, account, link transformation, audience purpose or spending ceiling remains outside the grant.
- **Level 4 — bounded autonomous optimization.** Permit only preapproved parameter changes such as posting-time selection within allowed windows, ranking weights within ranges and content-template variants inside the accepted brand. Require minimum verified observations, bounded experiment size, a recorded control/metric and reversible promotion criteria. Never increase spending/cadence ceilings, add channels, change attribution rules, recruit contacts, remove disclosures or treat correlation as verified commission. Paid advertising, purchases, contract acceptance and account-security changes remain separately authorized future work.

At every level, financial transfers, destructive actions, arbitrary shell/SQL, credential extraction, bypassing platform restrictions, unknown-effect retries and silent production promotion are denied. A new level does not grant those operations.

## Standing grant schema and setup choices

Propose immutable `CommerceGrant` revisions with grant ID, authenticated owner identity, agency/campaign IDs, maximum level, effective/expiry times, explicit account/channel IDs, niche/categories/audience and prohibited categories. Include allowed operations/formats, accepted BrandKit revision, allowed data sources and affiliate programs, official-link forms, model provider/purpose/model allowlist, data classes/regions/retention, disclosure profile, reporting timezone/window, schedule windows, cadence, audience cap and emergency policy. Persist the owner's accepted reviewable configuration and its hash; do not store a model paraphrase as consent.

Budget fields use checked integers: per-operation and UTC-day/month model maximums in microUSD, media/provider reservations, and any separately approved channel costs in explicit currency minor units. No implicit conversion or floating-point cross-currency aggregation. Unknown/retained reservations count as spent capacity; an accounting error closes admission. Owner local-day reporting is distinct from UTC budget intervals. Commercial accounting cannot erase or bypass MeuAssistente's one cumulative USD 10 validation ledger or Zaturo's existing personal/coding reservations.

Initial setup must resolve: one niche/audience, accepted visual identity/assets, exact eligible affiliate program/account and channel, content categories/claims, publication windows/cadence, budget ceilings, approved provider/data policy, reporting preferences and exception contact path. Account linking/authentication and platform consents remain interactive owner actions. A source/credential being present does not complete setup. Proposed small-pilot ceilings are one campaign, one publisher in flight, one scheduled post per day and no direct messages; these are review suggestions, not activated defaults.

A grant can expire in at most 30 days for the initial pilot, with an explicit owner renewal workflow; choose a shorter window when platform/provider review requires it. Include platform-evidence expiry so a grant cannot outlive its eligibility evidence. Renewal creates a revision rather than resetting budgets/history. Do not ask approval for every post within Level 3; ask only when setup, scope changes or exceptions actually require it.

## Deterministic admission decisions

Policy returns `allow`, `deny` or `needs_owner_action`, each with stable reason codes and the exact revisions evaluated. Evaluate before task admission and again immediately before each external call:

1. Principal, worker and service credentials are active, scoped and unexpired; the account is the enrolled exact account.
2. Campaign/brand/grant versions match the pinned immutable manifest; no emergency flag or revocation has advanced authority.
3. The action is within level, operation, niche/category, format, channel, audience and allowed time window. A due time outside the window is skipped/deferred by the documented missed-window policy, never caught up in a burst.
4. Platform/data-source/affiliate/provider purpose eligibility is supported by unexpired evidence, and the official link/required advertising disclosure is valid for this exact distribution.
5. Facts/rights evidence are current and support every material claim; offer TTL/variant/currency/coupon/shipping conditions match the exact content. Stale facts cannot be replaced under an old approval hash.
6. Cost is reserved transactionally before paid/network admission; queue/cadence/rate/concurrency limits remain available across restart and concurrent calls.
7. Communication additionally has current purpose-specific consent, no suppression, current AI ownership and expected consent/ownership revisions at the owning endpoint. An allowlist is not itself proof of marketing consent.
8. The operation has no prior accepted/dispatching/uncertain effect that would make another attempt unsafe; idempotency and receipt recovery rules pass.

Missing evidence is deny or needs_owner_action, not a model judgment call. The model may propose a new draft or a permitted alternative, never alter a denied guard. No fallback provider or alternate account is selected to escape a refusal.

## Consent and handoff policy

Recipient consent belongs to MeuAssistente's future reviewed production registry: opaque recipient/channel handle, purpose, collection source, terms/notice version, granted/withdrawn time, expiry where applicable and audit reference. Marketing, transactional customer support and group administration are separate purposes. Keep phone numbers and message bodies off the Hub. Only the communication endpoint can evaluate current consent and suppression against its authoritative record.

Production recipients must have a clear stop/unsubscribe path, prompt suppression, no re-enrollment from message text or group membership alone, and explicit human transfer. A stop request immediately prevents new inference/send admission for that recipient and invalidates pending drafts/leases; human takeover never implicitly releases when global activation resumes. Human release is a trusted explicit CAS operation. Consent updates and restrictive stops must remain available when inference/publication quota is exhausted.

Current MeuAssistente already has deterministic pause/takeover, exact-hash contact/group block and allowed-only lists, incoming claims and outbox fencing. Its schema fixes `synthetic_only=1`, its normal runtime rejects external adapters, and its WhatsApp path is fictitious-only. It does not yet implement this production consent registry or affiliate fact tools. Do not weaken those checks to make a commercial adapter fit. Preserve the connected process, profile, filters, bindings and cumulative validation ledger.

The owner selected no Meta API. WhatsApp commercial operation remains blocked under that decision and existing experimental scope until an eligible path and a new explicit decision exist. A compliant alternative channel may be proposed; no alternative is activated by this document.

## Revocation, expiry and distributed stop semantics

The owner can revoke a grant, pause a campaign/channel, disable a credential or invoke a global commerce emergency stop. Commit the restrictive state/revision/audit atomically at the Hub, stop future task/permit admission and revoke existing worker leases. Endpoint-local stop/suppression similarly advances authority and cancels pending work inside its existing fence. Resume is explicit; it does not change expired grants, release human ownership or resend uncertain effects.

Before dispatch, the executor must confirm current Hub authority online and obtain a short-lived one-operation permit, proposed maximum 5 seconds, bound to OperationId/content/account/grant revision and the worker lease. The endpoint checks its own authority again inside the dispatch fence. A lost Hub connection denies a new permit. Persist bounded permit/dispatch evidence without secrets. A queued job cannot keep using an old permit after its expiry, account change or known revocation.

A distributed system cannot recall a platform request already admitted or prove instant remote stop during a network partition. The owner UI must distinguish `stop requested` from `all execution endpoints acknowledged stopped`. Revocation immediately denies new Hub permits; a previously admitted bounded call can finish or become uncertain before the endpoint sees revocation/permit expiry. Show in-flight operation IDs and last acknowledgment; never claim that a cancelled Hub task removed a remote post. On reconnect, reconcile every admitted operation before normal dispatch. No stop acknowledgment is inferred from silence.

Persist UTC validity and use monotonic elapsed deadlines for local admitted work. Detect material clock rollback/skew and fail closed; restarting cannot renew a permit or grant. Never perform expiry recovery by issuing a new OperationId for the same business effect. Retain revocation tombstones through credential rotation and journal recovery.

## Decisions that still require owner or external evidence

Implementation can proceed offline after a later implementation request, using conservative proposals and fixtures. Production cannot proceed until exact niche/brand/account/provider/budget grants and official platform eligibility are resolved. MeuAssistente changes remain read-only in this planning execution. Installing a production communication adapter or changing no-Meta-API policy is a separate decision, not a prerequisite to the draft MVP.

Acceptance scenarios: Level 0/1 never call a publisher; Level 2 refuses unapproved or changed content; Level 3 performs permitted fixture posts on two distinct injected days without another owner prompt but denies an extra post exceeding daily cadence; Level 4 cannot expand a grant. Test concurrent reservations, repeated requests, stale revisions, recipient withdrawal/human takeover, clock rollback, revoke-before-dispatch, partition-after-dispatch and restart after uncertain effects. Tests use local fake providers and synthetic identities; real eligibility/send acceptance remains separately observed evidence. See [test plan](10-TEST-AND-ACCEPTANCE-PLAN.md).
