# Procurali business model and website plan

Status: owner-requested planning specification, 2026-11-15. No production implementation,
account creation, paid generation, scheduling or publication is activated by this document.
Owner facts and budget decisions live in [open questions](../../OPEN_QUESTIONS.md).
The earlier [refined plan](ZATURO-COMMERCE-REFINED-PLAN.md) remains the implementation
baseline; this document proposes the creative, discovery and website extension.
Use the [creative playbook](PROCURALI-CREATIVE-PLAYBOOK.md) for production briefs, the
[Fable prompt](PROCURALI-FABLE-PROMPT.md) for the website/design specification, and the
[Procurali prototype](procurali-prototype/README.md) ([handoff](procurali-prototype/HANDOFF.md))
for the prepared non-production visual identity, static HTML prototype and contracts.
Documentation is English; quoted customer-facing copy is Brazilian Portuguese.

## 1. Business proposition

Procurali is an affiliate-funded buying guide and offer curator for Brazilian shoppers.
Its promise is to reduce the work of choosing: explain who a product suits, what to check,
what it currently costs and where to buy it. The owner does not become the merchant,
payment processor or warranty provider. The merchant remains the seller.

Proposed brand signature: **Procura Ali**. The compact digital wordmark is **procurali**.
Proposed tagline: **“Achados que fazem sentido.”** This extends the chosen name; the
tagline and visual direction remain proposals for owner review. It must look independent
of Shopee and AliExpress, without copying their marks or implying official affiliation.

Launch audience hypothesis: adults looking for useful, affordable technology accessories
and home/desk organization, initially around R$ 30–300. Start with one editorial cluster
and one affiliate program rather than every category at once. The niche and price band
still need owner selection. Expand only when attributed results justify the work.

The offer is free for readers. Initial revenue comes from approved affiliate commissions.
No inventory, checkout, paid traffic or paid membership is required for the pilot.
Later sponsorships would need clear labeling and a separate owner decision. Commission
rate must not determine the product-quality ranking; disclose commercial relationships.

## 2. Acquisition and retention

The proposed path is: a useful search result or social video leads to a relevant guide
or offer, the visitor compares the options, voluntarily follows an official affiliate
link or joins a topic-specific community, and returns for useful updates.

- Search: answer narrow buying questions and keep evergreen guides accurate.
- Shorts/Reels: demonstrate one buying consideration, limitation or use case in a brief
  vertical video; use a destination supported by that platform.
- Instagram/Pinterest: adapt a comparison or checklist into saved/shareable visual content
  after the owner confirms existing accounts and media eligibility.
- Telegram: @procurali is the confirmed public channel. Propose scheduled useful offers
  and a weekly digest after integration, policy approval and publication acceptance.
- WhatsApp: list only owner-created groups the owner deliberately chooses to expose.
  Joining is voluntary. Provide an export/copy workflow for human sharing initially.
- Earned distribution: share helpful answers where community rules permit and the owner
  already participates. No unsolicited DMs, automatic community joining, purchased members,
  scraped contact lists, comment floods, backlink exchanges or copied competitor posts.

This is a zero-paid-media strategy, not a zero-cost operation. Hosting, domain renewal,
generation, rendering, electricity and owner review time are costs. Reach and revenue
are outcomes to measure, not promised results.

## 3. Revenue, expenses and measurement

Use these identities without inserting made-up conversion or commission rates:

- Approved affiliate revenue = sum of approved commissions in the official report.
- Operating contribution = approved revenue minus measured direct operating costs.
- Revenue per affiliate click = approved attributed commission / attributed clicks,
  only when both cover the same cohort/window and clicks are nonzero.
- Offer click rate = observed outbound clicks / observed offer impressions, only if
  both are measured under a stated method. Site clicks are not affiliate-attributed sales.

Pending, approved, canceled and paid commissions remain separate. Missing reports mean
unknown, never zero. Keep BRL revenue and USD costs separate until applying a documented
exchange rate and date; do not invent current rates. No income forecast is asserted.

Measure search impressions/clicks through owner-configured Search Console; site visits
and outbound clicks through a privacy-reviewed setup; channel joins only where actually
observable; attribution through the affiliate program's own supported reports/sub_ID.
Do not modify official affiliate URLs with home-made tracking parameters or redirectors.
Site click analytics must not block the direct anchor navigation or expose identifiers.
Never display a WhatsApp group member count, Telegram post view count or conversion
number that the integration does not actually supply.

At pilot reviews, compare eligible clicks, approved commission, direct cost, content
corrections, unsubscribes/complaints where available, and owner time. A useful early
result is accurate, maintainable content and attributable interest. Scale only after
repeatable publishing quality; no promise of first-page rankings or viral distribution.

## 4. AI budget and accounting

The owner's current decision supersedes the earlier percentage/call-count proposal:
**USD 20 per month combined**, with no call-count ceiling, for only the two models the
owner described as DeepSeek Flash 4.1 and Muse Spark 1.3 contributor. Exact provider
identifiers and tariff versions remain unverified. OpenCode Go stays subscription-only;
Extra Usage stays off and no credit fallback or automatic top-up is introduced.

The owner wants dollar-valued usage returned by OpenCode as the accounting reference.
Treat this as a budget metric, not evidence of an additional cash payment. Before
implementation, verify whether each returned field is actual charge, reference token
valuation or subscription usage; record the meaning and source. Never count both the
reported value and a token-price estimate for the same request. If a model has no
usable reference tariff or the usage cannot be reconciled, pause that route.

Proposed enforcement: one aggregate durable monthly ledger for research, copy, metadata,
public recommendations and any other commerce inference on those two routes. Reserve a
conservative upper bound before each request, settle actual usage, and retain uncertain
reservations. Admission includes reserved plus settled usage. No race may overspend the
ceiling. Use integer money units; keep UTC records and an explicit budget-month boundary
in America/Cuiaba. This monthly boundary is a proposal to confirm before coding.

At the cap, pause model-driven content/recommendations. Existing valid static content
can remain available; a manually reviewed deterministic template may still be prepared.
No model substitution. Having no call-count limit does not remove bounded inputs,
timeouts, concurrency, output limits, idempotency or the remaining monetary ceiling.
Paid image/video services or Codex media are not automatically funded by this budget.
Start with licensed assets and deterministic layouts; separately approve any extra
generation route and its accounting before use.

## 5. Offer research and evidence

AI organizes and compares evidence; it is not the source of prices or product tests.
Discovery may consult permitted public sources and official manufacturer material.
Transactional offers come from owner exports or approved affiliate APIs. The previous
Shopee source restrictions remain: no automated marketplace scraping, browser-session
copying or extraction bypass. A webpage cannot grant action permissions.

For each candidate retain: program/product/variant identifiers, seller, exact official
link, observed price/currency, payment conditions, coupon eligibility, freight status,
stock/expiry if known, observation time, allowed source, evidence references and asset
rights. Freight may depend on destination: never present a total delivered price or free
shipping without the required evidence. Do not request a full address for discovery.

Rank eligible candidates using a published editorial method: fit for the use case,
verified price value, supported specifications, seller evidence, limitations and data
freshness. Record why a candidate was excluded. Do not fabricate numerical performance
ratings or call a desk-research comparison a physical test. Prefer “seleção da semana”
or “opções para [uso]” when no first-hand evidence supports “melhor”.

Proposed freshness policy: priced content requires a check immediately before its slot,
with a maximum observation age of 30 minutes; changeable inventory still cannot be
guaranteed. If no permitted recheck exists, withhold the live-price claim, show its dated
historical observation or postpone. Do not promise permanent real-time coverage.

“Menor preço observado pelo Procurali nesta semana” requires a defined rolling seven-day
sample, exact matching variant/seller/payment basis, dated observations, disclosed sample
coverage and no hidden freight differences. Sparse samples do not support a week-wide
minimum. At launch, prefer “encontrado por R$ … em [data/hora]”. A genuine coupon expiry
can support urgency; invented stock, countdowns and percentages cannot.

## 6. Public website

The proposed domain is procurali.com.br, reported owned by the owner. DNS/TLS/hosting
and operational status are unverified. The site is a guide and community hub; purchases
are completed at the identified merchant. No account is needed for public discovery.

### Navigation and page purposes

- `/`: concise proposition, a few current verified picks, category navigation, a buying
  guide, an AI-finder entry and separate community CTAs. Prioritize one primary action.
- `/ofertas`: filter by supported category, budget and verified conditions; sort labels
  explain their basis. Expired or stale items cannot look like live offers.
- `/ofertas/[slug]`: exact variant, price/time/conditions, official source, intended use,
  advantages/limits, price evidence where available and “Ver oferta na Shopee”. Prominent
  affiliate disclosure sits beside the purchase link, not only in the footer.
- `/categorias/[slug]`: useful curated category introduction and eligible current offers;
  no thousands of near-identical keyword pages.
- `/guias` and `/guias/[slug]`: original buying criteria, practical comparisons, sources,
  author/reviewer, published/updated dates and update history when material.
- `/selecoes-da-semana`: weekly selection with date range, selection criteria and current
  availability; do not claim it covers every internet offer.
- `/encontre-com-ia`: guided selection by need, budget and constraints, with reasons and
  evidence timestamps for a small set of eligible catalog matches.
- `/grupos`: curated owner-group directory with network, topic, intended posting
  frequency, rules and a clear voluntary join action. Telegram is @procurali. WhatsApp
  invite URLs are provided by the owner only after groups exist and are approved for
  public discovery. Do not invent invites or auto-enroll visitors.
- `/sobre`, `/como-selecionamos`, `/transparencia`, `/privacidade`, `/termos`, `/contato`:
  ownership/editorial method, affiliate funding, data handling and useful support routes.
  Legal text needs owner review for the actual deployment; no fake company or address.

Group cards must distinguish Telegram channels from WhatsApp groups, and make frequency
and purpose visible before joining. Explain that joining a WhatsApp group may expose
profile/phone information according to its configuration. Do not promise anonymity.
Support full, invalid/revoked and temporarily unavailable invites. The owner rotates
compromised invites and removes abusive groups. No public third-party group submission
or crawling of directories in the first version. Do not export members or add them to DMs.

### Useful persuasion

Make benefits specific, reduce the number of decisions, put relevant evidence beside the
CTA, explain product limitations and offer a simple next step. Pair a primary merchant
CTA with a secondary community action without obstructing the first. Use consistent CTA
labels and compare truthful variants through measured experiments. Social proof appears
only if verified and properly attributed. No fake reviews, fake viewer counts, resetting
countdowns, forced notification consent or “remaining places” claims without evidence.

Proposed hero copy: “Encontre o que vale a pena para você.” Supporting copy:
“Ofertas verificadas, guias diretos e escolhas explicadas para comprar com mais contexto.”
Primary CTA: “Explorar ofertas”. Secondary CTA: “Receber achados no Telegram”.
Use “verificadas” only once the actual source/freshness process is operating; a prototype
must use “Ofertas e guias para escolher com mais contexto”.

### AI finder behavior

Ask for use case, maximum price and a few product constraints. Match against an approved
catalog first, showing why each option fits and what is unknown. If no match exists,
explain the gap or offer a normal filtered search. Do not invent live internet research
or a streaming answer when the model is not connected. Anonymous free-form input is
untrusted; separate it from policy and never pass browser cookies or private credentials.

Future backend requirements: server-side keys, bounded queries, catalog grounding,
abuse/rate controls, aggregate USD 20 admission, safe link allowlisting and secret-safe
logs. Apply short output and concurrency bounds without inventing a daily model-call
budget. Prefer ephemeral query processing; no account, chat history or personal profiling
in the pilot. Select provider/data handling before deploying the public feature. Do not
reuse an owner-personal-data provider grant for arbitrary third-party visitor data.

### SEO and web quality

Use server-rendered or statically generated indexable public content, semantic headings,
unique titles/descriptions, canonical URLs, crawlable internal links, sitemap and correct
status codes. Internal search/filter combinations and previews should not generate an
indexable page explosion. `noindex` is not authentication: keep admin routes protected.
Create original guides that explain tradeoffs rather than rewriting merchant listings.
Google's [AI-content guidance](https://developers.google.com/search/docs/fundamentals/using-gen-ai-content)
emphasizes accuracy and added value, not a daily publishing quota.

Affiliate anchors use `rel="sponsored noopener"` when opening a new tab; the href retains
the exact approved program URL. Google documents
[outbound-link qualification](https://developers.google.com/search/docs/crawling-indexing/qualify-outbound-links).
Use Article, BreadcrumbList and Organization structured data where accurate. Product/Offer
markup needs valid eligible data matching the visible page; no invented AggregateRating,
Review, price history or merchant ownership. Structured data does not guarantee rich results.

Optimize product imagery, avoid autoplay-heavy video, reserve image dimensions, lazy-load
below the fold and make mobile navigation/filters work by keyboard and screen reader.
Use readable contrast, visible focus, semantic forms and reduced-motion preferences.
Treat accessibility and performance measurements as launch checks, not decorative badges.
Analytics/cookie choices and public privacy wording must match actual deployed behavior.

## 7. Editorial cadence and organic distribution

Proposed starting cadence, subject to evidence and owner policy approval:

- Daily: research/update queue; target one useful guide or material guide update, two
  Telegram opportunities at the earlier suggested Cuiaba windows, and one creative batch.
- Weekly: three original short videos, two carousels, one selection/digest; reuse their
  underlying research instead of spending on independent prompts for every platform.
- WhatsApp: prepare a reviewed digest for manual sharing; start around three digests per
  week per relevant group and review audience feedback. Automated group sending remains
  conditional and inactive.
- If quality, fresh offers, rendering capacity or the monthly budget are insufficient,
  reduce output. A daily editorial task may correctly conclude “nothing worth publishing”.

First seven editorial themes: buying criteria for the initial category; compatibility
mistakes; three alternatives for distinct budgets; price versus delivered price; useful
accessories by scenario; coupon conditions; weekly evidence-based selection. Adapt to
the niche the owner selects. Avoid cannibalizing the same search query with daily clones.

## 8. Reuse and automation boundaries

YoutubeBot was inspected at revision `ab2885a9f6ad40f4a0904f0b8d8ecb8724b7b8d4`:
[component map](https://github.com/AlexandreZanata/YoutubeBot/blob/ab2885a9f6ad40f4a0904f0b8d8ecb8724b7b8d4/docs/ARCHITECTURE.md),
[media tasks](https://github.com/AlexandreZanata/YoutubeBot/blob/ab2885a9f6ad40f4a0904f0b8d8ecb8724b7b8d4/docs/CODEX-MEDIA.md),
[service contract](https://github.com/AlexandreZanata/YoutubeBot/blob/ab2885a9f6ad40f4a0904f0b8d8ecb8724b7b8d4/docs/SERVICE.md).
Reuse its separation of briefs, assets, metadata, render records and generation-only jobs,
plus durable operation identity, readiness checks and reconciliation of uncertain effects.
Preserve original channels, profiles, extension, voice and upload behavior. A Procurali
vertical-video profile and affiliate-specific checks are new work, not existing acceptance.

The repository contains native Codex media tasks and legacy browser-based media/upload
paths; it does not establish that those routes are eligible for commerce. Current
Zaturo commerce planning excludes copying browser-driven commercial publication. Prefer
reviewed permitted generation and official publishing integrations. Do not borrow signed-in
profiles, run setup commands that copy cookies or silently adopt its existing channels.

Planned pipeline: source import → evidence/rights validation → eligible selection →
fact-locked creative brief → reusable layout/video generation → editorial/price check →
exact owner batch approval → channel-specific publish → receipt/status → commission report.
Text, media and destinations are bound to the approved version/hash. Editing them requires
new approval. Missed or uncertain posts are not resent blindly. An offer becoming stale
invalidates publication even if yesterday's draft was approved. Provide stop/pause controls.

Telegram automation uses the official Bot API in the bounded commerce implementation
after readiness/rights verification. Public-channel smoke is the owner's explicit choice
from setup, but a specific live test still needs its reviewed payload and authorization.
Website article scheduling requires its own authenticated publishing interface and grant.
Other social networks require existing-account checks, approved media and supported APIs;
a schedule proposal is not permission to publish to every account.

For the owner's personal WhatsApp groups, initially implement directory links and a
manual-share package only. The current [WhatsApp terms](https://www.whatsapp.com/legal/terms-of-service)
restrict impermissible auto/bulk messaging and unauthorized non-personal use. An official
[Groups API exists](https://developers.facebook.com/documentation/business-messaging/whatsapp/overview),
but this does not establish eligibility for, or access to, personal-account groups. An
official route needs an independent compatibility/eligibility/cost review and owner
decision before implementation. No WhatsApp Web bot or unofficial sending workaround.

## 9. Delivery sequence and acceptance

1. **This delivery:** business/site specification, creative instructions and Fable prompt,
   with evidence and unresolved decisions recorded. No production code changes.
2. **Design prototype:** owner uses the Fable prompt. Accept identity, responsive page
   layouts, truthful demo labels, real navigation, usable filters and accessible states.
   No production integration or live offer/AI promise is part of a static prototype.
3. **Commerce foundation:** review/integrate the isolated commerce work under a separate
   code task; verify budget accounting, model IDs, offer imports, approval and pause/stop.
   Do not embed API tokens in a public site or ship the hub's admin interface to visitors.
4. **Editorial vertical slice:** one evidenced offer → text/graphic/short draft → owner
   preview. Require licensed assets, correct variant and disclosure, intelligible audio,
   legible subtitles, decoded final media and an exact manifest; no publication yet.
5. **Website slice:** authenticated CMS feed → one reviewed guide/offer → direct official
   link → observable click event. Test stale/expired/no-match states and public/backend
   authorization. Owner approves domain deployment separately.
6. **Limited launch:** affiliate/media approval, one authorized public Telegram test,
   validated receipts, then gradual owner-approved recurring policy. No implied bulk grant.
7. **Expand:** add organic networks and the grounded AI finder after measured demand and
   budget/abuse checks. Revisit WhatsApp official automation only as a separate decision.

## 10. Remaining owner choices

Choose the first niche and budget band, confirm the calendar and budget-month boundary,
approve identity/tagline, provide existing-account status for each proposed network,
confirm which owner WhatsApp groups may be listed, and obtain affiliate approval for each
actual medium including the website. Exact model IDs, source entitlements, artifact
licensing and production publication remain implementation gates. Site creation through
Fable is a handoff, not an instruction for this agent to purchase hosting or deploy.

## 11. Dated policy evidence

Consulted 2026-11-15; platform terms can change and must be rechecked before activation.

- [Shopee affiliate terms](https://help.shopee.com.br/portal/10/article/124094-Programa-de-Afiliados-da-Shopee-Termos-e-Condi%C3%A7%C3%B5es):
  affiliate media require approval; the program governs links and access to platform assets.
  Submission through Other is not proof of approval. Article 76443 in the old runbook is
  titled for the participating-seller program; it must not be treated as the sole contract
  for the owner's affiliate enrollment. Applicable account terms still need owner review.
- [CONAR's 2026 guide announcement](https://www.conar.org.br/noticias/guia-de-marketing-e-publicidade-por-influenciadores-digitais):
  the revised guidance covers transparent identification, truthful testimonials and AI use.
- [Google review guidance](https://developers.google.com/search/docs/specialty/ecommerce/write-high-quality-reviews):
  evidence, comparison and reader value matter. Desk research must not imply physical use.
- [YouTube link guidance](https://support.google.com/youtube/answer/13748639?hl=en-BR):
  URLs in Shorts descriptions/comments are not clickable; use an available profile-link
  path or supported related video, without promising direct description-link clicks.
- [YouTube promotion disclosure](https://support.google.com/youtube/answer/154235?hl=pt-BR):
  applicable commercial relationships need platform disclosure in addition to visible copy.
- [WhatsApp Business policy](https://business.whatsapp.com/policy): business messaging
  has consent, opt-out and product-specific requirements. A group invite is not consent
  to scrape member data or contact members privately.
