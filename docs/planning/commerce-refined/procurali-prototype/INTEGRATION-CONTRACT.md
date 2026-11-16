# Procurali — CMS and Frontend Integration Contract

Scope: frontend data structures and integration boundaries between future ZaturoBot editorial pipelines and the public Procurali site. This is a contract specification, not a replacement backend database and not an active publisher.

JSON Schema (Draft 2020-12): [cms-schemas.json](contracts/cms-schemas.json).  
Fictional demo fixtures: [demo-catalog.mjs](data/demo-catalog.mjs).  
Pure contract validators: [validate-contracts.mjs](src/validate-contracts.mjs).

## 1. Core Entities

| Entity | Key fields | Integrity & policy rule |
|---|---|---|
| `Offer` | `id`, `slug`, `isDemoFixture`, `title`, `exactVariant`, `categorySlug`, `merchant`, `program`, `officialAffiliateUrl`, `priceBrlCents`, `currency` (`BRL`), `paymentConditions`, `couponCondition`, `freightStatus`, `observedAt`, `validUntil`, `freshnessTtlMinutes` (`30`), `evidenceRefs`, `mediaRightsRef`, `state`, `approvalRevision` | Demo offers (`isDemoFixture=true`) or non-approved states disable purchase CTAs. Production `officialAffiliateUrl` bytes are preserved verbatim with `rel="sponsored noopener"`. |
| `PriceObservation` | `id`, `offerId`, `variantId`, `sellerName`, `paymentBasis`, `freightNote`, `priceBrlCents`, `observedAt`, `evidenceId` | Single observations yield only dated price points. The scoped weekly label *“Menor preço observado pelo Procurali nesta semana”* requires 7 distinct days with identical variant, seller and payment basis. |
| `Guide` | `id`, `slug`, `title`, `metaDescription`, `summary`, `methodologyLabel`, `author`, `reviewer`, `publishedAt`, `updatedAt`, `sourceLinks`, `relatedOfferSlugs`, `state`, `approvalRevision` | Labels desk research visibly so readers never infer hands-on lab tests; `dateModified` advances only on material updates. |
| `Community` | `id`, `slug`, `network` (`telegram` \| `whatsapp`), `purpose`, `categoryTopic`, `intendedFrequency`, `rules`, `availabilityState`, `inviteUrl`, `privacyNotice` | Telegram links to `https://t.me/procurali`. WhatsApp cards require owner-supplied invites and show a pre-join phone/profile visibility notice. |
| `CreativeAsset` | `id`, `offerId`, `format`, `assetPath`, `disclosureText`, `mediaRightsId`, `approvalRevision`, `isIllustrativeDemo` | Binds to shared `offerId` and `approvalRevision` so social cards, scripts and site prices cannot drift. |
| `EvidenceSource` | `id`, `sourceType`, `title`, `publisher`, `sourceUrl`, `retrievedAt`, `verifiedFacts` | External vendor text is treated strictly as untrusted data, never action authority. |
| `MediaRights` | `id`, `assetDescription`, `licenseType`, `rightsVerified`, `ownerPermittedUse`, `attributionText` | `rightsVerified=false` triggers `missing_rights` and blocks active commercial CTAs. |
| `EditorialApproval` | `revision`, `targetType`, `targetId`, `status`, `contentHash`, `mediaHash`, `approvedBy`, `approvedAt`, `blockers` | Any content/media edit or 30-minute price expiry invalidates approval (`invalidated_stale`). |

## 2. Future Server-Side AI Finder Contract (`/encontre-com-ia`)

In this prototype, `/encontre-com-ia` runs `runDeterministicFinder()` locally against demo fixtures and displays **“Demonstração — sem consulta ao vivo”**. Activating live model matching later requires:

1. **Server-side adapter:** no AI provider keys or `@ProcuraAliBot` tokens in browser code.
2. **Budget ceiling & routes:** aggregate **USD 20 per month** ceiling across only the two owner-selected routes (`DeepSeek Flash 4.1` and `Muse Spark 1.3 contributor`, pending exact provider ID verification), no daily call-count limit, and provider `Extra Usage` disabled. Missing usage telemetry or failed reconciliation pauses new model calls.
3. **Catalog grounding & visitor data safety:** bounded inputs/outputs grounded in approved catalog entries; never request CPF, full address, passwords or bank details; treat anonymous visitor prompts as untrusted data and do not reuse an owner-personal-data provider grant for arbitrary visitors.
4. **Required states:** explicitly handle `no-match`, `source-stale`, `service-unavailable` and `budget-cap` without fake typing or fabricated recommendations.

## 3. SEO, Search Console and Event Measurement

- **Demo staging robots policy:** all prototype pages emit `<meta name="robots" content="noindex, nofollow">` and `site/robots.txt` sets `Disallow: /`.
- **Search Console setup note for the owner:** no connected property or verification token is fabricated in this prototype. Before launching on `procurali.com.br`, verify domain ownership in Google Search Console, remove staging `noindex` from canonical public routes (keeping `/cms-preview/`, previews and filter combinations `noindex`), and submit `sitemap.xml`.
- **Product / Offer JSON-LD eligibility:** intentionally omitted on fictional demo offers. Emit `Product`/`Offer` structured data only when real price, `BRL` currency, availability and official URLs satisfy platform eligibility, without ever manufacturing `AggregateRating` or `Review`.
- **Privacy-respecting analytics:** distinguish `outbound_offer_click`, `community_link_click` and `finder_demo_query` asynchronously without blocking direct anchor navigation, without third-party ad pixels, and without inferring attributed affiliate sales from site clicks.
