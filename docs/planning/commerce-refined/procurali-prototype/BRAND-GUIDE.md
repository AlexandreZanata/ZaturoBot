# Procurali — Visual Identity and Brand Guide (Prototype)

Version: `0.1.0-prototype` (2026-11-15) · Public website and customer-facing copy language: Brazilian Portuguese (`pt-BR`).  
Structured design tokens: [design-tokens.json](brand/design-tokens.json) and [tokens.css](brand/tokens.css).

## 1. Brand Positioning and Wordmark

- **Owner-chosen brand signature:** **Procura Ali**
- **Compact digital wordmark:** **procurali** (lowercase, weight `700`, tight tracking `-0.03em`)
- **Proposed tagline (design proposal pending owner approval):** *“Achados que fazem sentido.”*
- **Visual independence:** never imitate Shopee, AliExpress or other marketplace marks, colors, shopping-cart clichés, mascots or fake trust seals.

## 2. Symbol and Delivered Brand Assets

The symbol combines a discovery/search-pin silhouette (open circular lens + `45°` diagonal tail) with an integrated Coral `#ED6346` verification check inside the bowl — expressing *procura* + *ali* + *informed choice*. Geometry remains legible down to `16×16 px` and in single-color monochrome.

| Variant | Editable SVG | Recommended usage |
|---|---|---|
| Standalone symbol | [symbol-mark.svg](brand/symbol-mark.svg) | Compact UI marks and favicons |
| Primary light | [logo-primary-light.svg](brand/logo-primary-light.svg) | Editorial headers on `#FAF8F3` or `#FFFFFF` |
| Primary dark | [logo-primary-dark.svg](brand/logo-primary-dark.svg) | Dark petrol `#123D3B` covers and footers |
| Monochrome dark | [logo-monochrome-dark.svg](brand/logo-monochrome-dark.svg) | Single-ink print and subtle stamps |
| Monochrome light | [logo-monochrome-light.svg](brand/logo-monochrome-light.svg) | Single-ink reverse surfaces |
| Vector favicon | [favicon.svg](brand/favicon.svg) | Browser tabs and bookmarks |
| Square avatar (`1080×1080`) | [avatar-square.svg](brand/avatar-square.svg) | `@procurali` Telegram and social profiles |
| Social cover (`1200×630`) | [social-cover.svg](brand/social-cover.svg) | Open Graph previews and decks |

### Deterministic PNG Exports (Inkscape CLI)

Generated raster files in `brand/png/`:
- [favicon-16.png](brand/png/favicon-16.png), [favicon-32.png](brand/png/favicon-32.png), [apple-touch-icon-180.png](brand/png/apple-touch-icon-180.png)
- [avatar-512.png](brand/png/avatar-512.png), [avatar-1080.png](brand/png/avatar-1080.png)
- [social-cover-1200x630.png](brand/png/social-cover-1200x630.png)
- [logo-primary-light-800.png](brand/png/logo-primary-light-800.png), [logo-primary-dark-800.png](brand/png/logo-primary-dark-800.png)

Re-export command using Inkscape CLI:

```bash
inkscape brand/avatar-square.svg --export-type=png --export-width=1080 --export-height=1080 --export-filename=brand/png/avatar-1080.png
```

## 3. Color Palette and WCAG Accessibility Checks

| Token | Hex | Role | Required pairing | Contrast ratio |
|---|---|---|---|---|
| `petrol` | `#123D3B` | Brand primary, header/footer, headings, focus ring | Text on `#FAF8F3` | `11.08:1` (AAA) |
| `canvas` | `#FAF8F3` | Warm off-white main editorial canvas | Surface under `#142322` (`ink`) | `15.18:1` (AAA) |
| `muted` | `#4E5A58` | Metadata, observation timestamps, captions | Text on `#FAF8F3` | `6.70:1` (AA+) |
| `coral` | `#ED6346` | Restrained primary-action fill and graphic accent | Pair **only** with `#0D1D1C` (`inkOnCoral`); never white small text | `5.00:1` (AA) |
| `coralText` | `#A8361E` | Accessible terracotta text for kickers on light canvas | Text on `#FAF8F3` | `6.66:1` (AA+) |
| `lime` | `#DCECB0` | Occasional supporting highlight | Text `#123D3B` + icon and explicit text label (never color-only) | `9.32:1` (AAA) |

## 4. Typography, Spacing, Buttons and Disclosures

- **Typefaces:** `'Fira Sans', 'Ubuntu Sans', system-ui, sans-serif` for display/body and `'Fira Mono', ui-monospace, monospace` for timestamps, revisions and IDs.
- **Spacing & grid:** `16 px` base, `4 px` / `8 px` rhythm (`--space-1` through `--space-16`), `1160 px` max container width, `72 ch` max prose line length.
- **Interactive controls:** `44 px` minimum touch targets, continuous visible focus ring (`3px solid #123D3B`, `3px` offset), and `prefers-reduced-motion: reduce` support.
- **Commercial disclosure:** every live merchant CTA must sit immediately beside *“Link de afiliado: podemos receber comissão se você comprar.”* and *“Preço e disponibilidade podem mudar. Confirme as condições na loja.”*

## 5. Creative Templates and Asset Licensing Rules

- Deterministic SVG templates in `brand/templates/`:
  - [feed-card-1080x1350.svg](brand/templates/feed-card-1080x1350.svg) and [feed-card-1080x1080.svg](brand/templates/feed-card-1080x1080.svg)
  - [carousel-5-slides.svg](brand/templates/carousel-5-slides.svg)
  - [story-cover-1080x1920.svg](brand/templates/story-cover-1080x1920.svg)
- **Asset licensing:** all SVGs, design tokens, code and technical line-art illustrations (`product-illustration-*.svg`) in this directory are original Procurali prototype assets.
- **Product imagery rule:** prototype illustrations explicitly carry *“Ilustração técnica demonstrativa — não representa foto real do lote”*. Production product photography requires verified `MediaRights`; generated backgrounds may be decorative only and must never fabricate ports, accessories, certifications or product appearance.
