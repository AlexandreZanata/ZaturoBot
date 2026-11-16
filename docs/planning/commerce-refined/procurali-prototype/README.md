# Procurali — Visual Identity and Website Prototype

Responsive Brazilian Portuguese (`pt-BR`) editorial commerce prototype prepared from the [business/site plan](../PROCURALI-BUSINESS-AND-SITE.md) and the [Fable handoff specification](../PROCURALI-FABLE-PROMPT.md). No domain purchase, hosting change, live AI call or production publication is performed here.

## Directory Layout

- `brand/` — [design-tokens.json](brand/design-tokens.json), [tokens.css](brand/tokens.css), editable brand SVGs, Inkscape PNG exports in `brand/png/`, curated Mercado Livre test product photos in `brand/mercadolivre/`, and deterministic creative templates in `brand/templates/`.
- `contracts/` — [cms-schemas.json](contracts/cms-schemas.json) defining `Offer`, `PriceObservation`, `Guide`, `Community`, `CreativeAsset`, `EvidenceSource`, `MediaRights`, and `EditorialApproval`.
- `data/` — [demo-catalog.mjs](data/demo-catalog.mjs) with fictional state-demo fixtures merged with [mercadolivre-hardware-offers.mjs](data/mercadolivre-hardware-offers.mjs) containing 4 real Mercado Livre affiliate hardware/peripheral test promotions.
- `app/` — Next.js App Router routes (34 public pages: home, offers, offer details, categories, guides, weekly selection, AI finder demo, groups, trust pages, creative kit, CMS mockup and 404).
- `components/` — `ui/` holds the installed [Aceternity UI](https://ui.aceternity.com) components (adapted to the brand palette and to work without JavaScript where they render forms); `site/` holds the shell, offer cards and shared primitives; `pages/` holds the interactive client islands (filters, gallery, AI finder, consent).
- `lib/` — [contracts.mjs](lib/contracts.mjs) (pure contract validators shared by pages and tests), the build-time catalog loader and SEO helpers.
- `scripts/` — [prepare.mjs](scripts/prepare.mjs) resolves the catalog (demo fixtures, a Hub snapshot file or the authenticated Hub API), validates it and writes `public/api/*.json`, `sitemap.xml`, `robots.txt` and the brand pack; [serve.mjs](scripts/serve.mjs) previews the static export.
- `out/` — generated static export (ignored by Git); `public/` generated assets are ignored too.
- Key documents: [BRAND-GUIDE.md](BRAND-GUIDE.md), [INTEGRATION-CONTRACT.md](INTEGRATION-CONTRACT.md), and [HANDOFF.md](HANDOFF.md).

## Stack

Next.js 16 (App Router, `output: "export"` — every route is pre-rendered HTML, no server required), React 19, Tailwind CSS 4, Motion and Aceternity UI components installed through the shadcn CLI registry (`components.json`). Dependencies are pinned in [package.json](package.json) with a committed lockfile. Core content and purchase/disclosure links are present in the HTML without JavaScript; interactivity (filters, saved items, gallery, AI finder demo) is progressive.

## Local Commands

Run from this directory (`docs/planning/commerce-refined/procurali-prototype`):

```bash
npm ci
```

```bash
npm run dev
```

Development server with hot reload on `http://127.0.0.1:4173/`.

```bash
npm run build
```

Static export into `./out/`. Use `PROCURALI_API_SNAPSHOT=path/to/site-catalog.json` (or `PROCURALI_HUB_API_URL` plus `PROCURALI_HUB_TOKEN`) to build from a Hub catalog instead of the demo fixtures.

```bash
npm test
```

Builds and runs the verification suite (pages, disclosures, disabled demo CTAs, contrast, sitemap/robots, Hub snapshot export).

```bash
npm run serve
```

Optional loopback preview of `./out/` on `http://127.0.0.1:4173/`.
