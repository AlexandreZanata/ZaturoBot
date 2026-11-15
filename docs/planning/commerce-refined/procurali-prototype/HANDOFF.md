# Procurali — Prototype Handoff Report

Date: `2026-11-15` · Scope: visual identity, responsive `pt-BR` static website prototype, reusable creative kit, demo fixtures and CMS integration contracts in `docs/planning/commerce-refined/procurali-prototype/`.

## 1. Delivered Prototype vs. Demo / Future Features

| Capability | Status in this handoff | Honest boundary |
|---|---|---|
| Visual identity, SVGs & PNGs | **Delivered** | Custom `procurali` wordmark, symbol, favicon, square avatar, social cover, design tokens and [brand guide](BRAND-GUIDE.md). |
| Responsive static `pt-BR` pages | **Delivered (29 HTML pages in `site/`)** | Zero-dependency Node.js SSG (`src/build.mjs`); core content works without JavaScript, enhanced progressively by `app.js`. |
| Filters, explicit sorting & saved items | **Delivered (demo data)** | Filters by category, price band, condition, search and state; `Salvos` uses opt-in local `localStorage` only. |
| Official Telegram channel `@procurali` | **Real active link** | Links directly to `https://t.me/procurali` across Home, footer and `/grupos`. |
| Shopee purchase & WhatsApp groups | **Disabled / honest preview** | Shopee affiliate application is under review and WhatsApp invites are not yet supplied; zero invented links or fake counts. |
| Guided AI finder (`/encontre-com-ia`) | **Local deterministic demo** | Displays *“Demonstração — sem consulta ao vivo”* and supports `match`, `no-match`, `source-stale`, `service-unavailable` and `budget-cap`. |
| Contact form (`/contato`) | **Disabled demo layout** | No backend destination connected; submit button stays disabled without any false success message. |
| CMS queue mockup (`/cms-preview`) | **Non-production mockup (`noindex`)** | Visual contract preview only; exposes no admin API or unauthenticated publishing endpoint. |
| Live scheduling, Shopee API, AI calls | **Planned (outside prototype)** | Requires future ZaturoBot backend integration, verified rights/evidence, and explicit owner deployment approval. |

## 2. Verification Results and Known Limitations

- **Automated verification suite (`test/verify-prototype.test.mjs`):** 8/8 checks passing — validates all 29 HTML pages, single `<h1>`, unique title/description, staging `noindex`, internal link integrity, affiliate disclosures beside offer actions, URL byte-preservation helper, disabled demo/WhatsApp CTAs, real Telegram link, filter/7-day/freshness/AI-finder logic, absence of forbidden claims, WCAG contrast ratios (`>= 4.5:1` across all active text pairings), valid PNG headers, and CMS schema definitions.
- **Visual SVG/PNG QA:** rendered brand assets (`logo-primary-light-800.png`, `logo-primary-dark-800.png`, `avatar-512.png`, `social-cover-1200x630.png`, `favicon-32.png`) and creative templates (`feed-card-45.png`, `feed-card-11.png`, `carousel-5.png`, `story-cover.png`) were exported via Inkscape and inspected visually to prevent text clipping.
- **SSR & environment limitations:** public pages use static site generation (SSG) rather than a live Node/Rust SSR server; dynamic server routes (`/encontre-com-ia` live inference, authenticated CMS approvals, live price freshness rechecks) require future server deployment. Live multi-viewport browser interaction was not run in a headless browser session; responsiveness rules (`overflow-x: clip`, `44px` touch targets, `360px`/`768px`/`1200px` breakpoints) were verified in CSS and HTML structure.

## 3. Remaining Owner Inputs Before Production Launch

1. Approve or refine the proposed tagline (*“Achados que fazem sentido.”*) and select the first launch niche.
2. Confirm Shopee affiliate approval (including website media registration), supply real official affiliate URLs, and verify `MediaRights` for actual product photos.
3. Create personal-account WhatsApp groups if desired and supply real invite links to replace preview cards on `/grupos`.
4. Review draft Privacy Policy and Terms wording, establish a real support email/contact handler, and approve DNS/TLS/hosting for `procurali.com.br` separately.
5. Verify exact provider model IDs (`DeepSeek Flash 4.1` and `Muse Spark 1.3 contributor`) and budget-month boundaries (`America/Cuiaba`) before activating the server-side AI finder adapter.
