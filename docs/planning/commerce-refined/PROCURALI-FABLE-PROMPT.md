# Procurali website and visual identity prompt for Fable

Copy the prompt below into Fable. It is self-contained. The instructions are English;
all website copy and customer-facing assets must be Brazilian Portuguese. This handoff
requests design/source preparation, not a purchase, domain change or production launch.
The operating assumptions are defined in the [business/site plan](PROCURALI-BUSINESS-AND-SITE.md).
The resulting non-production prototype and visual identity pack are stored in
[procurali-prototype](procurali-prototype/README.md) with status in the
[handoff report](procurali-prototype/HANDOFF.md).

---

Act as a senior product designer, brand designer, frontend engineer and editorial
conversion strategist. Design and build a complete responsive website prototype and a
coherent visual identity for **Procurali**, the digital expression of **Procura Ali**.
Use Brazilian Portuguese (pt-BR) for the entire public interface. Deliver an original,
usable editorial commerce experience that helps people choose relevant affordable
products and voluntarily follow our offer communities.

## Project facts and delivery boundary

- Brand chosen by the owner: Procura Ali; compact digital wordmark: procurali.
- Domain already owned: procurali.com.br. DNS, hosting and production deployment are
  outside this request. Do not buy a domain, hosting, subscription or additional tool.
- Existing public Telegram channel: https://t.me/procurali.
- Existing bot identity: @ProcuraAliBot. Do not request or embed its token anywhere.
- Shopee affiliate application was submitted and is under review; Telegram was declared
  through Other. Do not claim affiliate/media/API approval or invent affiliate URLs.
- The owner will create WhatsApp groups using their existing personal account. Their
  invite URLs are not yet supplied. Do not create accounts/groups, invent invites or
  automate WhatsApp Web. Initially the website lists approved owner-group invites and
  the owner shares prepared messages manually.
- Future AI usage ceiling: USD 20 per month across only the two owner-selected routes
  described as DeepSeek Flash 4.1 and Muse Spark 1.3 contributor. Exact provider IDs and
  cost telemetry still need verification. No daily call-count ceiling. Subscription
  Extra Usage stays disabled. No paid AI/image/video service or fallback is authorized.
- Create the website identity, pages, reusable UI and clear integration contracts. Do
  not claim that a prototype already has live AI, real-time prices, approved affiliate
  attribution, CMS scheduling or working automation.
- Use the best supported framework in your environment. Prefer server-rendered or
  statically generated public pages when available; otherwise document that limitation.
  Do not conceal unsupported features behind success messages or fake loading animations.

## Business and audience

Procurali is a free affiliate-funded buying guide and evidence-based offer curator.
Readers use original comparisons, buying checklists and supported offers, then purchase
at the merchant. Procurali does not sell inventory, process payments or supply warranties.
Initial audience hypothesis: Brazilian adults looking for practical affordable home/desk
organization and technology accessories around R$ 30–300. Make categories editable; the
owner will choose the first niche before launch.

The primary conversion is a conscious click on a valid official merchant offer. Secondary
conversion is a voluntary Telegram follow or a relevant WhatsApp group join. AI helps
match needs to an approved catalog; it cannot fabricate prices, stock or personal tests.
Optimize for qualified interest, useful decisions and return visits, not misleading clicks.

Proposed tagline: “Achados que fazem sentido.” Show it as a design proposal for approval.
The name must not suggest affiliation with AliExpress, Shopee or another marketplace.

## Visual identity

Create a distinctive, practical Brazilian editorial brand: approachable, organized,
energetic and credible. Use a custom lowercase wordmark “procurali” and a simple symbol
combining discovery/location or a search gesture with a subtle choice/check idea. It must
be legible at favicon size and as a monochrome mark. Avoid generic shopping-cart logos,
mascots, fake trust seals and marketplace logo imitation.

Proposed palette to refine with accessibility checks:
- Deep petrol #123D3B for brand/navigation and high-contrast text.
- Warm off-white #FAF8F3 for the main editorial canvas.
- Coral #ED6346 for a restrained primary-action accent; choose text/background combinations
  that meet contrast requirements rather than assuming this color works for small text.
- Soft lime #DCECB0 for occasional supporting highlights, never as the only status signal.
- Neutral grays for metadata, dividers and secondary surfaces.

Choose a readable modern sans-serif with a slightly distinctive display/wordmark treatment.
Use a clear spacing grid, strong typography, restrained borders and generous breathing
room. Let product evidence and useful explanations dominate, not sale stickers. On mobile,
design for quick scanning and thumb-friendly actions. No overwhelming gradients, flashing
urgency, autoplay video or giant AI-chat widget blocking content.

Deliver logo/wordmark in editable SVG plus light/dark/monochrome variants, square avatar,
favicon, social cover and basic design tokens. If image exports are supported, include
PNG exports at useful sizes; otherwise provide exact export instructions. Define color,
typography, spacing, icon, button, disclosure and image-use rules in a concise brand guide.
Keep asset licensing explicit. Do not fabricate a license or present generated product
photography as evidence of the actual item.

## Site architecture

Build these connected page templates, with a clear navigation structure:

1. Home `/`.
2. Offers `/ofertas` and offer detail `/ofertas/[slug]`.
3. Categories `/categorias/[slug]`.
4. Guides `/guias` and article `/guias/[slug]`.
5. Weekly selection `/selecoes-da-semana`.
6. Guided AI finder `/encontre-com-ia`.
7. Community directory `/grupos`.
8. About `/sobre`, selection method `/como-selecionamos`, affiliate transparency
   `/transparencia`, privacy `/privacidade`, terms `/termos`, contact `/contato`.

Keep the initial navigation concise: Ofertas, Guias, Encontre com IA, Grupos. Categories
can be a mobile-friendly secondary menu. Include brand/home, search and saved items if
you can implement them honestly. Public discovery must work without signup.

### Home

Use a compact editorial hero with headline “Encontre o que vale a pena para você.”
Support text for the prototype: “Ofertas e guias para escolher com mais contexto.”
Primary CTA: “Explorar ofertas”. Secondary CTA: “Receber achados no Telegram”.

Below the hero show a small curated set, category shortcuts, an explanatory buying-guide
feature, a compact AI-finder introduction and one community section. Add a plain “Como
o Procurali seleciona” explanation: research, evidence, tradeoffs and affiliate funding.
Avoid a wall of identical product cards. Mix a featured guide, compact offer rows and a
useful comparison module. No fake visitor, sales or savings counters.

### Offer listing and detail

Cards show the exact item/variant, merchant, verified observation time, relevant condition,
one-line reason to consider it and a clear CTA. Optional price/history labels must be
derived from real supplied evidence. Filters include category, price band and supported
conditions. Sorting states its basis, such as “Menor preço informado”; do not imply
delivered-price sorting when shipping is unknown.

Detail layout: accurate image and variant, price and conditions, “para quem faz sentido”,
supported benefits, compatibility/limitations, evidence and update time, official source,
optional price observations and alternatives. Affiliate disclosure sits beside the CTA:
“Link de afiliado: podemos receber comissão se você comprar.”
Add “Preço e disponibilidade podem mudar. Confirme as condições na loja.”
Do not guarantee unchanged checkout price or universal coupon/free-shipping eligibility.

“Ver oferta na Shopee” must use the exact approved official affiliate URL as its href.
Never shorten it, add home-made tracking parameters, rewrite its bytes or introduce
automatic redirects/cookie stuffing. Mark affiliate anchors `rel="sponsored noopener"`
when opening a new tab. A lightweight analytics event must not prevent navigation.
Show meaningful expired, stale, missing-rights and unavailable states, with no active
purchase action for a demo or invalid offer.

Do not use “menor preço da internet”, “últimas unidades”, “só hoje” or countdowns without
evidence. A scoped historical claim requires a sufficiently documented seven-day sample,
matching variant/payment/seller scope and disclosed coverage. Otherwise show a dated price
observation. No invented rating stars, seller scores or reviews.

### Guides and weekly selection

Use a readable editorial template with author/reviewer, real publication/update dates,
summary, contents navigation, buying criteria, comparison, useful tradeoffs, sources,
methodology and disclosures. Label desk research when a reader might infer hands-on use.
Never invent product tests, quantitative performance measurements or author credentials.
Use a natural article title and useful headings, without keyword repetition.

The weekly page shows a real date range and selection scope with a small number of
meaningfully different choices. “Seleção da semana” is appropriate; an unsupported
universal “melhores e mais baratos” promise is not. Keep expired historical prices
distinct from current offers. Prefer updating a canonical guide to creating daily clones.

### Guided AI finder

Design an approachable guided form, not an enormous empty chat box. Ask what the visitor
needs, maximum budget and a few relevant requirements. Example interface prompts:
“O que você precisa resolver?”, “Quanto pretende gastar?”, “O que não pode faltar?”
Do not request CPF, bank details, passwords, a full address or account credentials.

Results contain at most a small useful shortlist, with catalog source/time, fit reasons,
tradeoffs and unknowns. Support no-match, source-stale, service-unavailable and budget-cap
states. Do not fabricate a response or show typing as though a model ran.

For this prototype, use a clearly labeled local deterministic demonstration with demo
catalog data, or an explicit unavailable state. It must say “Demonstração — sem consulta
ao vivo”. Future model calls need a server-side adapter, secret storage, grounded catalog,
bounded input/output, abuse controls and admission against the aggregate USD 20 ceiling.
Unknown usage/failed reconciliation pauses new model calls. Do not place provider keys
or the Telegram token in browser code or request them in chat. Do not assume an
owner-personal-data grant covers arbitrary visitors' free-form personal data.

### Community directory

Create a friendly directory limited to the owner's approved communities, not an internet
scraper or public directory submission service. Each card includes network, purpose,
topic/category, intended frequency, basic rules, availability and one clear join CTA.
Users choose voluntarily; no automatic joins, notifications, DM opt-ins or contact export.

Use the real Telegram link https://t.me/procurali. WhatsApp cards can be preview layouts
only until the owner supplies actual invite URLs; they must not contain invented active
links or false “grupo cheio” counts. Explain before joining that a WhatsApp group's
configuration may expose phone/profile information. Do not promise anonymity.
Support invalid/revoked/full group links and owner-controlled removal/rotation. Do not
collect member lists or infer the right to send DMs from a group visit.

### Trust and contact pages

Explain that affiliate funding may generate commission and that selection includes
limitations. Distinguish Procurali from the merchant responsible for purchase, payment,
delivery and product support. Privacy/terms must describe the actual planned behavior
and remain owner-review drafts until deployment details are known. Do not invent a
company ID, address, support email, legal signoff, certification or customer testimonial.
The contact form is demo-only until a real backend/destination is configured; no false
“enviado com sucesso” for an unconnected form.

## Editorial system and creative kit

Design CMS-ready structures for Offer, PriceObservation, Guide, Community, CreativeAsset,
EvidenceSource, MediaRights and EditorialApproval. Reuse shared identifiers/revisions and
avoid separate conflicting copies of prices/claims across formats. This is a frontend
integration contract, not a new replacement database for the ZaturoBot backend.

An offer includes exact variant, program, official URL, price/currency, payment/coupon/
shipping conditions, observed_at, valid_until if known, evidence references, media rights,
state and approval revision. A guide includes slug, summary/body, author/reviewer, dates,
source links, related offer IDs and draft/review/approved/expired state.

Provide reusable visual templates for:
- One 1080×1350 feed offer card and 1080×1080 adaptation.
- A five-slide carousel: need → criteria → choice/tradeoff → condition → CTA.
- A 1080×1920 Story/vertical-video cover with safe text zones.
- A short-video storyboard layout with disclosure, subtitles and one CTA.
- Telegram post formatting and a manual WhatsApp digest/export layout.

Style: natural Brazilian copy, specific benefits, occasional relevant emojis, one primary
CTA, visible #publi or equivalent advertising identification. Examples include “🔎 Vale
conferir para quem precisa de [uso]” and “⚠️ Confira a compatibilidade com [requisito]”.
“Imperdível” is sparing editorial emphasis that needs a real reason; do not fabricate
urgency. Insert exact text/price with deterministic layout over approved/licensed assets.
Generated backgrounds can be decorative; they cannot misrepresent the advertised product.

Use YoutubeBot only as a conceptual reference for separate brief, script, narration,
artwork, metadata, render and publish stages, with resumable records and exact approvals.
Do not run it, alter its channels, copy browser profiles or assume its existing long-form
profiles already support Procurali shorts. Native publishing is future integration work.

Daily editorial work is a goal, not permission to mass-publish low-value AI pages. Plan
for a draft/review queue, evidence freshness and exact text/media approval before a future
publisher can run. Price expiry must block stale scheduled offers. No real scheduling,
Telegram send or WhatsApp automation is part of this prototype.

## SEO, accessibility and performance

Public pages need useful HTML, unique title/meta description, one meaningful primary
heading, canonical URL, descriptive internal links, social metadata, sitemap and correct
status codes. Keep preview, internal search/filter explosions and private admin out of
the index; noindex is not access control. Use stable semantic routes and meaningful 404s.

Use accurate Organization, Article and BreadcrumbList structured data. Add Product/Offer
only when the supplied data and page eligibility meet the actual feature requirements.
Never manufacture AggregateRating, Review or merchant-sale data. No guarantee of rankings,
rich results or organic reach. Include an owner setup note for Search Console, not a
fabricated connected property or verification token.

Design and test at small mobile, tablet and desktop sizes with no horizontal overflow.
Use semantic controls, keyboard navigation, visible focus, readable contrast, form labels,
clear errors, useful alt text, reduced-motion support and touch-friendly targets. Optimize
images, reserve layout space and avoid blocking video. Do not display unmeasured “100/100”
performance/accessibility badges. Public content must be useful without a JavaScript-only
AI interaction. Explain actual hosting/runtime limits if server rendering is unsupported.

Analytics should distinguish on-site outbound clicks, community-link clicks and search
traffic; none proves a sale or group join. Actual affiliate reporting is a separate
official integration. Minimize stored visitor data, do not retain free-form AI prompts by
default, and provide appropriate choices for any non-essential analytics/storage. No
advertising pixels or third-party scripts silently added to the prototype.

## Implementation quality and demo data

Use reusable components and design tokens, not screenshots of UI. Navigation, category/
price filters, sorting, drawers, accordions and local demo finder behavior must work if
shown as interactive. Do not leave decorative dead buttons. If an action is unavailable,
label/disable it honestly and explain what integration it requires.

Keep demo catalog items, prices and claims clearly marked as fictional and separate from
production content. Demo offers cannot lead to a real purchase. No fake affiliate or
WhatsApp links, analytics totals, testimonials or time-sensitive promotions. Render the
real Telegram link normally. The public production launch requires owner-approved media,
real offer evidence/rights, privacy wording and reviewed deployment separately.

Do not expose admin endpoints or create an unauthenticated publishing API. A CMS/admin
mockup can be shown separately, clearly labeled non-production, but do not claim it is
secured merely because its link is hidden. Live account login, terms acceptance, form
submission, payment and production publishing remain owner actions.

## Deliverables and acceptance

Deliver the brand guide/assets, responsive website/source in your supported export form,
reusable components and creative templates, demo fixtures, content/data integration
specification and a concise handoff listing connected versus demo/future features.

Check before delivery:
1. Home, offer detail, guide, finder and community directory are coherent across mobile/
   desktop and all visible navigation reaches the intended page.
2. Affiliate disclosure is visible next to the relevant action; no fabricated live claim.
3. Exact official URLs can be preserved; demo purchase and WhatsApp actions are disabled.
4. No credentials, paid services, auto-sends, live scheduling or forced signups appear.
5. Filters and demo finder do what their labels say, including no-match/expired states.
6. Metadata is page-specific, structured data is truthful and demo staging stays noindex.
7. Keyboard/focus/contrast and image layouts were actually checked; report limitations.
8. Brand exports, component/field descriptions and remaining owner inputs are supplied.

Start with the visual system and home composition, then build the remaining connected
templates. Make routine design decisions yourself and clearly mark substantive unresolved
integrations. Return a reviewable prototype; do not publish or purchase anything.
