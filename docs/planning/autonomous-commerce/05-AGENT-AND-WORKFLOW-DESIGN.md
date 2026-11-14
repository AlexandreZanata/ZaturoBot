# 05 — Agent and workflow design

> Historical planning baseline at `eb6a0cb`. The [refined plan](../commerce-refined/ZATURO-COMMERCE-REFINED-PLAN.md) supersedes it where different; [current commerce contracts](../../COMMERCE.md) and [F1 evidence](../../COMMERCE_F1_REPORT.md) describe implemented behavior.

Status: proposed. Depends on [architecture](03-SYSTEM-ARCHITECTURE.md), [contracts](04-INTEGRATION-CONTRACTS.md), [policy](06-AUTONOMY-AND-POLICY-MATRIX.md) and [eligible inference](08-AI-PROVIDER-AND-COST-STRATEGY.md).

## Minimize agents and make work auditable

Use one bounded campaign planning invocation and one content-draft invocation when they add value. The existing diagnostic planner is an architectural pattern, not a reusable commerce brain. Each proposed planner receives a versioned schema, clipped public facts and explicit constraints, and returns a proposal. A model never owns a continuous background loop, task permission, budget, customer identity or publication credential. No debating agents, recursive delegation or continuous inference is required for this scale.

Deterministic code performs ingestion, schema and evidence validation, price/availability expiry, currency arithmetic, affiliate link preservation, exclusion filters, ranking, permissions, scheduling, cost reservation, dispatch, receipt reconciliation, report imports and aggregation. Use AI for synthesis, copy variants, an editorial brief and optional alt-text proposals. Media generation is separately bounded and licensed; direct commerce account actions are never a model tool.

## Bounded proposal contracts

CampaignProposal references agency/campaign/evidence/brand revisions, one niche, at most five product candidates, at most three proposed posts, eligible channel IDs, rationale codes, expected resources and required missing evidence. It cannot carry raw authentication, new recipients or executable commands. Deterministic validation rejects an unknown entity, field or tool, a proposal outside policy, changed affiliate tag, invented claim, invalid amount, missing disclosure or unapproved destination.

ContentProposal carries fact-backed claim IDs, evidence IDs, approved voice/template revision, channel rendition, disclosure, alt text, media references and an explicit list of unverifiable claims to omit. The renderer builds the publishable artifact from accepted structured values. Model text is not a product-fact database. A second model cannot certify a price or prove a physical test. Keep internal model reasoning out of logs and Hub reports; retain only the accepted bounded proposal and validation/evidence metadata under the retention policy.

A task allows at most the explicitly reserved calls. Recommend no automatic repair in the first slice; later allow one schema repair within the original reservation if policy permits. Unknown billed outcomes stop that task. An optional local or remote provider is selected before execution and cannot become an outage fallback without approval.

## End-to-end recurring workflow

1. **Admit campaign.** Trusted owner configuration selects account, niche, brand, level, budget and allowed channel categories. Services validate that policy/terms/consent evidence exists; draft mode can continue with production gaps explicitly flagged.
2. **Gather candidates.** Import authorized owner records or call an approved bounded discovery adapter. Input documents/web pages remain untrusted data. Do not obey embedded commands or fetch arbitrary URLs supplied by a model. Keep source/license timestamps and exact product variant.
3. **Normalize facts.** Validate amounts, currency, seller/region, availability, constraints and source times. A single observation cannot support a historical discount. Reject missing or stale prices; a price-free educational draft may continue if it makes no price claim.
4. **Rank offers.** Exclude disallowed category, unsupported claim, unlicensed asset and prohibited link/channel combination. Apply a configured transparent score using evidence completeness, audience fit and price quality. Treat commission as a separately disclosed commercial attribute; unknown metrics receive no invented positive score.
5. **Plan and draft.** Use deterministic templates first. An eligible model can propose a brief/content using only accepted facts. Validate schema and all references; render against the pinned BrandKit. Produce an editorial packet with revision/hash and source metadata.
6. **Review or schedule.** Level 1 stops at draft. Level 2 requires approval bound to content/hash/account/grant revision before a proposed occurrence may dispatch. Level 3 uses a standing category/window grant without per-post approval. Missing preconditions create a compact Hub exception rather than an approval loop for every post.
7. **Preflight.** Immediately before external send, revalidate price, rights, disclosure, account identity, grant/consent revision, window, rate and budget. Changed facts invalidate the old artifact; regenerate under policy or stop. A model's final answer cannot override preflight.
8. **Journal and publish.** Persist the send boundary and operation identity, then invoke exactly one typed adapter. Capture platform receipt/request ID where available. Accepted, processing, published, failed and uncertain evidence remain distinct. A UI observation or process exit does not prove publication.
9. **Observe and report.** Status-only reconciliation settles known outcomes; unknown effects remain quarantined. Import licensed aggregate performance using provider IDs/cursors and report revisions. Deduplicate terminal notices through Engine/activity services.
10. **Optimize within bounds.** Level 4 may choose among approved copy/time variants inside the same niche, destinations and caps. It cannot increase spend, contact new people, enable a provider or change commercial promises. Freeze experiments when outcomes are missing or samples cannot support an inference.

## MeuAssistente workflow

A future commerce business adapter supplies verified snapshots through a narrow authenticated read/draft contract. The current runtime and tools are restaurant-specific; new grounded rendering is required. Incoming customer text is a request for information, never a policy update. On unknown/stale facts, return a safe unavailable/draft result or offer a human route; do not invent an offer or turn an expression of interest into a verified sale.

Consent and ownership are checked at the endpoint before model use and again before send. Human takeover or withdrawal invalidates pending drafts/dispatch epochs. Metadata-only receipts can reach the Hub. Customer bodies/contacts do not belong in aggregate reports or public-coding prompts. Commercial WhatsApp remains unavailable under current synthetic-only rules. The first integration is a fake-channel contract test, not a real customer trial.

## Experiments and factual measurement

An ExperimentSpec pins hypothesis, allowed variants, metric source, start/end window, maximum sample/cost, stopping rule and rollback baseline. First experiments compare approved copy or local publishing windows, not fabricated product superiority. Deduplicate clicks/conversions by authorized provider identities and distinguish attribution windows, returns and settlement delays. Do not compare incomplete periods as evidence of improvement; state inconclusive when data is insufficient. Owner approval is required for new channels, product classes, claims or budgets.

## Acceptance

A malicious product description requesting a send cannot create one. A draft with nonexistent evidence cannot reach dispatch. A fake model claiming success without a receipt cannot complete a publication operation. Restart preserves accepted drafts, reservations and uncertain effects. The workflow succeeds with models disabled for the first offline slice and produces one truthful ordinary Hub summary. See [test plan](10-TEST-AND-ACCEPTANCE-PLAN.md).
