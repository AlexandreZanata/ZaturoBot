# Zaturo Commerce — refined plan

Status: **planning only**. Review date 2026-11-14 (`America/Cuiaba`). Nothing in this file is
implemented, registered, deployed or authorized for production. It supersedes the Codex
plan on branch `codex/autonomous-commerce` (commit `eb6a0cb`, folder
`docs/planning/autonomous-commerce/`, 13 documents plus index) wherever the two differ.

Companion files in this folder:

- [IMPLEMENTATION-BACKLOG.md](IMPLEMENTATION-BACKLOG.md) — ordered tasks, dependencies,
  repositories, acceptance criteria and tests.
- [EVIDENCE.md](EVIDENCE.md) — what was checked, how, when, and the classification of each
  conclusion (VERIFIED / INFERRED / UNVERIFIED), including external sources.

Engineering text is English (repository rule in `AGENTS.md`); owner-facing summaries and
UI strings are Brazilian Portuguese.

---

## Resumo executivo (pt-BR)

**O que muda em relação ao plano do Codex.** O plano do Codex é tecnicamente cuidadoso,
mas foi auditado contra um commit antigo (`9456053`) e ignora o trabalho em andamento no
`main`: o ZaturoBot já tem (sem commit) um conector social com Telegram, fila de aprovação
com hash, busca de preços armazenados (BitLupa), schema do hub na versão 9 e um ADR-0020
que colide com o ADR-0020 proposto pelo Codex. O plano também é grande demais para um
único dono: cinco níveis de autonomia, concessões com ~25 campos, permissões de 5 segundos,
protocolo distribuído de parada, sete estados de "recibo de efeito" e IANA/DST para um fuso
que não tem horário de verão. E ele empurra a publicação automática e a medição de comissão
para depois de um "piloto manual", o que contraria o objetivo de pouco trabalho recorrente.

**Decisão do proprietário sobre IA (registrada aqui como D-01).** A IA **não roda
sozinha**. Quem opera é um **bot determinístico** do ZaturoBot, que guarda os arquivos,
prompts, estados e ações. A IA é só o **motor**: o bot faz chamadas pequenas e pontuais ao
**OpenCode Go** (uma chamada por ação, entrada estruturada, saída em JSON validada pelo
código) e decide sozinho o que fazer com a resposta. Nenhuma publicação, preço, link ou
decisão de gasto vem do modelo. Isso é o padrão que o proprietário descreveu ("como no
YoutubeBot"). A seção 7 registra, ao lado da decisão, o que os termos do OpenCode dizem,
para que o risco seja conhecido e mitigado — não para revogar a decisão.

**Arquitetura final.** Um único hub (VPS), um único motor de tarefas, um único roteador de
modelos. O comércio entra como uma fatia nova (`commerce`) nas camadas existentes. Publicação
no **canal próprio do Telegram** pela Bot API oficial, reaproveitando o padrão do conector
social existente (rascunho com hash → verificação → um único envio → recibo/incerteza).
Agendamento mínimo no hub (primeira fatia do agendador já planejado), com chave única por
horário. Botão de parada geral. Ofertas e comissões entram por exportação oficial do
programa de afiliados (CSV) até haver acesso à API oficial. O MeuAssistente só entra
depois, como **cliente HTTP autenticado do hub** (sem banco compartilhado, sem porta nova
aberta nele), e o WhatsApp comercial continua bloqueado.

**Menor implementação com valor comercial:** fases F1–F5 (núcleo, ofertas importadas,
rascunhos por template, agendamento com parada geral, publicador Telegram e importação de
comissões com atribuição por sub-ID). Trabalho recorrente do dono: ~15 min/semana para
importar ofertas e relatório, até a API oficial estar liberada.

**Primeira fase sem credenciais reais:** F1 (e também F2 e F3 inteiras; F4 até o teste
com servidor Telegram falso).

---

## 1. Critical diagnosis of the Codex plan

### 1.1 What the Codex plan got right (keep)

- One Hub, one Engine, one authorization authority; commerce as a feature slice, no second
  task engine, no shared SQLite between repositories (VERIFIED against both codebases).
- Deterministic code owns facts, prices, links, permissions and dispatch; models only
  propose (matches both repositories' rules).
- Uncertain external effects are quarantined and never blindly retried (matches existing
  `service_journal` and MeuAssistente outbox semantics).
- Clicks are not commissions; pending is not approved; unknown is not zero.
- No WhatsApp commercial production; no-Meta-API decision preserved; MeuAssistente's
  USD 10 validation ledger untouched; TikTok direct posting not promised.
- Mercado Livre closed-group restriction treated as blocking until clarified.

### 1.2 What is wrong, stale or too heavy (fix)

| # | Codex statement or choice | Finding | Class |
|---|---|---|---|
| C1 | Hub schema 7, worker 8; "do not reserve 0009" | `main` working tree now has `0009_hub_computer.sql` and hub schema **9**. Next number is **0010** (one shared inventory). | VERIFIED |
| C2 | Worker has a "hardcoded nine-capability set" | Capabilities are a config-checked allowlist of **13** in the worker and **16** max in the hub, duplicated in two places. | VERIFIED |
| C3 | No publisher exists; plan a Telegram/other publisher | An uncommitted **social connector** (`deploy/computer/social_service.py`, [SOCIAL_MANAGEMENT](../../SOCIAL_MANAGEMENT.md)) already implements prepare → hash → approve → single POST → receipt/uncertainty for Postiz, including a `telegram` integration. Building a separate poster would duplicate it. | VERIFIED |
| C4 | Price intelligence is new work | `PriceSearch` / `price.search.v1` + BitLupa read-only stored-offer search exist (no collector, BRL, 6 h freshness). | VERIFIED |
| C5 | Adds "ADR-0020 commerce" | `main` already has **ADR-0020: semantic computer use and social drafts**. Commerce needs the next free number at implementation time (today 0021). | VERIFIED |
| C6 | Feed day query is fixed UTC-4 | Offset is client-selectable (−720..840, default −240). | VERIFIED |
| C7 | Five autonomy levels + `CommerceGrant` (~25 fields) + 5-second dispatch permits + multi-executor stop acknowledgements + 7-state effect receipts | Correct in spirit, oversized for one owner, one campaign, one publisher. Replaced by one policy document with four modes per action, one transactional dispatch check, and the existing content/journal states (sections 5–6). | INFERRED |
| C8 | IANA recurrence with DST gap/fold handling | `America/Cuiaba` has no DST. Use daily slots in a configured zone via a maintained tz library; no custom DST logic. | VERIFIED (tz rules) / INFERRED (scope) |
| C9 | Price freshness ≤ 15 min before dispatch | Unachievable without an official API; would either block every post or invite fake compliance. Replaced by a configurable price-age cap (default 6 h, matching BitLupa) and a timestamp in every post; stale price → post without price. | INFERRED |
| C10 | MVP-P: the owner publishes manually and records the post reference | Contradicts "minimal recurring work". Telegram Bot API posting to an owned channel is official and low-risk, so automatic publication comes right after the offline core (F4), with batch approval first. | INFERRED |
| C11 | Commission import after the publisher and after inference (C8) | Kill rules and "optimize by approved commission" need attribution from day one. Conversion import with official sub-IDs moves to F5, before AI and before the affiliate API. | INFERRED |
| C12 | MeuAssistente exposes `/commerce/v1/drafts`, `/communication-intents`, `/authority`, `/suppress`, `/handoff`; Zaturo exposes offers/reports; 8 service principals | Two servers and two credential sets for a pilot that does not need MeuAssistente. MeuAssistente has **no** service auth and only a loopback human dashboard. Refined: MeuAssistente becomes an **outbound app client of the existing Hub API** (pull offers, push metadata events). No new listener in MeuAssistente. | VERIFIED (facts) / INFERRED (choice) |
| C13 | "Reuse YoutubeBot for permitted generation" | YoutubeBot generates text by driving chatgpt.com with Playwright and uploads by driving YouTube Studio in a browser. Neither is an official API path; neither is used for commerce autonomy. | VERIFIED |
| C14 | Go/Console are "not commercial routes" pending review | Confirmed with the current OpenCode Terms (2026-11-08). The **owner has now decided** to use OpenCode Go as the engine for small bot calls (D-01). The plan records the decision and the terms side by side (section 7). | VERIFIED (terms) |
| C15 | Global stop exists as a proposal | There is **no** global pause/kill switch today; only per-task pause/resume/cancel and local STOP files. Must be built (F3). | VERIFIED |
| C16 | Approval flows can be reused | The existing `/aprovar UUID` path (`crates/services/src/computer.rs`, `approve`) has **no actor check**: any app token with `can_submit` can approve. Commerce approvals must require owner authority. | VERIFIED |
| C17 | No economics, no program choice, no kill numbers | Plan had no unit economics, no concrete first program and no stop rule. Section 14 adds a single experiment with deterministic kill rules. | VERIFIED (absence) |
| C18 | Volume: ~1,000 dense lines across 13 files with heavy repetition | A next agent cannot pick "one phase" from it. This plan is one file plus a backlog. | INFERRED |

### 1.3 New risks the Codex plan did not see

- **R-N1 (MeuAssistente, pre-existing).** The synthetic WhatsApp mode defaults to
  `BLOCK_SELECTED` (answers every chat not explicitly blocked), the code cannot verify that
  the linked number is fictitious, and ADR-0032 recorded 111 contacts and 27 groups on the
  linked account. If automation is activated without `ALLOW_SELECTED`, real third parties
  could receive automated replies. This plan changes nothing there (scope restriction), but
  the owner should switch to allow-only with the test chats before any activation (O-12).
- **R-N2 (gate).** `cargo xtask check` / `cargo xtask secrets` stop at a credential-shaped
  literal reachable only through `refs/codex/turn-diffs/*` snapshot refs (four exist today).
  Every implementing agent will hit this until the owner resolves it (blocker B0).
- **R-N3 (provider).** OpenCode Console's last live check returned HTTP 402, and the
  DeepSeek zero-retention arrangement on OpenCode expires 2026-11-30.
- **R-N4 (owner Go allowance).** The OpenCode Go allowance is shared with the owner's own
  coding use and ZaturoBot diagnostics; commerce must have its own small cap so it never
  exhausts the coding allowance.
- **R-N5 (concurrent edits).** Another session is editing `main` (docs, registry, ADRs)
  right now. Implementation must start only after that work is committed.

Full evidence with file references: [EVIDENCE.md](EVIDENCE.md).

---

## 2. Architectural changes proposed (Codex → refined)

| Area | Codex | Refined decision | Why |
|---|---|---|---|
| Publisher | New official adapter per channel; Telegram candidate after grants/scheduler/budget | Reuse the existing social-connector pattern and journal; add a `telegram_bot` transport (direct Bot API) next to the Postiz transport | Avoid duplicating prepare/hash/journal/uncertainty; Postiz is not deployed and is heavy (AGPL app + its own stack) for one channel |
| Publish trigger | Model-planned ComputerTask (today's only route to social) | Typed `CommercePublish` service request, no model in the publication path | Matches D-01 (AI is only an engine) and avoids the Console planner (HTTP 402) |
| Scheduler | Bounded Hub loop with IANA/DST machinery | First slice of the already planned scheduler (`docs/SCHEDULER.md`): fixed daily slots, unique slot key, skip-missed, inside the Hub's existing tick loop | Smallest thing that is restart-safe; no new process |
| Authority | 5 levels + grant documents + permits + stop acks | One versioned policy (modes `off/draft/approve/auto` per action + caps) + global `commerce_control` + one transactional dispatch check | Same safety properties, far less code |
| Dispatch check transport | Protocol extension / permits | Device HTTP route with lease header, following the computer-use precedent (`/v1/device/computer-*`) | No protocol version change |
| Price freshness | ≤ 15 min | Price-age cap (default 6 h), timestamp in post, price omitted when stale | Achievable and honest |
| Evidence objects | `EvidenceRef`, `ProductSnapshot`, `OfferSnapshot`, `BrandKitManifest`, `ContentManifest` | One `commerce_offer` row with evidence hash + one `commerce_content` row; brand is a versioned file | Fewer types, same traceability |
| Attribution | Not designed | Official sub-ID / tracking parameters generated by the program's own link tool (never hand-edited links) | Only way to measure "attributed sales" |
| Metrics order | After publisher and inference | F5, right after publisher | Kill rules need it |
| MeuAssistente | Bidirectional `/commerce/v1` | MeuAssistente is an outbound Hub app client (pull offers, push metadata events) | No new listener; reuses Hub app credentials |
| Support pilot | Deferred indefinitely (WhatsApp blocked) | Telegram support bot (user-initiated, `/parar`, `/humano`), deterministic first, in MeuAssistente | Official channel, consent by construction |
| Inference | New commerce purpose on an approved commercial provider | Owner decision D-01: OpenCode Go as bounded engine for drafting; templates always available; provider swap by config | Owner directive; terms risk recorded and mitigated |
| Images/video | BrandKit with renditions; YoutubeBot media | Telegram link preview of the official product page in the pilot; branded cards later; no video in pilot | Zero rights risk, zero rendering cost |

---

## 3. Final architecture

### 3.1 Where things run (existing topology, VERIFIED)

- **Hub**: Oracle VPS, 1 GiB RAM, systemd `MemoryHigh 150M / MemoryMax 300M`, bound to
  `127.0.0.1:7878`; PCs reach it through outbound SSH tunnels. Must not run models,
  browsers, Postiz or Docker.
- **Home worker** (notebook/home PC, 22 GB RAM): holds the provider key, runs model calls
  and Python helpers.
- **Work-PC worker**: executes computer-use steps.

### 3.2 Components

```text
                    Owner (Android / Linux desktop / CLI)
                         │  existing app/owner tokens
                         ▼
┌──────────────────────── Hub (VPS) ─────────────────────────────┐
│ Engine (tasks, leases)   commerce services (new)               │
│  ├ slot admission (60 s tick, unique slot key)                 │
│  ├ policy + commerce_control (kill switch)                     │
│  ├ offers / content / conversions / exceptions (SQLite, 0010+) │
│  ├ dispatch check (device route, one transaction)              │
│  └ feed projection (existing FeedItem ≤ 4096 B)                │
└───────────────▲───────────────────────────────▲────────────────┘
                │ WebSocket v2 + device HTTP     │ app token (F9+)
                │                                │
     Home worker (notebook)              MeuAssistente (F9/F10)
      ├ CommerceDraft  → zaturo-llm → OpenCode Go (D-01, bounded)
      ├ CommercePublish → social helper, transport telegram_bot
      │                    → Telegram Bot API (owned channel)
      └ (F7) affiliate API sync / conversion sync
```

### 3.3 Ownership (who decides what)

| Concern | Owner component | Never owned by |
|---|---|---|
| Task ids, leases, retries, cancellation | Existing Engine | Any commerce code path |
| Policy, modes, caps, kill switch | `zaturo-services::commerce` + `zaturo-core::commerce::policy` | Models, MeuAssistente, Telegram users |
| Offer facts, prices, links | `commerce_offer` rows from owner import or official API | Models |
| Rendered text | Template renderer (+ model headline/benefits in F6) validated by core | Models alone |
| Send and receipt | Home worker + social helper journal | Hub (no outbound calls), models |
| Conversation, consent, opt-out, handoff (F10) | MeuAssistente | Hub |
| AI provider choice | `config/llm-roles.toml` + owner ADR | Application code |

### 3.4 Integration decision: MCP vs local IPC vs authenticated HTTP

| Option | Fit | Verdict |
|---|---|---|
| MCP | Designed for model-facing tools. MeuAssistente's business MCP is stdio/in-process and restaurant-specific; ZaturoBot's MCP is an inert shared-notes bridge by policy. Using MCP between services would put a model-shaped protocol on a service boundary and couple process lifecycles. | Rejected for service integration. Keep MCP for model tools only. |
| Local IPC (Unix socket) | Only works when both run on the same host; the Hub is on the VPS. | Rejected. |
| **Authenticated HTTP to the existing Hub API** | The Hub already authenticates app clients (digest registry, up to 8 apps, scopes), already sits behind the SSH tunnel, already bounds bodies/concurrency. MeuAssistente can call it outbound like the native clients do. | **Chosen.** New narrow scopes; versioned `/v1/commerce/*` routes; no listener added to MeuAssistente. |

---

## 4. Integration contracts

All shapes below are proposals. Rust types in `zaturo-core`/`zaturo-protocol` will be the
authority once implemented (repository rule). JSON field names are `snake_case`; money is
integer minor units plus ISO currency; times are UTC milliseconds or RFC 3339 `Z`.

### 4.1 Common rules (all new routes)

- Auth: owner bearer, or app token with an explicit new scope. Scopes:
  `commerce_view` (read status/feed), `commerce_stop` (restrictive actions only),
  `commerce_admin` (approve batches, change policy, resume, resolve exceptions — owner
  bearer only in the pilot), `commerce_facts` (read publishable offers; MeuAssistente),
  `commerce_events` (append metadata-only support events; MeuAssistente).
- Mutations require `Idempotency-Key` (1–128 ASCII). Same key + same body hash → same
  result; same key + different body → `409 idempotency_conflict`.
- Errors: `{ "code": "<stable_snake_case>", "message": "<safe pt-BR text>",
  "retry": "never" | "after_seconds" | "reconcile" }`. No provider bodies, tokens or
  personal data in errors.
- Body limit: existing 16 KiB default; offer import uses the existing artifact/upload path
  when larger (bounded to 1 MiB, 500 rows).
- Pagination: `limit` 1..50, opaque `cursor`.

### 4.2 Owner → Hub

| Method & path | Scope | Purpose |
|---|---|---|
| `GET /v1/commerce/status` | `commerce_view` | Dashboard snapshot (fields listed in backlog item BL-207): mode, channel health, last tasks, next slots, active campaign, verified commission, AI usage, open exceptions. |
| `POST /v1/commerce/stop` | `commerce_stop` | Kill switch. Sets `mode=stopped`, increments revision, cancels queued commerce tasks via Engine. Idempotent. |
| `POST /v1/commerce/resume` | `commerce_admin` | `stopped/paused → running`; requires an `active` policy; does not clear `uncertain` or exceptions. |
| `POST /v1/commerce/policy` | `commerce_admin` | Propose a policy document (validated). Returns `revision`, `sha256`, diff summary. |
| `POST /v1/commerce/policy/{rev}/activate` | `commerce_admin` | Activate after owner review; previous becomes `superseded`. |
| `POST /v1/commerce/offers/import` | `commerce_admin` | Import an official export (CSV/JSON). Returns accepted/rejected counts with reason codes. |
| `GET /v1/commerce/batches/next` | `commerce_view` | The next batch of drafts with exact texts and hashes. |
| `POST /v1/commerce/batches/{id}/approve` | `commerce_admin` | Approve exactly the listed `(content_id, text_sha256)` pairs. |
| `POST /v1/commerce/conversions/import` | `commerce_admin` | Import an official commission/conversion report. |
| `GET /v1/commerce/exceptions` / `POST …/{id}/resolve` | view / admin | List and resolve with a resolution code (`uncertain` sends need `published_confirmed` or `not_published_confirmed` + evidence note). |

### 4.3 Hub ↔ worker

New `ServiceRequest` variants (core), capability-gated, added to **both** allowlists
(`crates/services/src/lib.rs` and `apps/zaturo-worker/src/config.rs`):

```text
CommerceDraft   { batch_id, offer_ids: [≤5], role: "commerce.draft", policy_revision }
                capability commerce.draft.v1, side_effecting = false (paid-call fenced by budget)
CommercePublish { content_id, channel_id, text (≤1500 B), text_sha256,
                  link_preview_url, policy_revision, control_revision }
                capability commerce.publish.v1, side_effecting = true
```

Result summaries stay ≤ 2048 bytes like `PriceSearch`/`Computer`. Draft results carry
structured JSON (headline, ≤3 benefit lines, per offer) ≤ 16 KiB.

Dispatch check (device HTTP route, `x-zaturo-lease` header, mirrors
`/v1/device/computer-actions/{id}/grant`):

```text
POST /v1/device/commerce/dispatch-check
{ "task_id": "...", "content_id": "...", "text_sha256": "...",
  "policy_revision": 7, "control_revision": 12 }
→ 200 { "decision": "allow", "check_id": "..." }
→ 200 { "decision": "deny", "reason": "stopped|policy_changed|content_changed|offer_stale|
        channel_blocked|daily_cap|not_scheduled|lease_invalid" }
```

`allow` atomically moves the content from `scheduled` to `publishing`. Worker completion
reports `{ "state": "published", "message_id": 123, "chat_id": "@…", "sent_at": … }`,
`{ "state": "failed", "code": "telegram_400_…" }` or `{ "state": "uncertain" }`.

### 4.4 Worker → Telegram (inside the social helper, `telegram_bot` transport)

- Profile adds `[telegram] bot_token_file`, `channel_id`, `allow_execute`.
- `readiness`: `getMe` + `getChat(channel_id)` + bot admin rights check
  (`getChatMember(channel, bot_id).can_post_messages`).
- `execute`: exactly one `sendMessage` with `parse_mode=HTML` (escaped), link preview
  enabled for the official product URL, `disable_notification` per policy. No retry on
  timeout. 429 → `retry_after` honoured by **rescheduling inside the same slot window**
  only if no request reached Telegram's acceptance (HTTP 429 is a definite rejection).
- `status`: there is no Bot API lookup by our operation id; an intent without a stored
  receipt stays `uncertain` until the owner resolves it.
- Metrics: `getChatMemberCount` once per day → `commerce_metric(members)`.

### 4.5 Affiliate program inputs

- **Owner import (F2)**: CSV/JSON exported from the program's official dashboard/tools.
  Required columns: `program`, `item_id`, `title`, `price` (decimal string), `currency`,
  `observed_at`, `affiliate_url`, `category`; optional `commission_rate`, `sub_id`,
  `valid_until`, `image_url`. Unknown columns ignored; missing required → row rejected.
- **Official API (F7)**: only after the owner obtains access; adapter maps the official
  fields to the same row shape. The program's own short-link generator with sub-IDs is the
  only allowed way to create tracked links.
- **Conversion report (F5)**: program export or API rows with `conversion_id`, `item_id`,
  `sub_id`, `status`, `commission`, `currency`, `order_time`.

### 4.6 MeuAssistente ↔ Hub (F9/F10)

| Method & path | Scope | Notes |
|---|---|---|
| `GET /v1/commerce/offers/publishable?cursor=` | `commerce_facts` | Returns ≤ 50 `{offer_id, revision, title, price_minor, currency, observed_at, valid_until, affiliate_url, disclosure_text}`. Expired offers never returned. |
| `POST /v1/commerce/support-events` | `commerce_events` | `{event_id, kind: "answered"\|"handoff"\|"opt_out"\|"unknown_fact", offer_id?, at}`. No user ids, names, phone numbers or message text. Idempotent on `event_id`. |

MeuAssistente stores its Hub token through its existing secret wrapper; only
`apps/business-assistant/src/config.rs` reads it (its rule). No Hub database access, no
dashboard cookie reuse, no owner token.

---

## 5. Minimal data model

Nine Hub tables in one additive migration (number allocated at implementation time;
**0010** today, hub stream 9 → 10), no new worker table, and two MeuAssistente tables only
in F10. All money is `i64` minor units plus ISO currency (`BRL`, exponent 2). All times are
UTC milliseconds.

| Table | Purpose | Key columns | Invariants |
|---|---|---|---|
| `commerce_control` | Single row: global mode / kill switch | `mode` (`setup`/`paused`/`running`/`stopped`), `revision`, `changed_at`, `reason_code` | Every admission and dispatch check reads `mode` and `revision`. Stop increments `revision`. |
| `commerce_policy` | Owner-approved policy revisions | `revision`, `state` (`proposed`/`active`/`superseded`), `document_json`, `sha256`, `approved_at` | Exactly one `active`. Validated by `zaturo-core::commerce::policy`. No model path writes here. |
| `commerce_campaign` | One experiment | `id`, `status` (`draft`/`active`/`paused`/`ended`), `policy_revision`, `slots_json`, `kill_rule_json`, `starts_at`, `ends_at`, `pause_reason` | Pins a policy revision. Kill-rule pauses are deterministic. |
| `commerce_offer` | Verified offer | `id`, `program`, `external_item_id`, `title`, `category`, `price_minor`, `currency`, `price_observed_at`, `valid_until`, `commission_bp`, `affiliate_url`, `sub_id`, `source` (`owner_import`/`api`), `evidence_sha256`, `status` (`candidate`/`active`/`stale`/`rejected`), `reason_code` | `UNIQUE(program, external_item_id, evidence_sha256)`. Link stored byte-for-byte. |
| `commerce_content` | One publishable unit | `id`, `campaign_id`, `offer_id`, `batch_id`, `channel_id`, `template_id`, `brand_sha256`, `text`, `text_sha256`, `generated_by` (`template`/`model:<id>`), `state`, `slot_key`, `task_id`, `operation_id`, `external_ref`, `published_at`, `error_code` | States: `draft → approved → scheduled → publishing → published`; exits `rejected`, `failed`, `uncertain`, `withdrawn`. `uncertain` only leaves by owner resolution. |
| `commerce_slot` | Editorial occurrence | `slot_key` (`campaign/local_date/index`), `due_at`, `state` (`pending`/`admitted`/`skipped`/`done`), `skip_reason`, `content_id` | `UNIQUE(slot_key)`; missed → `skipped(missed_window)`; never caught up in bursts. |
| `commerce_conversion` | Imported conversions | `program`, `external_id`, `sub_id`, `content_id`, `status` (`pending`/`approved`/`cancelled`/`unknown`), `commission_minor`, `currency`, `ordered_at`, `report_revision`, `imported_at` | `UNIQUE(program, external_id)`; re-import updates; unattributed keeps `content_id NULL`. |
| `commerce_metric` | Channel observations | `channel_id`, `metric`, `value`, `observed_at`, `source` | Only official values. Missing ≠ 0. |
| `commerce_exception` | Owner-visible problems | `id`, `dedupe_key`, `kind`, `severity`, `scope`, `opened_at`, `resolved_at`, `resolution_code` | One open row per `dedupe_key`; each open projects one feed item. |

- **Brand**: versioned file `config/commerce/brand.toml` (name, handle, voice rules,
  forbidden words, disclosure text, emoji policy). Its SHA-256 is stored per content row.
- **Worker side**: the social helper's private JSON receipt journal (flock, fsync, 500
  ops / 8 MiB) plus the existing `service_journal` (`Execute` / `Cached` / `Reconcile`).
- **AI usage**: existing token ledgers plus a `commerce` profile; a monetary/allowance
  column only if F6 needs one (section 7).
- **MeuAssistente (F10)**: `telegram_contact(sha256(bot_id,user_id), state
  active|opted_out|blocked, first_contact_at, opted_out_at)` and a read-only
  `commerce_offer_cache` with `valid_until`. Ownership/outbox/handoff reuse its tables.

---

## 6. Autonomy policies

### 6.1 One policy document, four modes per action

- One owner-approved, versioned, hashed policy; exactly one `active`.
- Each action has a mode: `off` → `draft` (prepare only) → `approve` (prepare a **batch**,
  one owner action approves it, bound to text hashes) → `auto` (execute within caps).
- Caps live next to modes: posts/day, AI calls/day and allowance share, offers per sync,
  max price age, categories, windows, channels.
- Global `commerce_control.mode`: `setup`, `paused`, `running`, `stopped` (kill switch;
  cancels queued commerce tasks through the existing Engine cancel path, which revokes
  leases).

Mapping from the Codex levels: L0 = all `off/draft`; L1 = content `draft`; L2 = publish
`approve`; L3 = publish `auto`; L4 = optimize `auto`.

### 6.2 Action matrix

| Action | Allowed modes | Pilot default | Always needs an explicit owner decision |
|---|---|---|---|
| Import offers from an official export | `auto` | `auto` | New program |
| Sync offers from official affiliate API | `off`, `auto` | `off` | New API credential |
| Rank/select offers (deterministic) | `auto` | `auto` | Changing excluded categories |
| Draft from template | `draft`, `auto` | `auto` | — |
| Draft headline/benefits with OpenCode Go (D-01) | `off`, `draft`, `auto` | `off` until F6 | Raising the allowance share; new provider/model |
| Schedule into slots | `auto` | `auto` | New windows; more posts/day |
| Publish to the owned Telegram channel | `off`, `draft`, `approve`, `auto` | `approve` (Stage B), then owner may set `auto` | New channel/account; higher daily cap |
| Publish to any other surface | `off`, `draft`, `approve`, `auto` | `off` | Every new surface |
| Reply to a user who wrote to the support bot | `off`, `auto` | `off` until F10 | Model-written replies to the public |
| Initiate a message to anyone | — | never | Not offered by the system |
| Import conversions | `auto` | `auto` | — |
| Pause campaign on a kill rule | `auto` | `auto` | — |
| Resume a kill-rule pause | `approve` | `approve` | — |
| Optimize slot/template choice | `off`, `auto` | `off` until F11 | Anything outside pre-approved ranges |
| Delete/edit a published post | `approve` (per item) | `approve` | Always per item |
| Spend money, buy ads, accept terms, create accounts | — | never automated | Owner acts outside the system |

### 6.3 Progressive autonomy

1. **Stage A — draft only (≥ 3 days).** Owner checks templates, disclosure and links.
2. **Stage B — weekly batch approval (≥ 14 days).** One approval per batch. Proposed exit:
   zero validator failures reaching `publishing`, zero unresolved `uncertain`, zero owner
   rejections in the last 7 days. Owner activates a policy with publish `auto`.
3. **Stage C — auto within caps.** Owner involvement: weekly report and exceptions only.
4. **Stage D — bounded optimization (F11).** Only after ≥ 30 days of attributed data.

Automatic demotion to `approve` with an exception: any `uncertain` send, channel block,
content invalid at dispatch check, or three validator rejections in one day.

### 6.4 Always blocked until an explicit decision

Deleting posts; new channels, accounts, programs or providers; spending above caps;
system-initiated messages; changing the disclosure text; resolving an `uncertain` send;
promoting anything experimental (including MeuAssistente's synthetic WhatsApp mode) to
production; any compliance exception.

### 6.5 Dispatch check (replaces 5-second permits and stop acknowledgements)

Immediately before the Telegram call, the worker calls the dispatch check (section 4.3).
In one SQLite transaction the Hub verifies: lease current, `mode = running` with the
pinned `control_revision`, policy revision still `active`, text hash equals the approved
hash, offer not expired and price age within cap, channel not `blocked`, daily cap not
reached, content `scheduled`. Success moves the content to `publishing` and returns
`allow`; anything else returns `deny(reason)` and no send happens. A second worker or a
re-delivered task cannot get a second `allow`. Hub unreachable → no send (fail closed).

Residual risk, stated to the owner: one HTTP call already in flight when a stop arrives;
without a stored receipt it becomes `uncertain`.

### 6.6 Owner experience

**Onboarding (five steps, CLI in F3, native wizard in F8; pt-BR):**

1. **Conectar contas elegíveis** — paste the Telegram channel id; the system runs
   readiness (bot is admin with post rights) and shows ✅/❌. Affiliate program: record
   program, account label and approved media (owner attests what the dashboard shows;
   stored as evidence with date). No credential ever passes through a model.
2. **Escolher negócio e preferências** — brand file (name, voice, forbidden words),
   niche/categories, price band, posting windows. Defaults from section 14.1.
3. **Definir limites** — posts/day, AI allowance share and calls/day, max price age,
   kill-rule thresholds.
4. **Revisar e autorizar políticas** — the system renders the whole policy as readable
   pt-BR text plus its hash; the owner activates it (owner authority only).
5. **Ativar** — `mode: paused → running`. Stage A starts (drafts only).

**Dashboard (`GET /v1/commerce/status`, then native panel):** operation
`running/paused/stopped` with one big **Parar tudo** button; channel health; last 10
commerce tasks with result; next 5 slots with their content; active campaign and stage;
**verified** commission (approved) separated from pending and unknown; AI usage
today/month vs cap; open exceptions with one-tap resolution where safe; in-flight send
count at the time of the last stop.

**Recurring owner work in Stage C:** read the weekly report, import the weekly offer
export and conversion report until F7, resolve rare exceptions (`uncertain` sends,
channel problems). Target: ≤ 15 minutes per week.

---

## 7. Permitted inference strategy

### 7.1 Owner decision D-01 (2026-11-14): OpenCode Go is the engine, the bot is the operator

The owner stated: OpenCode will be used for **small calls for the bot's actions**; the AI
will **not** run on its own; it runs **through a bot** that holds all files and actions;
the AI is only the **engine**. In this plan that means:

1. **The bot is deterministic and owns everything**: files, prompt templates (registered in
   `prompts/registry.toml`), schedules, offers, policy, approvals, publication, metrics.
2. **The AI is called only inside a bot action**, never in a loop, never with tools, never
   with authority. One action = one bounded request (`max_calls_per_task = 1`, plus at most
   one schema repair within the same reservation).
3. **Input is structured and minimal** (offer title, category, price fields, brand voice
   rules). No customer data, no Hub history, no credentials.
4. **Output is a JSON proposal** (headline and up to three benefit lines). Code validates
   it; price, link, disclosure and timestamp are inserted by the renderer from the
   database, never from model text.
5. **The AI never publishes, never answers end users, never decides spending.** If the
   call fails, is rejected or the allowance cap is hit, the bot uses the deterministic
   template (or blocks, per policy) — the business keeps running without the AI.
6. **Drafting is triggered by the bot**: by default when the owner presses "Gerar lote"
   (Stage A/B), and on the bot's own schedule only after the owner sets drafting to `auto`
   (Stage C).

Clarification about the reference project (VERIFIED on GitHub, default branch,
2026-11-14): the public YoutubeBot code generates articles by driving chatgpt.com with
Playwright and uploads through YouTube Studio pages; OpenCode Go appears there only in a
test fixture that strips `OPENCODE_GO_API_KEY` from a subprocess environment. The
**pattern** the owner describes (deterministic bot, AI as a pluggable engine) is the one
this plan adopts; the browser-driven parts of YoutubeBot are not reused for commerce.

### 7.2 What the OpenCode terms say (recorded next to D-01, not overriding it)

Checked 2026-11-14 on the official pages (details in [EVIDENCE.md](EVIDENCE.md)):

- The Terms of Service (effective 2026-11-08) cover "the website, the inference product
  and the hosted software", so they apply to Go and to the Console API.
- Use is limited to the customer's "own internal use, and not on behalf of or for the
  benefit of any third party".
- The restrictions list includes running "any form of auto-responder" and "any processes
  that run or are activated while you are not logged into the Services", and forbids
  representing output as human-generated.
- The Go documentation says Go is designed for OpenCode and other coding agents, asks
  clients to send typical coding-agent traffic with a stable `x-opencode-session` and an
  identifying user agent, says Go traffic is monitored for abuse, and that limits may
  change.
- Nothing on those pages explicitly allows or forbids marketing drafting (VERIFIED absence).

How the D-01 design stays inside the plainest reading of those clauses, and what remains:

| Clause | Design response | Residual risk |
|---|---|---|
| Internal use only | AI drafts copy for the owner's own business; published text is assembled by the bot | Low–medium (interpretation) |
| No auto-responder | Go is **never** used to answer users (F10 support is deterministic; model replies need a separately eligible provider) | Low |
| No processes while logged out | Default trigger is the owner's "Gerar lote" action; scheduled drafting only after the owner opts in (O-04) | Medium if scheduled drafting is enabled |
| Not human-generated | Pinned channel note says posts are prepared with automation; no "I tested" claims | Low |
| Coding-agent traffic expectation | Honest user agent (`zaturo-commerce-bot/<version>`), stable session per operation, very low volume; **never** disguise as a coding agent (MeuAssistente rule: never impersonate a client) | Medium: OpenCode may throttle or suspend non-coding traffic at its discretion |

Mitigation for every residual risk: templates keep the business running if Go is
unavailable, and the provider is swappable by configuration (section 7.4).

Repository consequences: ZaturoBot ADR-0010/ADR-0019 keep Go as `PublicCoding` only, so
F0 must record D-01 as a new ADR that adds a separate `CommerceDraft` purpose on the Go
route with its own caps. MeuAssistente is **not** changed by D-01: its Go use stays
synthetic-only, and customer-facing inference there remains blocked (Q-001).

### 7.3 Allowance and cost control

- Go is a flat subscription with a monthly allowance and 5-hour/weekly windows, shared with
  the owner's coding use and ZaturoBot diagnostics. Commerce gets its own cap expressed as
  a share of the allowance (proposal: ≤ 10 % of the monthly allowance, ≤ 20 calls/day),
  computed from tokens × the published Go rates for the pinned model and recorded per call.
- Existing token ledgers are reused (global day/month admission already sums
  `model_calls` and `assistant_calls`); commerce adds its own profile and per-day call cap.
  MeuAssistente's USD 10 ledger is untouched.
- No Extra Usage, no automatic fallback model, no second account.

### 7.4 One adapter, configured per role (provider swap without code changes)

Extend `zaturo-llm` (no second client). F6 turns today's fixed allowlist into a small role
registry:

```toml
# config/llm-roles.toml (proposed, F6) — no secrets here
[[provider]]
id = "opencode-go"                 # existing route; D-01
endpoint = "https://opencode.ai/inference/go/openai/v1/chat/completions"
secret_ref = "worker-local:opencode-go"
terms_evidence = "docs/planning/commerce-refined/EVIDENCE.md (2026-11-14)"

[[role]]
id = "commerce.draft"
purpose = "CommerceDraft"          # cannot use PublicCoding/Personal and vice versa
provider = "opencode-go"
model = "deepseek-v4.1-flash"      # existing allowlisted model
max_input_tokens = 3000
max_output_tokens = 600
max_calls_per_task = 1
timeout_seconds = 30
daily_call_cap = 20
allowance_share_percent = 10
session_id = "operation"           # stable per operation; never a user identifier
user_agent = "zaturo-commerce-bot"
data_class_max = "public_product_facts"
```

Adapter rules: purpose gate; reserve before send, settle after receipt, timeout keeps the
reservation as `uncertain`; data-class check before reservation; per-call log (role,
provider, model, tokens, reserved/settled, latency, outcome); invalid JSON, unknown ids,
any number or URL not present in the offer, forbidden superlatives or first-person test
claims → template fallback or `blocked(model_unavailable)`; never a silent provider switch.
Swapping the provider = editing `[[provider]]`/`[[role]]` and the evidence record.

### 7.5 Alternatives if OpenCode limits non-coding traffic (owner choice, none activated)

| Option | Terms fit | Data | Price (2026-11-14, per 1M tokens in/out) |
|---|---|---|---|
| Templates only | n/a | stays local | 0 |
| Anthropic API (small model) | commercial use incl. serving end users; DPA; no training on customer content | US | Haiku 5.5: US$ 0.10 / 0.50 |
| Google Gemini API, paid tier only | business use; paid tier not used to improve products | DPA | 2.5 Flash-Lite: US$ 0.10 / 0.40 |
| DeepSeek first-party API | commercial integration allowed; must disclose AI output | stored in PRC | Flash: US$ 0.30 / 1.20 (off-peak −50 %) |
| Local Apache-2.0 model (e.g. Qwen3 4B/8B) | no provider terms | local | hardware only; 8B Q4 ≈ 6–8 GB RAM; home PC only, never the VPS; quality unproven |

---

## 8. Platform capability matrix

Official sources retrieved 2026-11-14; exact URLs and clause references are in
[EVIDENCE.md](EVIDENCE.md). This is engineering due diligence, not legal advice. Every
"owner check" must be confirmed inside the owner's logged-in dashboard before activation.

### 8.1 Affiliate programs

| Program | Official automation interface | Media / channel rules relevant here | Link rules | Plan use | Evidence |
|---|---|---|---|---|---|
| **Shopee (BR)** | Affiliate Open API (GraphQL, signed requests) — Brazil endpoint exists; offer lists, short-link generation and conversion reports are documented for Shopee SG | Media must be registered and approved; Telegram/WhatsApp not named. **§4.3(d): content must not link to other e-commerce platforms.** **§4.4(j): no affiliate links without relevant content; no duplicate content to the same audience.** §4.4(d) no scraping; §4.3(c) follow CONAR/CBAP | §2.2/§1.3 links are Shopee materials, not to be modified; §4.5 no automatic redirects/iframes/cookie stuffing; shorteners not addressed → use only Shopee's own links | **Recommended first program**, conditional on owner check that the Telegram channel can be registered as approved media. Only path with an official API for offers, tracked links and conversions (F7). | Terms text VERIFIED (page titled for participating sellers, binds "o Afiliado", updated 2026-08-17); BR API credentials, sub-IDs and rate limits UNVERIFIED |
| **Mercado Livre** | No public affiliate API (§1.7 "may" provide); developer API needs OAuth and its terms forbid redistributing content outside the app | Declared media only (§1.3); offline excluded; the high-volume "Divulgadores de Ofertas" category names WhatsApp/Telegram groups and sites; §5.3 no comparative ads; §5.4 only information present in the listing; §7.3 no bought lists or mass messaging; §1.9/§3.1.2(c) sales from misused software/robots unpaid | §1.8 no shorteners, no editing or masking of links | **Fallback program.** Links generated manually in the dashboard (more owner toil). Electronics/computing/phones commission 5 % direct, 2.5 % indirect, 24 h attribution. | VERIFIED |
| **Amazon (BR)** | PA-API 5 deprecated → Creators API (OAuth; needs 10 qualified sales in the trailing 30 days; starts at 1 TPS / 8,640 req/day) | Social profiles count as a "Site"; links in email/SMS/DMs only when requested; offline banned; mandatory disclosure sentence | No shorteners/buttons hiding the Amazon destination; prices only via Amazon link or API data with timestamp (cache ≤ 24 h, disclaimer when not refreshed hourly) | **Not in the pilot.** API gated by sales; revisit after F5 if the channel converts. Never mix with Shopee in the same channel (Shopee §4.3(d)). | VERIFIED; PA-API shutdown date UNVERIFIED |

Consequences built into the policy and templates:

- **One affiliate program per channel** in the pilot (Shopee §4.3(d)).
- **Every post carries relevant content** (headline + 2–3 factual benefit lines); never a
  bare link (Shopee §4.4(j)); the same item is not re-posted to the same channel within
  14 days (policy cap `repost_cooldown_days = 14`).
- **Links are used exactly as produced by the program's own tool**; no shorteners,
  redirects or parameter edits (all three programs).
- **Attribution by sub-ID only if the program's own link tool offers it** (owner check for
  Shopee BR); otherwise attribution is per channel/campaign and kill rules use
  campaign-level numbers.

### 8.2 Channels

| Channel | Official interface | Automated posting | Affiliate links | Key restrictions | Plan use | Evidence |
|---|---|---|---|---|---|---|
| **Telegram channel** | Bot API (bot as channel admin with `can_post_messages`) | Yes | No Telegram rule against them; program media rules apply | No idempotency key on `sendMessage`; `retry_after` on flood control; ~1 msg/s per chat; bot developer terms require a privacy policy, minimal data, no spam | **Primary pilot channel (F4).** Two posts/day is far below limits. | VERIFIED |
| **Telegram bot DMs** | Bot API | Replies only — "Bots can't start conversations with users" | Same | Same | Support pilot (F10), user-initiated by construction | VERIFIED |
| **WhatsApp via automated WhatsApp Web** | None | **No** — WhatsApp terms forbid bulk/auto-messaging and automated access; bans can be automatic | — | Account ban risk | Never for commerce; MeuAssistente synthetic tests only (situation A) | VERIFIED |
| **WhatsApp Business Platform (Cloud API)** | Yes, but it is the Meta API the owner rejected (ADR-0027) | 1:1 only, opt-in, templates outside 24 h, human escalation required; **groups and Channels not supported** | Not prohibited (INFERRED) | AI-provider clause §4.7 (general-purpose AI as primary function barred; a business's own product Q&A is ancillary — INFERRED; Brazil exemption status UNVERIFIED) | Only if the owner reverses ADR-0027 (B6) | VERIFIED / INFERRED |
| **WhatsApp Channels** | No official posting API | No | — | Channel guidelines discourage too many/low-quality updates | Manual only (situation B) | VERIFIED |
| **Instagram** | Graph API content publishing (professional account) | Yes, own account (publishing limit stated as 100/24 h; the same page also says 50 for carousels) | Not covered by the API docs | Meta app; the owner's no-Meta decision was made for WhatsApp in MeuAssistente — whether it extends to Instagram is an owner decision | Not in pilot; F11 candidate (O-10) | VERIFIED |
| **Postiz-connected networks (Bluesky, Mastodon, Threads, …)** | Existing ZaturoBot social connector → Postiz Public API | Yes, per network rules | Per network | Postiz must be deployed off the VPS; per-network policies | F11 organic discovery candidate | VERIFIED (connector) |
| **YouTube via YoutubeBot** | Existing code drives YouTube Studio in a browser | Not via an official API | — | Browser automation of a Google account | Not used for commerce | VERIFIED |

### 8.3 Brazilian rules applied

- **CONAR 2026 influencer guide**: content with tracked affiliate links is advertising; it
  must be identified clearly, preferably with the platform's paid-partnership tool,
  otherwise with `#publi` / `#publicidade` visible without expanding the text; English tags
  may be insufficient → every post **starts** with `#publi`.
- **CDC** arts. 36–38: advertising must be immediately identifiable; misleading by
  omission (e.g. price) is forbidden; the burden of proof is on the advertiser → evidence
  hashes and timestamps are kept for every published claim.
- **LGPD**: consent or legitimate interest with transparency and free revocation; ANPD's
  legitimate-interest guide accepts promotional messages to existing relationships with an
  unsubscribe mechanism → the support bot answers only users who wrote first and honours
  `/parar` immediately; the channel itself stores no personal data.
- **Chatbot disclosure**: no general rule in force found (PL 2338/2023 awaiting report as of
  2026-11-13); MeuAssistente already discloses automation in every reply and this plan keeps
  that rule for Telegram.

---

## 9. End-to-end journeys

Each journey lists trigger, observable states, expected result and the test that proves
it. "Feed" is the existing Hub activity feed.

| # | Journey | Trigger | Observable states | Expected result | Test (phase) |
|---|---|---|---|---|---|
| J01 | First configuration | Owner runs onboarding (CLI first, native panel in F8) | `mode=setup` → policy `proposed` → `active`; `mode=paused` | Nothing can publish until a policy is `active` **and** `mode=running`; missing channel/program evidence keeps actions at `draft`. | `onboarding_without_active_policy_never_admits_publish` (F3) |
| J02 | First offer collection | Owner imports an official export (or API sync in F7) | `offer.status`: `candidate` → `active` / `rejected(reason)` | Rows without official link, currency, observation time or allowed category are rejected with stable codes; repeat import is idempotent. | `offer_import_is_idempotent_and_rejects_unproven_rows` (F2) |
| J03 | First campaign | Owner creates the campaign from onboarding defaults | `campaign.status`: `draft` → `active` | Campaign pins the policy revision and caps; policy changes create a new revision and never mutate the campaign silently. | `campaign_pins_policy_revision` (F2) |
| J04 | First content produced | Batch generation (template; model in F6) | `content.state=draft`, `generated_by`, `text_sha256` | Only accepted offer fields; disclosure present; forbidden claims absent; ≤ 1500 bytes. | `template_draft_contains_disclosure_and_only_evidenced_facts` (F2); `model_draft_with_invented_number_is_rejected` (F6) |
| J05 | First scheduling | Hub tick reaches a due slot | `slot=admitted` with `task_id`; `content=scheduled` | Exactly one Engine task per slot key under restart and concurrent ticks. | `slot_admission_is_unique_under_concurrency_and_restart` (F3) |
| J06 | First authorized publication | Worker claims the publish task | `scheduled → publishing → published(message_id)` | Dispatch check passes; intent journaled before the call; one Telegram request; receipt cached; one feed item. | `publish_records_intent_before_send_and_receipt_after` (F4) |
| J07 | First consented user interaction | A user writes to the support bot first | MeuAssistente conversation AI-owned; Hub event `answered` | Reply uses only Hub offer snapshots; first reply states it is automated and how to stop or reach a human. | `first_reply_discloses_automation_and_stop_command` (F10) |
| J08 | First registered conversion | Conversion import | `conversion=pending` with `sub_id → content_id` | Counted once; attributed by official sub-ID; unattributed rows shown as unattributed, never guessed. | `conversion_import_attributes_by_sub_id_and_is_idempotent` (F5) |
| J09 | First reconciled commission | Later report revision | `pending → approved` (or `cancelled`) | Only `approved` counts as verified revenue; reversals update the same row and report revision. | `late_reversal_updates_existing_conversion_and_report` (F5) |
| J10 | First system restart | Hub and/or worker restart mid-cycle | Hub reloads mode/slots/tasks; worker journal `Execute`/`Cached`/`Reconcile` | No slot re-admitted; completed receipt replayed from cache; in-flight send → `uncertain`, never resent. | existing `hub_restart_recovers_task_and_rejects_previous_lease` + `worker_crash_after_send_marks_uncertain_without_resend` (F4) |
| J11 | First API error | Telegram 400/401/403/429/5xx/timeout | `failed(code)` / `uncertain` / breaker open | 400 → failed, no retry; 429 → definite rejection, reschedule within window; timeout after send → `uncertain` + exception. | `telegram_error_classes_map_to_failed_uncertain_or_backoff` (F4) |
| J12 | First channel suspension | `getChat`/`getMe` 401/403 or bot loses admin rights | `channel_health=blocked`; future slots `skipped(channel_blocked)`; exception | Only that channel stops; one deduplicated exception; nothing retried until resolved. | `revoked_bot_token_blocks_channel_and_skips_slots` (F4) |
| J13 | First handoff to a human | User sends `/humano` or an ungroundable question | MeuAssistente ownership `human`; Hub event `handoff` | AI/bot stops answering that conversation until explicit owner release; owner notified once. | existing MeuAssistente `r10_*` takeover tests + `telegram_handoff_fences_bot_replies` (F10) |
| J14 | First duplicate execution attempt | Same slot twice, task re-delivered, same import repeated | Second attempt returns existing task/receipt | Zero extra Telegram calls (fake server counter), zero duplicate feed items, zero double-counted conversions. | `duplicate_publish_delivery_sends_once` (F4); `duplicate_import_counts_once` (F5) |
| J15 | First unsubscribe request | Support-bot user sends `/parar`, "parar" or "sair"; channel member leaves | Contact `opted_out`; Hub event `opt_out` | No further bot reply except the confirmation; works with the model disabled or budget exhausted. Leaving the channel needs no action. | `stop_command_opts_out_without_model_and_blocks_replies` (F10) |
| J16 | First stop due to a financial/usage limit | AI allowance cap reached, or a kill rule fires | Drafts `blocked(budget)` or template fallback; campaign `paused(kill_rule)`; exception | Cap never exceeded (reserved + uncertain counted); visible in dashboard; resuming needs owner action or next period. | `allowance_cap_falls_back_to_templates` (F6); `kill_rule_pauses_campaign_and_skips_future_slots` (F5) |

---

## 10. Corrected implementation roadmap

Each phase is sized for one agent session and ends in something that runs and is tested.
Order follows the requested priorities, adjusted by the audit: metrics come before AI and
before the affiliate API because kill rules depend on them; the support pilot comes after
the MeuAssistente bridge.

| Phase | Result | Repos | Needs real credentials? | Depends on |
|---|---|---|---|---|
| **F0** | Preconditions: computer-use work committed; secrets-scan blocker resolved by owner; ADR for commerce + D-01 accepted; owner decisions O-01…O-08 answered | ZaturoBot (docs) | No | — |
| **F1** | `zaturo-core::commerce`: money, offer validation, content state machine, policy document + validator, claim/disclosure lint, slot computation, kill-rule evaluation, sub-ID attribution — pure, fully tested | ZaturoBot | No | F0 (ADR) |
| **F2** | Migration 0010 (hub), repository, services: offer import, deterministic ranking, template drafts, batch creation, feed summary, `/v1/commerce/status` + import routes + CLI | ZaturoBot | No | F1 |
| **F3** | Policy activation, `commerce_control` + stop/resume, slot admission in the Hub tick, batch approval (owner authority), dispatch-check route, exceptions; fake publisher end-to-end | ZaturoBot | No | F2 |
| **F4** | `CommercePublish` service, worker executor, `telegram_bot` transport in the social helper, readiness/metrics, uncertainty handling; fake Telegram server tests; **then** owner-authorized live smoke in a private test channel | ZaturoBot | Only for the final smoke (bot token + test channel) | F3 |
| **F5** | Conversion import (official export), sub-ID attribution, report revisions, kill rules active, weekly pt-BR report in the feed | ZaturoBot | No (fixtures); real report for pilot | F4 |
| **F6** | D-01: `CommerceDraft` purpose on the Go route, role config, prompt file, JSON proposal validator, allowance cap, template fallback | ZaturoBot | Existing Go key on home worker (owner-run smoke only) | F2, F0 ADR |
| **F7** | Official affiliate API adapter (offers + tracked short links + conversions) replacing manual imports | ZaturoBot | Yes (program API access) | F5, access granted |
| **F8** | Native owner panel (Linux Slint + Android): status, stop, next posts, batch approval, exceptions, revenue, AI usage; onboarding wizard | ZaturoBot | No | F3–F5 |
| **F9** | MeuAssistente as Hub app client (offers pull, events push) in synthetic mode; new `affiliate-v1` grounded response kinds behind tests | Both (needs explicit authorization to edit MeuAssistente) | No | F2, owner authorization |
| **F10** | Telegram support pilot in MeuAssistente: official Bot API long polling (no inbound port), deterministic `/ofertas`, `/parar`, `/humano`, opt-out registry, handoff; model replies only with an eligible production provider | MeuAssistente | Yes (support bot token) | F9, owner production decision |
| **F11** | Controlled scale: second organic surface via the existing Postiz connector (e.g. Bluesky/Threads) or an owned static page, branded image cards, bounded optimization, second campaign | ZaturoBot | Yes per surface | F5 + ≥ 30 days data |

Production never depends on fictitious tests: F4/F7/F10 each end with a separately
authorized real smoke, recorded as real evidence, distinct from fixture results.

---

## 11. Acceptance tests per phase

Global rules: tests are offline and deterministic (fixed clocks, temporary databases,
loopback fake servers that count requests durably, synthetic identities); no paid calls,
no real accounts, no real ledgers; existing tests keep passing; `cargo xtask check` is the
final gate (see blocker B0). Test names are the contract for the next agent.

| Phase | Must pass (new) | Must keep passing (existing) |
|---|---|---|
| F1 | `money_rejects_overflow_and_float_inputs`; `offer_without_official_link_or_timestamp_is_rejected`; `content_state_machine_rejects_illegal_transitions`; `policy_document_rejects_unknown_fields_and_out_of_range_caps`; `claim_lint_rejects_testing_claims_superlatives_and_unbacked_discounts`; `disclosure_must_precede_link_and_survive_truncation`; `slots_for_cuiaba_are_stable_across_utc_midnight`; `kill_rule_no_clicks_14_days_pauses`; `sub_id_attribution_maps_known_ids_only` | `cargo test -p zaturo-core` |
| F2 | `offer_import_is_idempotent_and_rejects_unproven_rows`; `campaign_pins_policy_revision`; `template_draft_contains_disclosure_and_only_evidenced_facts`; `ranking_is_deterministic_and_explains_exclusions`; `status_route_requires_scope_and_hides_secrets`; `hub_nine_upgrades_to_ten_and_failed_upgrade_rolls_back`; `commerce_disabled_leaves_existing_routes_unchanged` | `crates/db/tests/native_migrations.rs`, `crates/services/tests/engine.rs`, activity tests |
| F3 | `onboarding_without_active_policy_never_admits_publish`; `slot_admission_is_unique_under_concurrency_and_restart`; `missed_slot_is_skipped_not_caught_up`; `stop_cancels_queued_commerce_tasks_and_denies_dispatch`; `resume_does_not_clear_uncertain_or_exceptions`; `batch_approval_requires_owner_and_binds_hashes`; `changed_text_after_approval_is_denied_at_dispatch`; `dispatch_check_allows_once_per_content` | Engine lease/cancel tests |
| F4 | `publish_records_intent_before_send_and_receipt_after`; `worker_crash_after_send_marks_uncertain_without_resend`; `duplicate_publish_delivery_sends_once`; `telegram_error_classes_map_to_failed_uncertain_or_backoff`; `revoked_bot_token_blocks_channel_and_skips_slots`; `bot_without_post_rights_is_not_ready`; `html_escaping_prevents_markup_injection_from_offer_titles`; Postiz transport regression suite unchanged | social helper tests run by `cargo xtask check`, `service_journal` tests |
| F5 | `conversion_import_attributes_by_sub_id_and_is_idempotent`; `late_reversal_updates_existing_conversion_and_report`; `duplicate_import_counts_once`; `mixed_currencies_are_never_summed`; `unknown_is_reported_as_unknown_not_zero`; `kill_rule_pauses_campaign_and_skips_future_slots`; `weekly_report_fits_feed_limit_and_is_deduplicated` | feed/activity tests |
| F6 | `commerce_purpose_never_uses_public_coding_or_personal_profiles`; `existing_go_and_console_profiles_unchanged`; `model_draft_with_invented_number_is_rejected`; `model_output_urls_are_rejected`; `allowance_cap_falls_back_to_templates`; `timeout_keeps_reservation_uncertain`; `request_contains_only_public_product_facts`; `user_agent_and_session_id_are_honest_and_stable` | `crates/llm` router tests, orchestrator malicious-proposal tests |
| F7 | `api_offers_map_to_same_validation_as_imports`; `tracked_links_come_only_from_official_generator`; `api_outage_falls_back_to_last_valid_offers_without_price_claims` | F2/F5 suites |
| F8 | Slint/Android UI tests: `stop_button_calls_stop_and_shows_in_flight_warning`; `status_shows_unknown_not_zero`; `approval_screen_shows_exact_text_and_hash` | existing native client tests |
| F9 | `hub_client_uses_scoped_token_and_never_db_access`; `expired_offers_never_rendered`; `events_contain_no_personal_data`; `affiliate_kind_cannot_alter_price_or_link`; MeuAssistente `cargo xtask check` | MeuAssistente full suite (ownership/outbox/provider tests) |
| F10 | `first_reply_discloses_automation_and_stop_command`; `stop_command_opts_out_without_model_and_blocks_replies`; `telegram_handoff_fences_bot_replies`; `bot_never_initiates_conversation`; `opted_out_user_is_never_re_enrolled_by_new_message` | MeuAssistente `r10_*`, `r11_*`, `r20_*` |
| F11 | `optimization_cannot_change_destination_disclosure_or_caps`; `insufficient_sample_keeps_baseline` | all above |

---

## 12. Rollback procedures

Rolling back software never pretends to roll back effects: published posts stay
published, imported conversions stay recorded, spent allowance stays spent.

| Phase | Rollback | Preserved | Never |
|---|---|---|---|
| F0 | Documentation only | — | Delete snapshot refs before the owner confirms the flagged literal is synthetic |
| F1 | Revert the commit (no state) | — | — |
| F2 | `mode=paused` / feature off; keep tables. Binary rollback needs the pre-migration backup **and** a binary that knows that schema | Offers, content, hashes | Edit an applied migration; copy a live WAL file as backup |
| F3 | `mode=paused` cancels queued commerce tasks; tick stops admitting | Slots (keys never reused), exceptions | Recreate or replay slots |
| F4 | `mode=stopped`; `allow_execute=false`; revoke bot token in BotFather if compromised; remove capability from worker config | Journals, receipts, `uncertain` rows | Resend `uncertain`; delete journal rows |
| F5 | Disable importer; mark reports stale | All rows and revisions | Rewrite approved/cancelled history |
| F6 | Role `commerce.draft` → template mode | Usage log, reservations | Auto-switch provider; reset ledgers |
| F7 | Offer sync `off`; back to owner imports | Cursors, evidence | Fall back to scraping |
| F8 | Hide panel; CLI + feed remain | — | — |
| F9 | Revoke MeuAssistente's Hub app credential | Both databases | Give it database access as a workaround |
| F10 | Support bot `off` in MeuAssistente; channel publishing unaffected | Opt-out records, audit | Delete opt-out records |
| F11 | Restore baseline template/slot config for future slots | Experiment results | Re-run historical slots |

Before every migration: stop the writer, take a consistent SQLite backup with the
`.backup` command (not a file copy of a live WAL database), record binary and schema
versions next to it, then start the new binary — following `docs/DEPLOYMENT.md` at the
time of implementation.

---

## 13. Unresolved blockers

| Id | Blocker | Blocks | Who resolves | Status |
|---|---|---|---|---|
| B0 | `cargo xtask check`/`secrets` fail on a credential-shaped literal reachable via `refs/codex/turn-diffs/*` | Every phase's final gate | Owner (decide whether the literal is synthetic; then fix the source and prune snapshot refs under the scanner's policy). Never weaken the scanner. | Open |
| B1 | Concurrent computer-use work uncommitted on `main` (schema 9, ADR-0020, registry) | F1 start | Owner/other session commits it | Open |
| B2 | Affiliate program enrollment and Telegram channel as an approved/declared medium (Shopee does not name Telegram; Mercado Livre requires declared media) | Real pilot (F4 smoke onward) | Owner, inside the program dashboard | Open — see section 8.1 |
| B2b | Sub-ID support in the chosen program's own link tool (UNVERIFIED for Shopee BR) | Per-post attribution (falls back to per-campaign) | Owner check | Open |
| B3 | Official affiliate API access | F7 (manual imports work until then) | Owner applies | Open |
| B4 | OpenCode Go acceptance of non-coding traffic (D-01 risk) | F6 quality only; templates unaffected | Observed in pilot; owner may switch provider | Accepted risk, monitored |
| B5 | No production-approved provider in MeuAssistente (Q-001), retention/privacy (Q-005), Phase 7 not authorized | Model replies in F10 | Owner | Open |
| B6 | WhatsApp commercial automation: no official path without the Meta API, which the owner rejected (ADR-0027) | WhatsApp situation C | Owner (only by reversing ADR-0027) | Blocked by decision |
| B7 | `/aprovar` has no actor check | Reusing that path for commerce | Fixed by requiring owner authority on commerce approvals (F3); general fix belongs to the computer-use owner | Open |
| B8 | Organic discovery: a Telegram channel has weak discovery on its own | Revenue scale | Owner chooses F11 surface | Open |
| B9 | Home worker availability (slots are skipped while the PC is off) | Cadence reliability | Owner may later run a commerce-only worker elsewhere (O-09) | Accepted for pilot |

### 13.1 WhatsApp: the three situations, evaluated separately

| Situation | Today | Allowed by this plan | Path |
|---|---|---|---|
| **A. Fictitious conversations and controlled tests** | Implemented in MeuAssistente Phase 6 (synthetic-only, startup OFF, manual activation) | Yes, unchanged | Recommend `ALLOW_SELECTED` with only test chats before any activation (R-N1). No commerce data is routed through it. |
| **B. Human-assisted publishing and service** | A human using WhatsApp normally is not automation; MeuAssistente's assisted routes exist but its production inference is disabled | Owner may **manually** share channel posts/links in WhatsApp groups where they already participate and the affiliate program allows that medium (see section 8). No automated sending, no AI drafting for real customers in MeuAssistente until Q-001/Q-005 are resolved. | Manual only |
| **C. Automated commercial service in production** | Blocked: WhatsApp's terms do not allow automating the consumer app/Web client; the official route is the WhatsApp Business Platform (Meta), which the owner rejected (ADR-0027); MeuAssistente schema enforces `synthetic_only=1` | **No.** The plan does not simulate success. | Official alternative: Telegram support bot (F10). WhatsApp automation is possible only if the owner reverses ADR-0027 and adopts the Business Platform with its own policies. |

The plan never includes spam, cold outreach, contact scraping, detection evasion,
human-presence simulation or unconsented sending.

---

## 14. Commercial pilot plan

### 14.1 The single experiment

| Item | Proposal | Status |
|---|---|---|
| Brand | "Zaturo Setup" — check availability of the Telegram handle, domain and INPI before adoption | Owner decision O-01 |
| Niche | Affordable tech accessories/setup items (cables, chargers, hubs, mice, keyboards, headsets, webcams, lighting, stands), price band R$ 30–300 | O-02 |
| Excluded | Health claims, financial products, used/refurbished without seller evidence, anything needing a physical-test claim | Proposed default |
| Monetization | Shopee affiliate program only (section 8.1), if the owner confirms the Telegram channel can be registered as approved media; otherwise Mercado Livre with a declared Telegram channel. One program per channel. Official links only; sub-IDs only if the program's own tool offers them | O-03 |
| Retention | Owned public Telegram channel (members join voluntarily; leaving is the opt-out) | Proposed |
| Discovery | Pilot: owner shares the channel where they already participate (manual); F11 adds one automated organic surface | O-10 |
| Cadence | 2 posts/day, windows 11:30–12:30 and 19:00–21:00 `America/Cuiaba` | O-05 |
| Format | Text post: first line `#publi`, headline, 2–3 factual benefit lines taken from the listing (never a bare link — Shopee §4.4(j)), price with timestamp, official link (Telegram's link preview shows the official product page, so no image is copied), disclosure line. Same item not re-posted within 14 days | Proposed |
| Disclosure | First line `#publi` (CONAR 2026: visible without expanding). Closing line: "Link de afiliado: posso receber comissão se você comprar. Preço verificado em DD/MM às HH:MM e pode mudar." Pinned post: "Este canal usa automação para preparar as ofertas. Nenhum produto foi testado fisicamente, salvo quando indicado." | O-06 (wording) |
| Duration | 8 weeks after Stage B begins | Proposed |
| Budget | AI: within the D-01 allowance share (no extra money); no ads; no paid tools | O-07 |

### 14.2 What is measured — and what is not

Measured only from official sources: posts published (Telegram receipts), channel member
count (`getChatMemberCount`), clicks/orders/commission status per sub-ID (program report),
AI usage (ledger). Not reported as numbers: per-post views (not reliably available to
bots), unapproved revenue, "estimated" earnings.

### 14.3 Decision gates

- **Week 2:** quality and zero-incident check → owner may switch publishing to `auto`.
- **Week 4:** first attributed clicks/conversions expected. No attributed click for 14
  consecutive days → kill rule pauses the campaign and asks the owner to change offers.
- **Week 8:** continue / adjust / stop. Proposed continue criterion: approved commission ≥
  direct cost in the period and growing member count. The owner sets the numbers; the
  system reports them truthfully.

### 14.4 Kill rules (deterministic, in the policy document)

1. AI allowance share reached → model drafts stop; templates continue if allowed.
2. No attributed click for 14 days → `paused(no_clicks)`.
3. No approved commission after 45 days of `auto` publishing → `paused(no_revenue)`.
4. Channel block, unresolved `uncertain` older than 24 h, or 3 validator rejections in a
   day → publishing demoted to `approve`.
5. Offer source unavailable for 48 h → no price-bearing posts.

---

## 15. Exact instructions for the next agent

1. **Do not start until** B0 and B1 are resolved and the owner has answered at least
   O-01…O-06 (or explicitly accepted the proposed defaults). Ask if unclear; silence is not
   approval.
2. Read, in order: root `AGENTS.md`; this file; [IMPLEMENTATION-BACKLOG.md](IMPLEMENTATION-BACKLOG.md);
   `crates/core/AGENTS.md`; [SCHEDULER](../../SCHEDULER.md); [SOCIAL_MANAGEMENT](../../SOCIAL_MANAGEMENT.md);
   [DECISIONS](../../DECISIONS.md) (ADR-0010, 0019, 0020); `docs/MIGRATIONS.md`.
3. Re-verify the facts this plan depends on (they may have moved):
   `git status`; latest migration file and `PRAGMA user_version` constants in
   `crates/db/src/lib.rs`/`repository.rs`; capability allowlists in
   `crates/services/src/lib.rs` and `apps/zaturo-worker/src/config.rs`; last ADR number.
   Record differences before coding.
4. Work on a new branch from the then-current `main` (e.g. `commerce/f1-core`) in a
   separate worktree. Do not touch the original checkout's uncommitted files, do not
   stash/reset, do not restart services.
5. Implement **only F1** (backlog items BL-101…BL-110): add `crates/core/src/commerce/`
   (`mod.rs`, `money.rs`, `offer.rs`, `content.rs`, `policy.rs`, `claims.rs`, `slots.rs`,
   `kill_rules.rs`, `attribution.rs`) and
   `crates/core/tests/commerce_*.rs`. No SQL, no I/O, no new dependency except a reviewed tz
   library if `slots.rs` needs one (document it in `docs/DEPENDENCIES.md`).
6. Write the F1 tests listed in section 11 first, then the code.
7. Run: `cargo fmt --all -- --check`; `cargo test -p zaturo-core`; `cargo xtask arch`;
   `cargo xtask docs`; `cargo clippy -p zaturo-core -- -D warnings`; then
   `cargo xtask check`. If the gate fails only at B0, report it verbatim; never weaken the
   scanner.
8. Update together: `docs/COMPONENTS.md`, `crates/core/AGENTS.md`, `docs/ROADMAP.md`
   (commerce marker), `config/registry.toml` (planned → partial only if the registry
   allows it), and the commerce ADR status. Do not mark later phases implemented.
9. Report: commands and actual results, tests added, limitations, and the next backlog id.
10. Never in F1–F3: call any provider, create accounts, use real tokens, publish, message,
    touch MeuAssistente or WhatsApp, or change the Go/Console profiles.

---

## 16. Final answers

**Smallest implementation that can generate commercial value.** F1–F5: verified offers
imported from the program's official export, deterministic ranking, template posts with
disclosure and timestamped price, batch approval then auto, two slots per day, kill
switch, Telegram channel publication via the official Bot API, and conversion import with
sub-ID attribution and kill rules. No AI is required for it to work; F6 (OpenCode Go
engine) improves copy without changing authority.

**First phase that can start without real credentials.** F1 — and F2 and F3 entirely; F4
up to the fake-server suite.

**Integrations that require human actions.**

- Telegram: create the channel and the bot with BotFather, add the bot as channel admin
  with post rights only, place the token in the worker's private file.
- Affiliate program: enroll, accept terms, register the Telegram channel (and any other
  medium) as an approved medium, generate/export links with sub-IDs, download conversion
  reports (until API access), apply for API access.
- OpenCode Go: confirm the subscription and the allowance share for commerce (D-01);
  keep the key worker-local.
- MeuAssistente (F9/F10): explicit authorization to edit that repository; a production
  decision for the support bot; a second bot token.
- Any new surface (F11): account creation, OAuth consent or Postiz deployment by the owner.

**Risks that block full automation.**

- No official offer/link/conversion API access → weekly manual import (~15 min).
- Telegram has no lookup by our operation id → every `uncertain` send needs the owner.
- Affiliate medium approval for Telegram may be refused or limited.
- OpenCode may throttle or suspend non-coding traffic (D-01 risk) → template fallback.
- Weak organic discovery of a standalone Telegram channel.
- Home worker downtime skips slots.
- WhatsApp automation is not possible without reversing the no-Meta decision.

**Decisions that need the owner's approval.** See the list below; none is assumed.

| Id | Decision | Proposed default |
|---|---|---|
| D-01 | OpenCode Go as bounded engine for bot actions | **Decided by owner 2026-11-14**; record as ADR in F0 |
| O-01 | Brand name and handle | "Zaturo Setup", after availability check |
| O-02 | Niche, price band, exclusions | Section 14.1 |
| O-03 | First affiliate program | Section 8 recommendation |
| O-04 | Drafting trigger in Stage C (owner action vs bot schedule) | Owner action until week 2, then bot schedule if no OpenCode limits observed |
| O-05 | Posting windows and posts/day | 2/day, 11:30–12:30 and 19:00–21:00 |
| O-06 | Disclosure wording and pinned automation note | Section 14.1 |
| O-07 | Allowance share and daily call cap for commerce | ≤ 10 % of the monthly Go allowance, ≤ 20 calls/day |
| O-08 | Kill-rule thresholds | Section 14.4 |
| O-09 | Where the publishing worker runs | Home worker for the pilot |
| O-10 | Discovery surface for F11 | Decide after week 8 |
| O-11 | Authorize MeuAssistente edits (F9) and a production support bot (F10) | Not before week 8 |
| O-12 | Switch MeuAssistente synthetic WhatsApp to `ALLOW_SELECTED` with only test chats | Recommended now (owner action; nothing changed by this plan) |
| O-13 | Resolve B0 (snapshot refs secret finding) | Required before F1 |
