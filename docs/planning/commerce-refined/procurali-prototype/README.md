# Procurali — Visual Identity and Website Prototype

Responsive Brazilian Portuguese (`pt-BR`) editorial commerce prototype prepared from the [business/site plan](../PROCURALI-BUSINESS-AND-SITE.md) and the [Fable handoff specification](../PROCURALI-FABLE-PROMPT.md). No domain purchase, hosting change, live AI call or production publication is performed here.

## Directory Layout

- `brand/` — [design-tokens.json](brand/design-tokens.json), [tokens.css](brand/tokens.css), editable brand SVGs, Inkscape PNG exports in `brand/png/`, and deterministic creative templates in `brand/templates/`.
- `contracts/` — [cms-schemas.json](contracts/cms-schemas.json) defining `Offer`, `PriceObservation`, `Guide`, `Community`, `CreativeAsset`, `EvidenceSource`, `MediaRights`, and `EditorialApproval`.
- `data/` — [demo-catalog.mjs](data/demo-catalog.mjs) with clearly marked fictional demo fixtures.
- `src/` — [validate-contracts.mjs](src/validate-contracts.mjs), [pages.mjs](src/pages.mjs), [build.mjs](src/build.mjs), [serve.mjs](src/serve.mjs), stylesheet and progressive client script.
- `site/` — generated static prototype output (`29` HTML pages, CSS/JS bundle, brand assets, `sitemap.xml`, `robots.txt`).
- Key documents: [BRAND-GUIDE.md](BRAND-GUIDE.md), [INTEGRATION-CONTRACT.md](INTEGRATION-CONTRACT.md), and [HANDOFF.md](HANDOFF.md).

## Local Commands (Zero-Dependency Node.js)

```bash
# 1. Build static prototype into ./site/
node docs/planning/commerce-refined/procurali-prototype/src/build.mjs

# 2. Run prototype verification tests
node --test docs/planning/commerce-refined/procurali-prototype/test/verify-prototype.test.mjs

# 3. Optional loopback preview server (http://127.0.0.1:4173/)
node docs/planning/commerce-refined/procurali-prototype/src/serve.mjs
```
