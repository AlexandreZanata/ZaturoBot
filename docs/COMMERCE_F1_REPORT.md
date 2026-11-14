# Commerce F1 implementation report

Date: 2026-11-14, `America/Cuiaba`. Work branch: `codex/autonomous-commerce`.
Base integrated: main `d878998`, preserving its Personal Go routing and computer/social
contracts. Original checkout and its untracked refined planning files remain untouched.

## Scope and evidence boundary

F1 implements only the pure domain slice in [commerce](COMMERCE.md): checked money,
offer evidence/URL/expiry rules, content transitions, policy/hash rules, claim/disclosure
validation, escaped templates, timezone slot calculation, kill decisions and attribution.
It does not add SQL, task execution, account/provider calls, publication, scheduling or
MeuAssistente changes. Tests use fixed clocks and synthetic data. No real affiliate
price, enrollment, link provenance, account, public identity or revenue is verified here.

The owner explicitly requested implementation and GitHub publication. The refined plan's
instruction to wait for brand/niche/window decisions is applied to activation; F1 accepts
parameters and uses fixtures without adopting those proposals. The old snapshot scanner
finding was resolved before this work: the unchanged current-main scanner passed during
read-only refresh. A fresh full scan is still required before this branch is pushed.

## Refresh corrections

- Current main already occupies ADR-0020 (computer/social) and ADR-0021 (Personal Go).
  Commerce uses ADR-0022; it does not overwrite those decisions.
- Hub schema is 9, worker schema 8 and protocol 2. F1 changes none of them. Migration 0010
  remains unallocated until F2 verifies the current inventory.
- Personal Go is implemented and has its own opt-in, billing proof and bounded ledger.
  Historical Console HTTP402 is not a current prerequisite. Commerce inference remains
  a proposed separate F6 purpose; no provider profile/key/budget is changed.
- The social helper implements Postiz transport. Direct Telegram Bot API publishing and
  global commerce stop are later work. A transaction-only dispatch check still has a
  stop-after-admission race; future design must state its in-flight boundary honestly.
- Existing `/v1/artifacts` routes are encrypted-transfer metadata, not a generic cleartext
  upload API. F2 must design bounded intake against actual transport capabilities.

## Verification record

All final checks passed on the source in this branch:

- `cargo fmt --all -- --check`: passed.
- `cargo test -p zaturo-core --locked`: passed, **62 tests**, including **48 new commerce
  tests** across nine integration suites; the existing 14 core tests remain passing.
- `cargo clippy -p zaturo-core --all-targets --locked -- -D warnings`: passed during
  focused development; the final workspace gate rechecked current core code as well.
- `cargo xtask arch --locked` and `cargo xtask docs --locked`: passed.
- `cargo xtask check --locked`: passed, including formatting, denied-warning workspace
  Clippy, **273 Rust tests**, rustdoc, architecture/docs/registry/secret checks and
  **89 Python helper tests** (34 service + 55 computer fixtures). No ignored Rust tests.
- `cargo run -p zaturo-core --example commerce_draft --locked`: prints a synthetic pt-BR
  draft with `#publi`, affirmative affiliation disclosure, listing benefit, timestamped
  fixture price, brand label and unchanged fixture URL; the returned content also pins its brand revision. No account/provider/send.
- `cargo xtask docs --locked` after the report/backlog update: passed.
- `cargo xtask secrets --locked` with the complete staged change: passed; visible work,
  index and reachable history were scanned. Staged whitespace and scope checks passed.
- The completion message records the local commit and verified GitHub branch. A fresh
  post-commit secret scan remains mandatory before the authorized push.

Cargo used `CARGO_TARGET_DIR=/tmp/zaturo-commerce-target`, `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2` and
`CARGO_NET_OFFLINE=true`. Cached dependencies were used; no paid provider or production
account was contacted. These build settings do not alter source behavior. The new
lockfile entries are chrono-tz 0.10.4, phf 0.12.1 and phf_shared 0.12.1; existing unrelated
Apple package versions were retained after offline resolution initially proposed a
cached-version downgrade.

The first full run stopped at the unchanged desktop client's loopback fixture because
sandbox socket admission denied `127.0.0.1:0` with OS error 1 (`Operation not permitted`).
The three unchanged client tests passed with authorized local-socket execution permission,
then the entire gate passed with the same permission. No test was skipped or weakened,
no original service stopped, and no desktop source changed.

## Review and observed acceptance

Tests preceded domain modules; early integration compilation exposed a fixture helper
shadowed by a local variable, which was corrected. An independent review then produced
specific regressions for deceptive extra-slash URL authorities, stale prices inside
imported text, negative/misleading affiliation statements, percent-only discounts and
premature revenue kill rules. The revenue rule now requires known complete observations
from the actual start of automatic operation.

A final numeric-evidence regression was observed **failing before the fix**: a product
claim of “14 months of warranty” borrowed 14 from a generated date. The renderer/linter
now share an exact trusted price/observation line; date tokens are excluded from general
product-number evidence. The regression and ordinary timestamp rendering both pass.
The final 48 commerce tests include these failures, not only happy-path templates.

Fresh/stale/expired are separate: a fresh amount can render with its observation time;
a stale amount is omitted or rejected if embedded in imported text; an actually expired
offer cannot render. Unsupported claims return typed reasons. Exact link bytes survive
validation; unknown sub-IDs stay unattributed. Policy changes alter canonical hashes;
uncertain content cannot exit via ordinary transitions. Missed slots are not caught up.

## Remaining boundaries and handoff

F1 is a pure library and a local synthetic demonstration. It has no persistence, Hub API,
worker execution, active policy, account enrollment, publication or measured revenue.
Digests and host allowlists bind/validate supplied data; they do not independently verify
an official affiliate source or channel permission. Disclosure wording is deliberately
limited to two reviewed affirmative forms. Claim lint is conservative, not a general
natural-language truth verifier.

No Go/Console profile, provider model allowlist, key, billing ledger, worker capability,
wire protocol, schema, MeuAssistente or original YoutubeBot behavior was changed by F1.
No live phone/desktop/account acceptance or runtime memory target is claimed. Registry
status is `commerce:partial`; F2 and all later phases remain planned.

Next backlog item: **BL-201**, the F2 persistence phase, after rechecking current schema
versions/migration numbers. Reuse Engine and the existing ownership boundaries. Resolve
brand/niche/media/provider-budget choices before activation, not from fixture defaults.
Push only this isolated branch after the final secret scan; do not merge/deploy main or
start a real account smoke as an automatic continuation.
