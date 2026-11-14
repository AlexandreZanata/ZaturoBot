# Zaturo Autonomous Commerce — engineering plan

> Historical planning baseline at `eb6a0cb`. The [refined plan](../commerce-refined/ZATURO-COMMERCE-REFINED-PLAN.md) supersedes it where different; [current commerce contracts](../../COMMERCE.md) and [F1 evidence](../../COMMERCE_F1_REPORT.md) describe implemented behavior.

Planning baseline: 2026-11-14, `America/Cuiaba`. Status: **proposal only; no application behavior or production permission changes**.

The owner requested an independent commercial agency module built on ZaturoBot: curate useful offers, create a consistent visual identity and original content, operate eligible social accounts within standing grants, and report results/exceptions in the Hub. The supplied mission explicitly limits this execution to repository audit and planning. Engineering documents are English; the eventual owner interface defaults to Brazilian Portuguese.

## Executive recommendation

Keep one Zaturo control plane, durable task engine and authorization authority. Add a bounded commerce feature slice and typed worker adapters. Keep MeuAssistente as a separately authenticated communication service, with its own conversation ownership and consent checks. Never share SQLite files or introduce a second task scheduler. Reuse YoutubeBot for permitted generation; publication has a separate gate.

Start with one owner-selected niche; affordable technology is a proposed default, not an activated choice. Deliver a versioned brand kit, verified product evidence, an editorial packet, official affiliate links when eligible, and a Hub report. The first implementation milestone produces drafts with synthetic fixtures. The shortest real commercial pilot uses owner-verified evidence and assisted publication in one approved channel. Progress to unattended publication only after platform eligibility, account authorization, budget admission and uncertain-effect recovery are proved. Do not couple launch to experimental WhatsApp, TikTok eligibility, several networks or a full agent platform.

## Read in order

1. [Repository audit](01-REPOSITORY-AUDIT.md): actual code, evidence and gaps in both repositories.
2. [Product vision](02-PRODUCT-VISION.md): agency scope, brand identity, funnel and MVP.
3. [System architecture](03-SYSTEM-ARCHITECTURE.md): reuse, module boundaries and data ownership.
4. [Integration contracts](04-INTEGRATION-CONTRACTS.md): candidate typed API and recovery protocol.
5. [Agent and workflow design](05-AGENT-AND-WORKFLOW-DESIGN.md): bounded proposals versus deterministic services.
6. [Autonomy and policy matrix](06-AUTONOMY-AND-POLICY-MATRIX.md): levels, grants and revocation.
7. [Platform compliance](07-PLATFORM-COMPLIANCE.md): official evidence and production gates.
8. [AI provider and cost strategy](08-AI-PROVIDER-AND-COST-STRATEGY.md): eligibility, routing and accounting.
9. [Implementation roadmap](09-IMPLEMENTATION-ROADMAP.md): deliverable phases and rollback.
10. [Test and acceptance plan](10-TEST-AND-ACCEPTANCE-PLAN.md): observable gates and evidence boundaries.
11. [Operations and recovery](11-OPERATIONS-AND-RECOVERY.md): outage, uncertainty and emergency procedures.
12. [Risk register](12-RISK-REGISTER.md): priorities, owners and release blockers.
13. [Claude handoff](13-CLAUDE-HANDOFF.md): continuation and actual validation record.

## Isolation and authority

The isolated worktree branches from ZaturoBot commit `9456053` under `codex/autonomous-commerce`. Original `main` has concurrent uncommitted computer-use work. Those changes are excluded and must remain untouched. No reset, stash, service stop, deployment, push or account connection belongs to this work. Re-audit interfaces after that work merges; never allocate migrations by guessing the next number.

Repository rules and current typed contracts retain authority over this proposal. Suggested contracts are not live Rust types, registrations, migrations or endpoints. Open questions are implementation/activation gates, not permission to infer consent. The plan cannot promise revenue or universal autonomous platform access.
