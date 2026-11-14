# Zaturo Commerce — evidence and verification log

## Implementation refresh — 2026-11-14

The owner has now requested the base implementation and GitHub publication after
validation. Work is scoped to **F1 only**; [current commerce contracts](../../COMMERCE.md)
and [F1 verification](../../COMMERCE_F1_REPORT.md) describe the actual result. The original
review below remains dated evidence; the following refresh supersedes stale assumptions:

- Main computer/social work is committed (`a0cdb4f`), followed by explicit Personal Go
  routing (`d878998`, ADR-0021). Hub9/worker8/protocol2 remain unchanged by F1.
- B0's old snapshot scanner finding was resolved: the unchanged current-main scan passed
  during refresh. Fresh full-gate and pre-push scans remain required; no refs were pruned
  and no scanner weakened by this implementation. B1 is resolved by the committed main.
- ADR-0020 and ADR-0021 are occupied. Commerce F1 uses ADR-0022. F6's proposed CommerceDraft
  purpose is not implemented, and current Personal Go/coding profiles remain unchanged.
- O-01…O-08 are activation choices. The current explicit request authorizes parameterized
  pure F1 development with synthetic fixtures, without accepting a brand/niche/channel or
  cap proposal. Do not treat that interpretation as permission for a later live operation.
- The existing artifact API is encrypted-transfer metadata, not a generic plaintext upload
  endpoint. F2 must design bounded intake against real endpoints rather than assume one.
- Claim lint must distinguish typed title/benefit numbers and generated observation times
  from invented claims. No auto-responder, paid call or new provider path belongs in F1.


Companion to [ZATURO-COMMERCE-REFINED-PLAN.md](ZATURO-COMMERCE-REFINED-PLAN.md) and
[IMPLEMENTATION-BACKLOG.md](IMPLEMENTATION-BACKLOG.md). Review date: 2026-11-14.

Classification:

- **VERIFIED** — seen directly in code, repository documents or an official web page.
- **INFERRED** — a conclusion drawn from verified material; the reasoning is stated.
- **UNVERIFIED** — could not be confirmed (page blocked, JavaScript-only, third-party only,
  or requires the owner's logged-in account).

## 1. Method and limits

- The Codex plan was read in full from branch `codex/autonomous-commerce` (commit
  `eb6a0cb`), worktree `~/.codex/worktrees/autonomous-commerce/ZaturoBot`, folder
  `docs/planning/autonomous-commerce/` (README + 13 documents), plus its Codex research
  notes in `/tmp/zaturo-commerce-*.md`.
- ZaturoBot was read on `main` **including the uncommitted computer-use work** (base
  `9456053`). Another session was editing `main` during the review (registry, ADRs, docs);
  line numbers can move. MeuAssistente was read at `main` `9cd1df2` (clean).
- Read-only: no builds or tests were run (to avoid the other session's build lock), no
  services touched, no `.env`, `.secrets/`, ledgers or MeuAssistente `data/` opened, no
  account, provider or channel contacted except public web pages.
- External pages were fetched on 2026-11-14. Some were summarized by a fetch tool; the
  clauses this plan depends on most (OpenCode terms; Shopee §4.3(d) and §4.4(j); CONAR
  `#publi`) were re-checked against the page text. Terms change; re-verify before each
  activation. This is not legal advice.

## 2. Repository evidence — ZaturoBot

| Fact | Where | Class |
|---|---|---|
| Layering: rules in core, SQL in db, use cases in services, wire in protocol, provider transport in llm, planning in orchestrator; binaries compose | `AGENTS.md` (working rules), `architecture.toml` | VERIFIED |
| Hub has no `zaturo-llm` dependency (never calls providers); worker may depend on llm, youtube, computer | `architecture.toml` | VERIFIED |
| Scheduling, full desktop automation and arbitrary account actions are still "planned" in the authorized scope | `AGENTS.md` (first paragraph), `config/registry.toml` feature `scheduling` | VERIFIED |
| Task engine: 10 states; kinds Diagnostic/AssistedDiagnostic/Service; request ≤ 4096 B; idempotency key ≤ 128; attempts 1..3; timeout 1..300 s | `crates/core/src/lib.rs` | VERIFIED |
| Hub schema 9 (`0009_hub_computer.sql`, table `computer_steps`); worker schema 8; numbering is one shared inventory → next is 0010 | `crates/db/src/repository.rs`, `crates/db/src/lib.rs`, `crates/db/migrations/`, `docs/MIGRATIONS.md` | VERIFIED (next number INFERRED) |
| Protocol VERSION 2, 16 KiB; unchanged by computer work, which added device HTTP routes with an `x-zaturo-lease` header instead | `crates/protocol/src/lib.rs`, `apps/zaturo-hub/src/computer.rs` | VERIFIED |
| Capabilities: config-checked allowlist (13 in worker, ≤ 16 in hub), duplicated in two places; new `computer.*`, `price.search.v1` | `apps/zaturo-worker/src/config.rs`, `crates/services/src/lib.rs` | VERIFIED |
| New service variants `ComputerTask`, `ComputerStep`, `PriceSearch`, `DesktopPrivateView` | `crates/core/src/service.rs` | VERIFIED |
| LLM: only `deepseek-v4.1-flash`; providers `opencode-go` (PublicCoding) and `opencode-console` (Personal, attempts = 1); no fallback between routes | `crates/llm/src/config.rs`, `crates/llm/src/opencode.rs`, `crates/llm/AGENTS.md` | VERIFIED |
| Computer planner reuses the Console/Personal route (≤ 8 calls/task) and the shared `assistant_calls` ledger | `apps/zaturo-worker/src/computer.rs`, `crates/orchestrator/src/computer.rs`, `config/computer-models.example.toml` | VERIFIED |
| Console's last live check returned HTTP 402 | `docs/COMPUTER_USE.md`, `docs/SOCIAL_MANAGEMENT.md` | VERIFIED |
| Budget admission counts tokens and calls; `cost_microusd` is optional evidence; no dollar cap; day/month totals union `model_calls` and `assistant_calls` | `crates/core/src/model.rs`, `crates/db/src/model_budget.rs`, `crates/db/src/assistant_budget.rs`, `docs/LLM_PROVIDERS.md` | VERIFIED |
| No scheduler/cron/timer; Hub has internal ticks (1 s view purge, 60 s transfer cleanup, 5 s recovery) | `docs/SCHEDULER.md`, `apps/zaturo-hub/src/lib.rs` (`serve`) | VERIFIED |
| Social connector: Postiz Public API; platforms include `telegram`; prepare → hash → `/aprovar` → one POST → receipt/uncertain; ≤ 1500 B content; `allow_execute`; Postiz not deployed; never on the VPS; reachable today only through the model-planned ComputerTask | `docs/SOCIAL_MANAGEMENT.md`, `deploy/computer/social_service.py`, ADR-0020 | VERIFIED |
| `approve()` for computer steps checks step/parent state and lease but not the actor; any app with `can_submit` reaches it through `/v1/chat` | `crates/services/src/computer.rs` (`approve`), `apps/zaturo-hub/src/activity.rs` | VERIFIED |
| BitLupa stored-offer search: read-only, BRL, 6 h freshness, five retailers, no collector | `deploy/computer/bitlupa_service.py`, `docs/COMPUTER_USE.md`, ADR-0020 | VERIFIED |
| Feed: text ≤ 4096 B; day query offset client-selectable (−720..840, default −240); `service_activity` accepts only YoutubeBot jobs | `crates/core/src/activity.rs`, `apps/zaturo-hub/src/activity.rs`, `crates/services/src/activity.rs` | VERIFIED |
| No global pause/kill switch; per-task pause/resume/cancel exists | `apps/zaturo-hub/src/http.rs`, `crates/services/src/lib.rs` | VERIFIED |
| Topology: Hub on a 1 GiB VPS (150/300 MB targets, systemd caps), bound to loopback, reached via SSH tunnels; home worker 22 GB RAM; separate work-PC worker; VPS must not run models, browsers, Postiz or Docker | `docs/RESOURCE_LIMITS.md`, `docs/LIVE_DEPLOYMENT.md`, `deploy/` units, `docs/YOUTUBE_SERVICE.md` | VERIFIED |
| YoutubeBot: text generated by driving chatgpt.com (Playwright); upload through YouTube Studio pages; OpenCode Go only in a test fixture that strips `OPENCODE_GO_API_KEY` | GitHub `AlexandreZanata/YoutubeBot` default branch: `tools/ytbot/chat.py`, `tools/ytbot/config.py`, `tests/test_accounts.py` (code search 2026-11-14) | VERIFIED |
| OpenCode Go research in repo: Go targets coding-agent traffic; not a blanket authorization for unattended automation | `docs/OPENCODE_GO_RESEARCH.md` | VERIFIED |
| Quality gate: secrets scan reaches `refs/codex/turn-diffs/*` (four refs present) | `git for-each-ref`, Codex handoff validation record | VERIFIED (refs exist); finding content not inspected |
| Docs gate validates every relative Markdown link in the repository | `xtask/src/documentation.rs` | VERIFIED |

## 3. Repository evidence — MeuAssistente

| Fact | Where | Class |
|---|---|---|
| Phase 6 experimental synthetic desktop; production inference, real-customer automation, remote dashboard and deployment disabled | `AGENTS.md` | VERIFIED |
| Only `Fake` and `OpenCodeGo` providers; Go only for synthetic development/evaluation; production denied | `crates/llm/src/context.rs`, `crates/llm/src/client.rs`, `crates/llm/src/config.rs` | VERIFIED |
| USD 10 cumulative validation cap; models `deepseek-v4.1-flash`, `muse-spark-1.3-contributor`; ledger path fixed at compile time | `crates/llm/src/monetary.rs`, `apps/business-assistant/src/config.rs` | VERIFIED |
| `synthetic_only INTEGER NOT NULL DEFAULT 1 CHECK(synthetic_only=1)`, immutable | `crates/db/migrations/0007_web_automation.sql` | VERIFIED |
| Channels: only `FakeChannelAdapter` and the WhatsApp Web DOM adapter; no Telegram, no Cloud API | `crates/channels/src/lib.rs`, `crates/channels/src/web.rs`, `docs/channels/whatsapp-web.md` | VERIFIED |
| MCP: 21 restaurant tools; stdio and in-process duplex; no network listener; `mcp business --operator` grants operator permissions to any local caller | `crates/mcp/src/business.rs`, `crates/mcp/src/gateway.rs`, `apps/business-assistant/src/main.rs` | VERIFIED |
| Dashboard: loopback only, cookie sessions, CSRF; no service-to-service auth | `crates/dashboard/src/http.rs`, `crates/dashboard/src/auth.rs` | VERIFIED |
| Single vertical `restaurant-v1`; no plugin loading; grounded response kinds cannot render links or free prose | `docs/business-adapters/contract.md`, `apps/business-assistant/src/config.rs`, `prompts/base.md` | VERIFIED |
| Every reply is prefixed as automated | `crates/agent/src/runtime.rs` | VERIFIED |
| No opt-out keyword handling; no customer consent registry | repository-wide search; `0001_foundation.sql` `customers` table | VERIFIED (absence) |
| Default filter `BLOCK_SELECTED`; code cannot verify the "fictitious-only" attestation; ADR-0032 recorded 111 contacts and 27 groups on the linked account | `crates/services/src/restaurant/web_automation.rs`, `apps/business-assistant/src/config.rs`, `docs/DECISIONS.md` ADR-0032 | VERIFIED (facts) / INFERRED (risk R-N1) |
| Open questions Q-001 (provider), Q-005 (retention), Q-007 (channel eligibility), Q-016 (groups) block production | `docs/OPEN-QUESTIONS.md` | VERIFIED |
| ADR-0027 owner rejected the Meta API; ADR-0030 fictitious-number automation grants no production authority | `docs/DECISIONS.md` | VERIFIED |
| No mention of ZaturoBot, Telegram, affiliates or OpenCode Zen | repository-wide search | VERIFIED (absence) |

## 4. External evidence (retrieved 2026-11-14)

### 4.1 AI providers

| Finding | Source | Class |
|---|---|---|
| OpenCode Terms (effective 2026-11-08) cover "the website, the inference product and the hosted software" | https://opencode.ai/legal/terms-of-service | VERIFIED |
| Use only for the customer's "own internal use, and not on behalf of or for the benefit of any third party" | same | VERIFIED |
| Restriction item: "runs Maillist, Listserv, any form of auto-responder or "spam" on the Services, or any processes that run or are activated while you are not logged into the Services" | same | VERIFIED |
| Must not "Represent that the Output was human-generated when it was not" | same | VERIFIED |
| Go "designed for OpenCode and other coding agents"; clients should send typical coding-agent traffic, an identifying user agent and a stable `x-opencode-session`; traffic monitored for abuse; limits may change; monthly allowance with 5-hour (20 %) and weekly (50 %) windows; plans US$ 10 / 40 / 20 per member | https://opencode.ai/docs/go/, https://opencode.ai/go | VERIFIED |
| Nothing on the Go or Console pages explicitly allows or forbids marketing drafting or customer support | same pages | VERIFIED (absence) |
| DeepSeek zero-retention arrangement on Go valid through 2026-11-30 (Global region) | https://opencode.ai/docs/go/ | VERIFIED |
| Applying the internal-use, auto-responder and logged-out clauses to an unattended public marketing bot and to a customer-facing support bot | reading of the above | INFERRED |
| Console pay-as-you-go: paid models callable from custom clients; at-cost pricing + processing fee; e.g. Haiku 5.5 US$ 0.10/0.50, DeepSeek V4.1 Flash US$ 0.30/1.20 per 1M tokens | https://opencode.ai/docs/zen/ | VERIFIED |
| DeepSeek API: commercial integration allowed; must disclose AI-generated output; personal data stored in the PRC | https://api-docs.deepseek.com/quick_start/pricing, https://cdn.deepseek.com/policies/en-US/deepseek-open-platform-terms-of-service.html, https://cdn.deepseek.com/policies/en-US/deepseek-privacy-policy.html | VERIFIED |
| Anthropic commercial terms allow serving the customer's end users; no training on customer content; Haiku 5.5 US$ 0.10/0.50 | https://www.anthropic.com/legal/commercial-terms, https://platform.claude.com/docs/en/about-claude/pricing | VERIFIED |
| Gemini API paid tier not used to improve products; 2.5 Flash-Lite US$ 0.10/0.40; free tier allows human review | https://aistudio.google.com/docs/pricing, https://aistudio.google.com/docs/terms | VERIFIED |
| Qwen3 Apache-2.0; Qwen3-8B Q4_K_M ≈ 5 GB file (≈ 6–8 GB RAM in use) | https://huggingface.co/Qwen/Qwen3-8B, https://huggingface.co/unsloth/Qwen3-8B-GGUF | VERIFIED (file) / INFERRED (RAM) |

### 4.2 Affiliate programs

| Finding | Source | Class |
|---|---|---|
| Shopee terms page linked as affiliate terms is titled for participating sellers, updated 2026-08-17, and binds "o Afiliado" | https://help.shopee.com.br/portal/article/76443-Programa-de-Afiliados-da-Shopee---Termos-e-Condi%c3%a7%c3%b5es | VERIFIED (text) / INFERRED (applicability) |
| Shopee §1.2 media must be registered and approved; Telegram/WhatsApp not named | same | VERIFIED |
| Shopee §4.3(d) "O conteúdo não deverá conter links ou direcionamentos para plataformas de comércio eletrônico distintas da Shopee" | same | VERIFIED |
| Shopee §4.4(j) forbids affiliate links without relevant content and duplicate content to the same audience; §4.4(d) no scraping; §4.5 no automatic redirects/iframes/cookie stuffing; §2.2 materials (incl. links) not to be modified | same | VERIFIED |
| Shopee affiliate Open API (offers, short links, conversion reports) documented for Singapore; Brazil GraphQL endpoint exists and rejects unsigned calls | https://help.shopee.sg/10/article/191702-API-Access, `https://open-api.affiliate.shopee.com.br/graphql` | VERIFIED (existence) |
| Shopee BR API credential process, rate limits, sub-ID support | third-party docs only | UNVERIFIED |
| Mercado Livre terms: declared media only; offline excluded; high-volume category names WhatsApp/Telegram groups and sites; no shorteners or link editing; misused software/robot sales unpaid; no comparative ads; only listing information; no bought lists/mass messaging | https://www.mercadolivre.com.br/ajuda/30228 | VERIFIED |
| Mercado Livre electronics/computing/phones commission 5 % direct, 2.5 % indirect, 24 h attribution | https://www.mercadolivre.com.br/ajuda/27913 | VERIFIED |
| Mercado Livre developer API requires OAuth; terms forbid redistributing content outside the app | https://developers.mercadolivre.com.br/pt_br/itens-e-buscas, https://developers.mercadolivre.com.br/pt_br/termos-e-condicoes | VERIFIED |
| Amazon BR: disclosure sentence mandatory; links in email/SMS/DMs only if requested; no hiding shorteners; prices only via link or API data with timestamp rules | https://associados.amazon.com.br/help/operating/agreement, https://associados.amazon.com.br/help/operating/policies | VERIFIED |
| Amazon PA-API 5 deprecated; Creators API needs 10 qualified sales in 30 days; 1 TPS / 8,640 req/day initially | https://affiliate-program.amazon.com/creatorsapi/docs/en-us/paapiv5-deprecation and related Creators API pages | VERIFIED; shutdown date UNVERIFIED |

### 4.3 Channels and law

| Finding | Source | Class |
|---|---|---|
| Bot API: `chat_id` accepts `@channel`; bot posts as admin with `can_post_messages`; `sendMessage` has no idempotency parameter; flood control returns `retry_after` | https://core.telegram.org/bots/api | VERIFIED |
| "Bots can't start conversations with users" | https://core.telegram.org/bots | VERIFIED |
| Limits ≈ 1 msg/s per chat, 20/min per group, ≈ 30/s broadcast | https://core.telegram.org/bots/faq | VERIFIED |
| Bot developer terms: no spam, privacy policy, minimal data | https://telegram.org/tos/bot-developers | VERIFIED |
| WhatsApp terms forbid bulk/auto-messaging and automated access; bans can be automatic | https://www.whatsapp.com/legal/terms-of-service, https://faq.whatsapp.com/5957850900902049 | VERIFIED |
| WhatsApp Business Platform: opt-in, templates outside 24 h, honour opt-outs, human escalation; groups and Channels not supported; AI-provider clause §4.7 | https://whatsappbusiness.com/pt-br/policy/, Meta business onboarding docs, https://www.facebook.com/legal/Meta-Terms-for-WhatsApp-Business-Platform | VERIFIED; Brazil exemption status UNVERIFIED |
| CONAR 2026 guide: affiliate links are advertising; identify with paid-partnership tool or `#publi`/`#publicidade` visible without "ver mais" | https://conar.wpenginepowered.com/wp-content/uploads/2026/05/260525_GUIA_INFLUENCIADORES_CONAR_v6.pdf | VERIFIED |
| CDC arts. 36–38 (identification, misleading by omission, burden of proof) | https://www.planalto.gov.br/ccivil_03/leis/l8078compilado.htm | VERIFIED |
| LGPD bases (consent, legitimate interest), free revocation; ANPD guide accepts promotion to existing relationships with unsubscribe | https://www.planalto.gov.br/ccivil_03/_ato2015-2018/2018/lei/l13709compilado.htm, https://www.gov.br/anpd/pt-br/centrais-de-conteudo/materiais-educativos-e-publicacoes/guia_legitimo_interesse.pdf | VERIFIED |
| PL 2338/2023 awaiting report (as of 2026-11-13) | https://dadosabertos.camara.leg.br/api/v2/proposicoes/2487262 | VERIFIED |
| Instagram content publishing needs a professional account; own-account publishing without App Review | https://developers.facebook.com/docs/instagram-platform/content-publishing, https://developers.facebook.com/docs/graph-api/overview/access-levels | VERIFIED / INFERRED (no App Review) |

## 5. Owner checks that no public page can answer

1. Shopee BR (logged in): the affiliate-side terms and date; whether a Telegram channel
   can be registered as approved media; whether the link tool offers sub-IDs; Open API
   menu, approval, signature docs and limits.
2. Mercado Livre (if fallback): whether the Telegram channel is declared; link tool
   tracking options.
3. OpenCode (account): any notice or limit observed after commerce drafting starts.
4. Telegram: handle availability for the chosen brand.

## F1 implementation evidence

The current implementation report is [COMMERCE_F1_REPORT](../../COMMERCE_F1_REPORT.md).
F1's 48 commerce cases and existing core behavior pass; the full gate passes 273 Rust
tests and 89 Python fixtures. These are synthetic/code acceptance only, not official
affiliate-account, provider-purpose, delivery or revenue evidence. No new external
policy assertion or account permission follows from a test result.
