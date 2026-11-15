# Zaturo Commerce — implementation backlog

Status: planning only; nothing below is implemented. Source of truth for scope and
rationale: [ZATURO-COMMERCE-REFINED-PLAN.md](ZATURO-COMMERCE-REFINED-PLAN.md). Evidence:
[EVIDENCE.md](EVIDENCE.md).

How to use this backlog:

- Work strictly in order inside a phase; a phase starts only when all its dependencies are
  done and verified. One agent session should finish one phase.
- Repos: **ZB** = ZaturoBot, **MA** = MeuAssistente (editing MA needs a new explicit owner
  authorization), **OWN** = owner action outside the code.
- "Done" means: acceptance criteria verified, listed tests pass, existing tests pass,
  `cargo xtask check` passes (or fails only at blocker B0, reported verbatim), and the
  docs named in the task are updated in the same change.
- Every test runs offline with fixed clocks, temporary databases and loopback fakes. No
  real tokens, accounts, providers or ledgers.
- Migration and ADR numbers below are today's next free numbers (0010, ADR-0021);
  re-check before use.

---

## F0 — Preconditions (no code)

| ID | Task | Repo | Depends | Acceptance |
|---|---|---|---|---|
| BL-001 | Commit or otherwise settle the concurrent computer-use work on `main` (schema 9, ADR-0020, registry, social helper) | ZB / OWN | — | `git status` clean on `main` except intentional untracked private files |
| BL-002 | Resolve the secrets-scan finding reachable via `refs/codex/turn-diffs/*` without weakening the scanner (owner decides if the literal is synthetic; fix source; prune snapshot refs per policy) | OWN | — | `cargo xtask secrets` passes on a clean checkout |
| BL-003 | Write ADR-0021 "Commerce feature slice" (accepting this plan's architecture) and ADR-0022 "D-01: OpenCode Go as bounded engine for commerce bot actions" (new `CommerceDraft` purpose; Go stays `PublicCoding` for everything else; terms evidence and residual risks) | ZB | BL-001 | ADRs accepted by the owner; `docs/README.md` links the refined plan; Codex plan marked superseded where it differs |
| BL-004 | Owner answers O-01…O-08 and O-13 (plan section 16) or accepts defaults in writing | OWN | — | Answers recorded in `docs/OPEN_QUESTIONS.md` commerce section |
| BL-005 | Add commerce planned entries: `config/registry.toml` feature `commerce` (planned), ROADMAP marker, COMPONENTS pointer | ZB | BL-003 | `cargo xtask docs` and contracts pass |

---

## F1 — Commerce domain core (`zaturo-core::commerce`, pure)

Files: `crates/core/src/commerce/{mod,money,offer,content,policy,claims,slots,kill_rules,attribution}.rs`,
tests in `crates/core/tests/commerce_*.rs`. No SQL, no I/O.

| ID | Task | Depends | Acceptance | Tests |
|---|---|---|---|---|
| BL-101 | `Money { minor: i64, currency: Currency }`; parse decimal strings (`"89,90"`, `"89.90"`) with checked arithmetic; supported set {BRL}; no floats | BL-005 | Overflow, negative, >2 decimals, unknown currency rejected | `money_rejects_overflow_and_float_inputs`, `money_parses_brazilian_and_dot_decimals` |
| BL-102 | `Offer` + `OfferInput` validation: program, item id, title (≤ 200 B, control chars stripped), category, price, observed_at, valid_until, official affiliate URL (HTTPS, host in program allowlist, stored verbatim), optional commission bp, sub-ID, evidence hash | BL-101 | Missing/invalid required field → typed `RejectReason` | `offer_without_official_link_or_timestamp_is_rejected`, `offer_url_with_foreign_host_is_rejected`, `offer_url_is_preserved_byte_for_byte` |
| BL-103 | `ContentState` machine: `draft, approved, scheduled, publishing, published, rejected, failed, uncertain, withdrawn` with legal transitions only; `uncertain` exits only through owner resolution | — | Illegal transition → error | `content_state_machine_rejects_illegal_transitions`, `uncertain_requires_owner_resolution` |
| BL-104 | `PolicyDocument` (serde, `deny_unknown_fields`): per-action modes, caps, windows, channels, one program per channel, categories, disclosure text, `repost_cooldown_days` (default 14), kill-rule thresholds, allowance share; `validate()`; canonical JSON + SHA-256 | — | Unknown field, out-of-range cap, empty disclosure, window overlap → error | `policy_document_rejects_unknown_fields_and_out_of_range_caps`, `policy_hash_is_canonical` |
| BL-105 | Claim/disclosure lint for rendered text: forbidden first-person test claims ("testei", "usei por", "na minha experiência"), unbacked superlatives ("o melhor", "menor preço da história"), discount % without reference-price evidence, comparative claims against other products (Mercado Livre §5.3), any number not present in the offer fields, URL other than the offer's, link of a program other than the channel's (Shopee §4.3(d)); first line must be `#publi` (CONAR 2026); body must contain at least one benefit line (no bare link, Shopee §4.4(j)) | BL-102 | Returns all violations with codes | `claim_lint_rejects_testing_claims_superlatives_and_unbacked_discounts`, `disclosure_must_precede_link_and_survive_truncation`, `post_without_publi_first_line_or_benefit_line_is_rejected`, `foreign_program_link_is_rejected` |
| BL-106 | Template renderer (pure): `render(template_id, offer, brand, now) → text` with HTML escaping for Telegram, price + timestamp only when price age ≤ cap, else "confira o preço atual"; ≤ 1500 bytes | BL-102, BL-105 | Output always passes BL-105 | `template_output_passes_lint_for_all_fixture_offers`, `stale_price_is_omitted_not_shown`, `html_escaping_prevents_markup_injection_from_offer_titles` |
| BL-107 | Slots: from policy windows + zone (`America/Cuiaba`) produce `SlotKey(campaign, local_date, index)` and `due_at` UTC; missed-window rule | BL-104 | Deterministic keys; no duplicates; missed → skip | `slots_for_cuiaba_are_stable_across_utc_midnight`, `missed_slot_is_skipped_not_caught_up_pure` |
| BL-108 | Kill rules: pure evaluation over counters (attributed clicks in N days, approved commission after N days, uncertain age, validator rejections/day, source outage) → `KillDecision` | BL-104 | Each rule fires exactly at threshold | `kill_rule_no_clicks_14_days_pauses`, `kill_rule_thresholds_are_inclusive_and_deterministic` |
| BL-109 | Attribution: map official sub-ID ↔ content id (short, URL-safe, ≤ program limit); unknown sub-ID → unattributed | BL-103 | Bijective for generated ids | `sub_id_attribution_maps_known_ids_only` |
| BL-110 | Docs: `crates/core/AGENTS.md`, `docs/COMPONENTS.md`; dependency note if a tz crate is added (`docs/DEPENDENCIES.md`, `architecture.toml` unchanged) | BL-101…109 | `cargo xtask check` | — |

---

## F2 — Persistence, intake, drafts, status

| ID | Task | Repo | Depends | Acceptance | Tests |
|---|---|---|---|---|---|
| BL-201 | Migration `0010_hub_commerce.sql` (hub stream 9 → 10): the nine tables of plan section 5, indexes, `user_version=10`; register in `config/registry.toml` and `docs/MIGRATIONS.md`; extend upgrade branches in `crates/db/src/lib.rs` | ZB | F1 | Upgrade from 9 is transactional; failed upgrade rolls back | `hub_nine_upgrades_to_ten_and_failed_upgrade_rolls_back`, existing `native_migrations` |
| BL-202 | `crates/db/src/commerce.rs` repository: CRUD with uniqueness/CAS as in section 5 | ZB | BL-201 | All SQL here only | `offer_unique_key_prevents_duplicates`, `content_cas_rejects_stale_state` |
| BL-203 | Service `import_offers(file, now)`: parse CSV/JSON (≤ 1 MiB, ≤ 500 rows), validate via core, upsert, summary with reason codes | ZB | BL-202 | Same file twice → same result, no duplicates | `offer_import_is_idempotent_and_rejects_unproven_rows` |
| BL-204 | Deterministic ranking: category allowlist, price band, evidence completeness, freshness, repost cooldown per channel, commission (disclosed attribute, never sole criterion); explain exclusions | ZB | BL-203 | Same input → same order | `ranking_is_deterministic_and_explains_exclusions`, `item_within_repost_cooldown_is_excluded` |
| BL-205 | Campaign create/activate pinning policy revision; brand file loader (`config/commerce/brand.toml`, hash) | ZB | BL-202 | Policy change creates new revision; campaign unchanged | `campaign_pins_policy_revision` |
| BL-206 | Batch builder: pick top N offers for the next K slots, render templates, store drafts with hashes | ZB | BL-204, BL-205 | Every draft passes lint | `template_draft_contains_disclosure_and_only_evidenced_facts` |
| BL-207 | Hub routes `GET /v1/commerce/status` (fields: mode+revision, stage, channel health, last 10 tasks, next 5 slots, campaign, approved/pending/unknown commission by currency, AI usage today/month vs cap, open exceptions, in-flight count at last stop) and `POST /v1/commerce/offers/import`; new app scopes; hub CLI subcommands | ZB | BL-206 | Scope enforced; no secrets/personal data in output | `status_route_requires_scope_and_hides_secrets`, `commerce_disabled_leaves_existing_routes_unchanged` |
| BL-208 | Feed projection: one deduplicated pt-BR summary per batch/import (≤ 4096 B) | ZB | BL-206 | Re-running does not duplicate | `commerce_feed_items_are_deduplicated` |
| BL-209 | Docs: ARCHITECTURE/COMPONENTS/ROADMAP/MIGRATIONS/crate AGENTS | ZB | all | `cargo xtask check` | — |

---

## F3 — Policy, kill switch, slots, approval, dispatch check

| ID | Task | Repo | Depends | Acceptance | Tests |
|---|---|---|---|---|---|
| BL-301 | Policy propose/activate routes (owner authority only); readable pt-BR rendering + hash | ZB | F2 | Only owner bearer (or `commerce_admin`) can activate | `policy_activation_requires_owner_authority` |
| BL-302 | `commerce_control` + `POST /v1/commerce/stop` (`commerce_stop`) and `/resume` (`commerce_admin`); stop cancels queued commerce tasks via Engine | ZB | BL-301 | Stop idempotent; resume keeps `uncertain`/exceptions | `stop_cancels_queued_commerce_tasks_and_denies_dispatch`, `resume_does_not_clear_uncertain_or_exceptions` |
| BL-303 | Slot admission in the Hub's existing tick loop (60 s): compute due slots, insert slot + Engine task + content `scheduled` in one transaction; skip missed | ZB | BL-302 | One task per slot key under restart and concurrent ticks | `slot_admission_is_unique_under_concurrency_and_restart`, `missed_slot_is_skipped_not_caught_up`, `onboarding_without_active_policy_never_admits_publish` |
| BL-304 | Batch approval routes (`GET …/batches/next`, `POST …/batches/{id}/approve`) bound to `(content_id, text_sha256)`; owner authority only (do **not** reuse `/aprovar`) | ZB | BL-303 | Changed text after approval cannot publish | `batch_approval_requires_owner_and_binds_hashes`, `changed_text_after_approval_is_denied_at_dispatch` |
| BL-305 | Dispatch check device route `POST /v1/device/commerce/dispatch-check` with `x-zaturo-lease` (mirror computer-use grant route); single transaction `scheduled → publishing` | ZB | BL-304 | Second call for same content → deny | `dispatch_check_allows_once_per_content`, `dispatch_check_denies_when_stopped_or_policy_changed` |
| BL-306 | `CommercePublish` service variant + capability `commerce.publish.v1` in both allowlists; worker executor with a **fake** transport; exceptions table wiring | ZB | BL-305 | End-to-end slot → fake publish → `published` | `fake_publish_end_to_end_records_receipt_once` |
| BL-307 | CLI onboarding (5 steps of plan section 6.6) | ZB | BL-301 | Produces an `active` policy only after explicit confirmation | `cli_onboarding_requires_explicit_confirmation` |
| BL-308 | Docs + SCHEDULER.md: mark commerce slots as the first scheduler slice | ZB | all | `cargo xtask check` | — |

---

## F4 — Telegram channel publisher

| ID | Task | Repo | Depends | Acceptance | Tests |
|---|---|---|---|---|---|
| BL-401 | Add `telegram_bot` transport to `deploy/computer/social_service.py` (or a sibling module importing its journal): profile `[telegram]`, token file 0600, `readiness` (`getMe`, `getChat`, `getChatMember` post rights), `execute` (one `sendMessage`, HTML, link preview on), `status` (journal only) | ZB | F3 | Postiz transport behaviour unchanged | `bot_without_post_rights_is_not_ready`, existing social helper tests |
| BL-402 | Error mapping: 400 → failed; 401/403 → channel blocked; 429 → definite rejection with `retry_after`; timeout/connection reset after send → `uncertain` | ZB | BL-401 | Matches plan J11/J12 | `telegram_error_classes_map_to_failed_uncertain_or_backoff`, `revoked_bot_token_blocks_channel_and_skips_slots` |
| BL-403 | Worker executor: dispatch check → journal intent → helper execute → receipt → completion; replay returns cached receipt | ZB | BL-402 | Crash points covered | `publish_records_intent_before_send_and_receipt_after`, `worker_crash_after_send_marks_uncertain_without_resend`, `duplicate_publish_delivery_sends_once` |
| BL-404 | Daily `getChatMemberCount` → `commerce_metric` | ZB | BL-401 | Missing value stays missing | `member_count_failure_is_unknown_not_zero` |
| BL-405 | Exception resolution for `uncertain` (owner marks `published_confirmed` / `not_published_confirmed` with note) | ZB | BL-403 | Resolution never triggers a resend by itself | `resolving_uncertain_never_resends` |
| BL-406 | **Live smoke (OWN-authorized):** private test channel, one message, receipt recorded as real evidence | OWN + ZB | BL-401…405 | Real `message_id` stored; labelled live evidence | manual record in phase report |

---

## F5 — Conversions, attribution, kill rules, weekly report

| ID | Task | Repo | Depends | Acceptance | Tests |
|---|---|---|---|---|---|
| BL-501 | Conversion import route + parser (official export columns; mapping per program in config) | ZB | F4 | Idempotent; status updates in place | `conversion_import_attributes_by_sub_id_and_is_idempotent`, `duplicate_import_counts_once` |
| BL-502 | Report revisions and reversals | ZB | BL-501 | Cancelled after approved updates totals | `late_reversal_updates_existing_conversion_and_report` |
| BL-503 | Money aggregation per currency; unknown vs zero | ZB | BL-501 | Never sums different currencies | `mixed_currencies_are_never_summed`, `unknown_is_reported_as_unknown_not_zero` |
| BL-504 | Kill-rule evaluator wired to campaign status + exceptions + slot skipping | ZB | BL-502 | Thresholds from active policy | `kill_rule_pauses_campaign_and_skips_future_slots` |
| BL-505 | Weekly pt-BR report (feed, ≤ 4096 B): posts, members, clicks/orders/commission by status, AI usage, exceptions, kill-rule state | ZB | BL-504 | Deduplicated per week | `weekly_report_fits_feed_limit_and_is_deduplicated` |

---

## F6 — D-01: OpenCode Go as bounded drafting engine

| ID | Task | Repo | Depends | Acceptance | Tests |
|---|---|---|---|---|---|
| BL-601 | `Purpose::CommerceDraft` in `zaturo-llm` with its own profile file (like `computer-models.toml`); purpose gate both ways; existing Go/Console profiles untouched | ZB | F2, BL-003 | Cross-purpose use denied with zero provider calls | `commerce_purpose_never_uses_public_coding_or_personal_profiles`, `existing_go_and_console_profiles_unchanged` |
| BL-602 | Role config (`config/llm-roles.toml` or extension of the existing config) with caps, honest user agent, session id = operation id, data class | ZB | BL-601 | Config validation rejects missing caps | `user_agent_and_session_id_are_honest_and_stable`, `request_contains_only_public_product_facts` |
| BL-603 | Prompt `prompts/commerce-draft.md` + JSON schema, registered in `prompts/registry.toml` | ZB | BL-602 | Registry contract passes | xtask contracts |
| BL-604 | Orchestrator one-call proposal path (like `conversation.rs`): ≤ 5 offers/call, ≤ 1 repair; validator rejects numbers/URLs/claims not in offer | ZB | BL-603 | Renderer still inserts price/link/disclosure from DB | `model_draft_with_invented_number_is_rejected`, `model_output_urls_are_rejected` |
| BL-605 | `CommerceDraft` service variant + capability `commerce.draft.v1`; allowance-share and daily-call caps; template fallback | ZB | BL-604 | Cap reached → templates or blocked per policy | `allowance_cap_falls_back_to_templates`, `timeout_keeps_reservation_uncertain` |
| BL-606 | "Gerar lote" owner action triggers drafting (default trigger); scheduled trigger behind policy flag (O-04) | ZB | BL-605 | Default policy never drafts without owner action | `default_policy_drafts_only_on_owner_action` |
| BL-607 | Owner-run live smoke with the existing Go key on the home worker (one call), recorded with usage | OWN + ZB | BL-606 | Real usage logged; no publication | manual record |

---

## F7 — Official affiliate API adapter (blocked until access)

| ID | Task | Repo | Depends | Acceptance | Tests |
|---|---|---|---|---|---|
| BL-701 | OWN obtains API access; record official docs, limits and terms in EVIDENCE.md | OWN | F5 | Evidence recorded | — |
| BL-702 | Adapter (Python helper or Rust transport, following the repo's pattern) for offer search, tracked link generation with sub-IDs, conversion report | ZB | BL-701 | Same validation as imports | `api_offers_map_to_same_validation_as_imports`, `tracked_links_come_only_from_official_generator` |
| BL-703 | Sync tasks (offers daily, conversions daily) with cursor + backoff + breaker | ZB | BL-702 | Outage → last valid offers without price claims | `api_outage_falls_back_to_last_valid_offers_without_price_claims` |

---

## F8 — Native owner panel and onboarding wizard

| ID | Task | Repo | Depends | Acceptance | Tests |
|---|---|---|---|---|---|
| BL-801 | Linux Slint panel: status, Parar tudo, next posts, batch approval (exact text + hash), exceptions, revenue, AI usage | ZB | F3–F5 | Uses `/v1/commerce/*` only | `stop_button_calls_stop_and_shows_in_flight_warning`, `status_shows_unknown_not_zero` |
| BL-802 | Android panel with the same features | ZB | BL-801 | Same | `approval_screen_shows_exact_text_and_hash` |
| BL-803 | Wizard for the five onboarding steps | ZB | BL-801 | Policy activation still explicit | UI test |

---

## F9 — MeuAssistente bridge (synthetic) — needs owner authorization to edit MA

| ID | Task | Repo | Depends | Acceptance | Tests |
|---|---|---|---|---|---|
| BL-901 | ZB: routes `GET /v1/commerce/offers/publishable`, `POST /v1/commerce/support-events`; scopes `commerce_facts`, `commerce_events` | ZB | F2 | No personal data accepted | `support_events_reject_personal_fields` |
| BL-902 | MA: ADR + architecture edge for a Hub client adapter; token via existing secret wrapper read only in `apps/business-assistant/src/config.rs` | MA | O-11 | `cargo xtask check` in MA | `hub_client_uses_scoped_token_and_never_db_access` |
| BL-903 | MA: `affiliate-v1` business adapter (static registration), offer cache with `valid_until`, new grounded kinds (`offer_list`, `offer_detail`, `handoff_offer`) that render price/link only from snapshots | MA | BL-902 | Restaurant vertical unchanged | `expired_offers_never_rendered`, `affiliate_kind_cannot_alter_price_or_link` |
| BL-904 | MA: metadata-only event push | MA | BL-903 | — | `events_contain_no_personal_data` |

---

## F10 — Telegram support pilot (MeuAssistente) — needs a production decision

| ID | Task | Repo | Depends | Acceptance | Tests |
|---|---|---|---|---|---|
| BL-1001 | `TelegramChannelAdapter` (official Bot API long polling; no inbound port); separate support bot token | MA | F9, O-11 | Never initiates conversations | `bot_never_initiates_conversation` |
| BL-1002 | Deterministic commands `/ofertas`, `/parar` (+ "parar", "sair"), `/humano`, `/ajuda`; first reply discloses automation and commands | MA | BL-1001 | Works with model disabled | `first_reply_discloses_automation_and_stop_command`, `stop_command_opts_out_without_model_and_blocks_replies` |
| BL-1003 | `telegram_contact` opt-out registry; never re-enrolled by a new message | MA | BL-1002 | — | `opted_out_user_is_never_re_enrolled_by_new_message` |
| BL-1004 | Handoff to owner (existing ownership CAS) + notification | MA | BL-1002 | Explicit release only | `telegram_handoff_fences_bot_replies` |
| BL-1005 | Optional model replies — only with an eligible production provider (not OpenCode: auto-responder clause), Q-001/Q-005 resolved | MA | OWN decision | Disabled by default | provider tests |

---

## F11 — Controlled scale (after ≥ 30 days of data)

| ID | Task | Repo | Depends | Acceptance | Tests |
|---|---|---|---|---|---|
| BL-1101 | Second organic surface through the existing Postiz connector (owner deploys Postiz off the VPS) or an owned static page | ZB / OWN | F5 | One surface at a time | Postiz transport tests |
| BL-1102 | Branded image cards (template, licensed assets only) | ZB | F4 | Rights record per asset | `cards_use_only_registered_assets` |
| BL-1103 | Bounded optimization: slot/template variant choice within approved ranges, minimum sample, baseline restore | ZB | F5 | Cannot change destination, disclosure or caps | `optimization_cannot_change_destination_disclosure_or_caps`, `insufficient_sample_keeps_baseline` |
| BL-1104 | Second campaign/niche | ZB | BL-1103 | Caps per campaign | existing suites |

---

## Dependency summary

```text
F0 ─► F1 ─► F2 ─► F3 ─► F4 ─► F5 ─► F7 (needs API access)
             │     └────────► F8
             ├─► F6 (needs ADR D-01; independent of F3–F5)
             └─► F9 (needs MA authorization) ─► F10 (needs production decision)
F5 + 30 days ─► F11
```
