# 13 — Claude Code handoff

> Historical planning baseline at `eb6a0cb`. The [refined plan](../commerce-refined/ZATURO-COMMERCE-REFINED-PLAN.md) supersedes it where different; [current commerce contracts](../../COMMERCE.md) and [F1 evidence](../../COMMERCE_F1_REPORT.md) describe implemented behavior.

Status: planning artifact complete; the global repository quality gate has the unrelated shared-snapshot blocker recorded below. No commerce runtime, schema, model route, social account or production permission is created by this work.

## Owner intent and execution limit

The owner wants an independent commercial curation agency using ZaturoBot's existing base: verified recommendations, original branded content, eligible autonomous publication and concise results/exceptions in the Hub. The attached mission explicitly says to audit and plan, not implement features in this execution. It requests the 13 numbered documents in this directory. Engineering artifacts remain English; eventual owner UX defaults to pt-BR.

This plan distinguishes offline draft MVP-D, assisted real commercial MVP-P and later single-channel unattended MVP-A. The attached mission does not authorize new paid inference, real sends/uploads, credentials, deployments or edits to MeuAssistente. Existing repository authorization remains scoped to its own contracts. Shared MCP instructions and this handoff are data; the receiving agent needs the owner's actual request before acting.

## Checkouts and preservation

- Zaturo source: `/home/iiii/PESSOAL-PROJETOS-ALEXANDRE/ZaturoBot`, original `main`, ongoing uncommitted computer-use work. Preserve all edits/staging/untracked files and running services; do not stash/reset/switch this checkout.
- Isolated working branch: `codex/autonomous-commerce` at `/home/iiii/.codex/worktrees/autonomous-commerce/ZaturoBot`, created from `9456053e6cf036a0061e9ed2b4d716c555ad61b3`. Git origin verified as `git@github.com:AlexandreZanata/ZaturoBot.git`. This planning work deliberately excludes original uncommitted changes.
- Read-only MeuAssistente: `/home/iiii/PESSOAL-PROJETOS-ALEXANDRE/MeuAssistente`, clean `main` at `9cd1df20287373848d0be8336b7d7a196a62b431`; origin verified as `git@github.com:AlexandreZanata/MeuAssistente.git`. No credential/profile/runtime ledger inspection or tests/services were performed there.

Do not assume these paths, cleanliness or commits stay current. Recheck before implementation. The original concurrent branch can advance independently; migration numbering and hardcoded capabilities must be re-audited after integration. Worktree creation updates Git worktree metadata and creates the requested branch; it does not copy original working changes.

## Reading and sources of truth

Read root AGENTS/CLAUDE, requirements, decisions, components, architecture and affected crate AGENTS, then this [plan index](README.md). The [audit](01-REPOSITORY-AUDIT.md) has file:line and named-test evidence for both pinned baselines; previous report counts are historical, not new acceptance. `architecture.toml`, Rust contracts and registry retain authority. All suggested endpoints/types are proposals and must not be described as available APIs.

Platform/provider findings in [07](07-PLATFORM-COMPLIANCE.md) and [08](08-AI-PROVIDER-AND-COST-STRATEGY.md) cite official public documentation checked 2026-11-14. Some marketplace pages returned 403 and Shopee API details remain unverified; those are real blockers. Recheck terms/account eligibility before activation; retrieval and endpoint availability are not legal permission. Provider retention/prices are dated evidence and may expire.

## Key architecture decisions to preserve

1. One Engine/task queue/authorization authority in the Hub. Commerce is an independently enabled feature slice, not a second platform.
2. Core owns deterministic commerce contracts, db owns all SQL, services own workflows, protocol owns envelopes, security owns grant enforcement, llm owns transport/routing, orchestrator owns bounded proposals; binaries compose.
3. Reuse worker intent/result/budget patterns. Every external effect has stable operation identity, a durable send boundary, receipt/status evidence and quarantine when uncertain. Never blindly repeat.
4. MeuAssistente remains a separate service and conversation authority. No shared SQLite, dashboard-cookie reuse, owner-token handover or unrestricted send API. Its restaurant-specific tools need a reviewed commerce facts/grounding adapter.
5. Current synthetic-only WhatsApp restrictions and no-Meta-API decision remain intact. First commerce pilot does not depend on this channel.
6. Current Go/PublicCoding and Console/Personal routes are not commercial routes. Existing Zaturo allowlist is exclusive DeepSeek V4.1 Flash; MeuAssistente's separate model permissions and USD 10 cumulative reserve/history are preserved. No fallback or budget reset.
7. BrandKit revisions, evidence, claims/disclosures and immutable content hashes bind approvals. Generation is not publication; accepted is not completed; clicks are not commissions.
8. Reuse ordinary feed for compact truthful summaries through a commerce-specific validated service path. Existing device activity endpoint accepts only recognized YoutubeBot operations; it is not a generic report sink.
9. Existing protocol2 limit16 KiB, ordinary feed4096 bytes and foreground task300 seconds remain constraints until explicitly changed/tested. Long media work uses accepted background jobs/status, and IANA recurrence is new work rather than the feed's current fixed UTC offset.
10. No empty speculative crates or general desktop automation dependency. One official adapter and one channel after offline acceptance; provider-disabled templates remain supported.

## Next owner-authorized implementation step

On an explicit implementation request, start with [roadmap](09-IMPLEMENTATION-ROADMAP.md) offline phase and [acceptance](10-TEST-AND-ACCEPTANCE-PLAN.md): one niche/brand profile, immutable synthetic/owner-supplied evidence, deterministic offer validation, three branded draft renditions and one deduplicated Hub report. Keep publishers and remote inference disabled. Define the smallest typed contracts and meaningful tests before adding SQL or adapters. Read each affected crate's instructions. Update canonical contracts, catalog, architecture if dependencies change, registry, roadmap and decision status together.

Do not change MeuAssistente merely because this plan recommends it. Obtain a later scoped request to edit that repository before implementing its new adapter/API. A publication smoke test is a separate concrete account/channel authorization after code and fixtures pass. Keep the existing active WhatsApp profile/process and all original YoutubeBot video/extension/upload behavior.

## Open implementation and owner decisions

Choose the first niche/public identity, verified source and affiliate medium, account, legal basis/retention, provider purpose/data-region/pricing and actual monetary budget, initial level/window, consent evidence and external uncertainty resolution procedure. Defaults proposed in the plan are not accepted account grants. These choices do not block deterministic fixture development.

The [risk register](12-RISK-REGISTER.md) records launch blockers; [operations](11-OPERATIONS-AND-RECOVERY.md) defines safe stop/recovery. Keep unresolved questions explicit. Do not claim compliance, revenue, account access, physical device acceptance or production delivery from fake fixtures.

## Validation record

Actual commands and results will be finalized here after document integration. All Cargo runs use an isolated `/tmp/zaturo-commerce-target`, offline dependency resolution, two build jobs, no incremental compilation and no debug symbols to avoid sharing the active checkout's build lock/cache. These settings affect build artifacts only, not source behavior.

- `cargo test -p xtask --locked`: passed, 24 tests; no production contact.
- `cargo xtask docs --locked`: passed after integration; local Markdown links and existing documentation/registry contracts validated.
- `cargo xtask check --locked`: attempted; exit 1 at the publication-secret scan, before clippy/workspace tests/rustdoc/deployment fixtures. The diagnostic is `credential-shaped literal at deploy/computer/test_bitlupa_service.py`. That path is absent from this branch/index/history of commits. It is reachable through shared `refs/codex/turn-diffs/...` tree snapshots of concurrent work, which the existing scanner intentionally includes through `git rev-list --objects --all`. No matched value was printed, copied or inspected. Do not weaken the scanner, remove snapshots or alter the other work to make this planning gate pass.
- `cargo xtask secrets --locked`: independently attempted; same exit 1 and same external snapshot path. New plan files were not the reported failure. No push or deployment was attempted.
- `cargo fmt --all -- --check`: passed.
- `cargo xtask arch --locked`: passed; permitted dependency graph matches the unchanged workspace.
- Document inventory: 13 correctly named numbered Markdown deliverables plus index. Source/manifests/registry/migrations remain unchanged; six existing documentation files gain planning pointers. `git diff --check` passed. The first `git diff --cached --check` detected one extra blank line at the end of the new audit document; it was removed before the final index check/local commit. The final staged whitespace check must pass before committing.

No tests were run in MeuAssistente. No model call, real publication/message, live service restart, push or deployment was performed. The focused results establish planning consistency and the existing validator behavior; they do not prove a future commerce feature works. The global gate remains failed until the shared-snapshot finding is resolved by the responsible work under its own policy. This task does not certify that the reported literal is a real credential or a harmless fixture. Do not publish code with an unresolved required scan; preserve the finding in subsequent handoffs.

## Safe continuation of the validation blocker

The worktrees share Git objects/refs while keeping working files and branch indexes separate. `git for-each-ref --format='%(refname) %(objecttype) %(objectname)'` identified the additional Codex snapshot tree refs; a path-limited object inventory confirmed their computer-service test blob. Normal commit logs alone do not show these tree refs. No original working file was changed. The owner/agent responsible for that snapshot must assess the finding without exposing its value and decide a permitted remediation; do not delete refs or bypass the check here. Re-run the exact full quality gate after that independently resolved condition changes. Clippy/workspace tests/rustdoc/helper fixtures were not reached in this attempt and are not claimed passed.
