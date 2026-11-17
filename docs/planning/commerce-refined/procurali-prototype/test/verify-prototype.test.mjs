import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  AI_BUDGET_POLICY,
  CATEGORIES,
  COMMUNITIES,
  GUIDES,
  MEDIA_RIGHTS,
  OFFERS,
  SITE_META,
} from "../data/demo-catalog.mjs";
import * as nodeFs from "node:fs";
import * as nodeOs from "node:os";
import * as nodePath from "node:path";
import { execFileSync } from "node:child_process";
import {
  buildOfficialAffiliateAnchorProps,
  checkForbiddenClaims,
  contrastRatio,
  filterAndSortOffers,
  formatBrl,
  isOfferFresh,
  isSafePublicAssetPath,
  resolveMediaSrc,
  runDeterministicFinder,
  validateCatalogSnapshot,
  verifySevenDaySample,
} from "../lib/contracts.mjs";

const TEST_DIR = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(TEST_DIR, "..");
const SITE_DIR = path.join(ROOT, process.env.PROCURALI_OUT_DIR || "out");
// Next.js also emits copies of the 404 page at 404/ and _not-found/; build assets live in _next/.
const SKIPPED_DIRS = new Set(["_next", "404", "_not-found"]);

function collectHtmlFiles(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (SKIPPED_DIRS.has(entry.name) && dir === SITE_DIR) continue;
      out.push(...collectHtmlFiles(full));
    } else if (entry.isFile() && entry.name.endsWith(".html")) {
      out.push(full);
    }
  }
  return out.sort();
}

/** Finds an opening tag of `tagName` that carries every given attribute (order-independent). */
function findTag(html, tagName, attrs) {
  const tags = html.match(new RegExp(`<${tagName}\\b[^>]*>`, "g")) || [];
  return tags.find((tag) =>
    Object.entries(attrs).every(([name, value]) => tag.includes(`${name}="${value}"`))
  );
}

/** Rendered markup only: drops the React Server Component payload scripts and JSON-LD blocks. */
function visibleMarkup(html) {
  return html.replace(/<script\b[^>]*>[\s\S]*?<\/script>/g, "");
}

function resolveLocalTarget(file, target) {
  const clean = decodeURIComponent(target.split("#")[0].split("?")[0]);
  let resolved = clean.startsWith("/") ? path.join(SITE_DIR, clean) : path.resolve(path.dirname(file), clean);
  if (fs.existsSync(resolved) && fs.statSync(resolved).isDirectory()) {
    resolved = path.join(resolved, "index.html");
  }
  return resolved;
}

test("1. All required static pages exist, use pt-BR, single H1, unique metadata, and valid relative links", () => {
  const htmlFiles = collectHtmlFiles(SITE_DIR);
  assert.equal(htmlFiles.length, 34, "Expected 34 static HTML pages including 4 ML offers, 1 ML category, and 404.html");

  const titles = new Set();
  const descriptions = new Set();

  for (const file of htmlFiles) {
    const html = fs.readFileSync(file, "utf8");
    const rel = path.relative(SITE_DIR, file);

    assert.match(html, /<html lang="pt-BR"/, `${rel}: missing pt-BR lang attribute`);
    assert.match(html, /<meta name="robots" content="noindex, nofollow"\/?>/, `${rel}: demo staging must stay noindex`);
    assert.match(html, /href="#conteudo-principal"/, `${rel}: missing skip link`);
    assert.match(html, /<main id="conteudo-principal"/, `${rel}: missing main landmark`);

    const h1Matches = visibleMarkup(html).match(/<h1[\s>]/g) || [];
    assert.equal(h1Matches.length, 1, `${rel}: must contain exactly one <h1>`);

    const titleMatch = html.match(/<title>([^<]+)<\/title>/);
    assert.ok(titleMatch, `${rel}: missing title`);
    assert.ok(!titles.has(titleMatch[1]), `${rel}: duplicate title ${titleMatch[1]}`);
    titles.add(titleMatch[1]);

    const descMatch = html.match(/<meta name="description" content="([^"]+)"\/?>/);
    assert.ok(descMatch, `${rel}: missing meta description`);
    assert.ok(!descriptions.has(descMatch[1]), `${rel}: duplicate description`);
    descriptions.add(descMatch[1]);

    // Check local href / src targets (site-absolute or relative) resolve to exported files.
    const attrMatches = [...visibleMarkup(html).matchAll(/(?:href|src)="([^"]+)"/g)];
    for (const m of attrMatches) {
      const target = m[1];
      if (
        target.startsWith("http://") ||
        target.startsWith("https://") ||
        target.startsWith("#") ||
        target.startsWith("data:")
      ) {
        continue;
      }
      const resolved = resolveLocalTarget(file, target);
      assert.ok(fs.existsSync(resolved), `${rel}: broken local link/asset ${target} -> ${resolved}`);
    }
  }
});

test("2. Home copy, primary nav, affiliate disclosure, and honest disabled demo/WhatsApp CTAs", () => {
  const home = fs.readFileSync(path.join(SITE_DIR, "index.html"), "utf8");
  assert.match(home, /Encontre o que vale a pena para você\./);
  assert.match(home, /Ofertas e guias para escolher com mais contexto\./);
  assert.match(home, /Explorar ofertas/);
  assert.match(home, /Receber achados no Telegram/);
  assert.match(home, /href="\/como-selecionamos\/"/);
  assert.match(home, /Achados que fazem sentido\./);
  assert.match(home, /href="https:\/\/t\.me\/procurali"/);

  const selectionPage = fs.readFileSync(path.join(SITE_DIR, "como-selecionamos", "index.html"), "utf8");
  assert.match(selectionPage, /Como o Procurali seleciona/);

  assert.doesNotMatch(home, /\.local\/|GANHOS 5%/);

  const mlOffers = OFFERS.filter((o) => !o.isDemoFixture);
  assert.equal(mlOffers.length, 4, "Expected 4 real Mercado Livre affiliate test offers");

  for (const offer of OFFERS) {
    const detailPath = path.join(SITE_DIR, "ofertas", offer.slug, "index.html");
    const detail = fs.readFileSync(detailPath, "utf8");
    assert.match(detail, /Link de afiliado: podemos receber comissão se você comprar\./);
    assert.match(detail, /Preço e disponibilidade podem mudar\. Confirme as condições na loja\./);
    assert.doesNotMatch(detail, /\.local\/|GANHOS 5%/);
    if (offer.isDemoFixture) {
      assert.ok(
        findTag(detail, "button", { "data-cta": "disabled", disabled: "", "aria-disabled": "true" }),
        `${offer.slug}: demo fixture must render a disabled purchase button`
      );
      assert.equal(findTag(detail, "a", { "data-cta": "affiliate" }), undefined, `${offer.slug}: demo must not link out`);
      assert.doesNotMatch(detail, /shopee\.com\.br/);
    } else {
      assert.ok(
        findTag(detail, "a", {
          href: offer.officialAffiliateUrl,
          rel: "sponsored noopener",
          target: "_blank",
          "data-cta": "affiliate",
        }),
        `${offer.slug}: expected exact official ML affiliate anchor`
      );
      assert.ok(
        findTag(home, "a", { href: offer.officialAffiliateUrl, rel: "sponsored noopener", target: "_blank" }),
        `Home must include exact live URL for ${offer.slug}`
      );
      assert.match(detail, /specs-grid/, `${offer.slug}: expected rendered specs matrix`);
      assert.match(detail, /long-desc-card/, `${offer.slug}: expected full description block`);
      assert.match(detail, new RegExp(offer.shareId), `${offer.slug}: expected shareId ${offer.shareId}`);
      if (offer.galleryPaths?.length > 1) {
        assert.match(detail, /data-gallery-btn/, `${offer.slug}: expected multi-photo gallery buttons`);
      }
      if (offer.variants?.length > 0) {
        assert.match(detail, /variants-grid/, `${offer.slug}: expected variants comparison cards`);
      }
    }
  }

  const groups = fs.readFileSync(path.join(SITE_DIR, "grupos", "index.html"), "utf8");
  assert.match(groups, /href="https:\/\/t\.me\/procurali"/);
  assert.doesNotMatch(groups, /chat\.whatsapp\.com/);
  assert.match(groups, /expor seu número de telefone/);
  assert.ok(findTag(groups, "button", { disabled: "", "aria-disabled": "true" }), "WhatsApp previews stay disabled");

  const contact = fs.readFileSync(path.join(SITE_DIR, "contato", "index.html"), "utf8");
  assert.doesNotMatch(contact, /enviado com sucesso/i);
  assert.match(contact, /Enviar mensagem \(desabilitado/);
});

test("3. Official affiliate URL byte-preservation and state blocking contract", () => {
  const realUrl = "https://s.shopee.com.br/example-exact-path?sub_id=procurali_01&u=abc%20def";
  const prodRights = [{ id: "mr-ok", rightsVerified: true }];
  const prodOffer = {
    isDemoFixture: false,
    state: "approved",
    mediaRightsRef: "mr-ok",
    officialAffiliateUrl: realUrl,
  };
  const props = buildOfficialAffiliateAnchorProps(prodOffer, prodRights);
  assert.equal(props.enabled, true);
  assert.equal(props.href, realUrl);
  assert.equal(props.rel, "sponsored noopener");
  assert.equal(props.label, "Ver oferta na Shopee");

  // Demo offer never gets an enabled link even if a URL is supplied
  const demoBlocked = buildOfficialAffiliateAnchorProps({ ...prodOffer, isDemoFixture: true }, prodRights);
  assert.equal(demoBlocked.enabled, false);
  assert.equal(demoBlocked.href, null);

  // Missing media rights blocks purchase link
  const rightsBlocked = buildOfficialAffiliateAnchorProps(prodOffer, [{ id: "mr-ok", rightsVerified: false }]);
  assert.equal(rightsBlocked.enabled, false);

  // Stale or expired state blocks purchase link
  const staleBlocked = buildOfficialAffiliateAnchorProps({ ...prodOffer, state: "stale" }, prodRights);
  assert.equal(staleBlocked.enabled, false);
});

test("4. No forbidden unsupported claims or false confirmations across all generated pages", () => {
  const htmlFiles = collectHtmlFiles(SITE_DIR);
  for (const file of htmlFiles) {
    const html = fs.readFileSync(file, "utf8");
    assert.deepEqual(checkForbiddenClaims(html), [], `Forbidden claim in ${file}`);
    assert.doesNotMatch(html, /ofertas verificadas/i);
    assert.doesNotMatch(html, /100\/100/);
    assert.doesNotMatch(html, /chat\.whatsapp\.com/);
  }
});

test("5. Filters, 7-day sample verification, freshness check, and deterministic AI finder states", () => {
  assert.equal(formatBrl(8990), "R$ 89,90");
  const sevenDayDemo = OFFERS.find((o) => o.id === "offer-demo-001");
  const singleDemo = OFFERS.find((o) => o.id === "offer-demo-002");
  assert.ok(sevenDayDemo && singleDemo);
  assert.equal(verifySevenDaySample(sevenDayDemo).eligible, true);
  assert.equal(verifySevenDaySample(singleDemo).eligible, false);

  // Mismatched variant inside sample breaks 7-day eligibility
  const tampered = structuredClone(sevenDayDemo);
  tampered.priceObservations[3].variantId = "outra-variante";
  assert.equal(verifySevenDaySample(tampered).eligible, false);

  // Freshness checks against prototype reference timestamp
  assert.equal(isOfferFresh(sevenDayDemo, SITE_META.prototypeReferenceTimeUtc).fresh, true);
  for (const ml of OFFERS.filter((o) => !o.isDemoFixture)) {
    assert.equal(isOfferFresh(ml, SITE_META.prototypeReferenceTimeUtc).fresh, true);
  }
  const staleOffer = OFFERS.find((o) => o.state === "stale");
  assert.equal(isOfferFresh(staleOffer, SITE_META.prototypeReferenceTimeUtc).fresh, false);

  // Filters & sorting
  const activeOnly = filterAndSortOffers(OFFERS, { statusFilter: "ativas", sort: "price_asc" });
  assert.equal(activeOnly.length, 7);
  for (let i = 1; i < activeOnly.length; i += 1) {
    assert.ok(activeOnly[i - 1].priceBrlCents <= activeOnly[i].priceBrlCents);
  }

  const hardwareOnly = filterAndSortOffers(OFFERS, {
    category: "hardware-e-perifericos",
    statusFilter: "ativas",
  });
  assert.equal(hardwareOnly.length, 4);

  const above300 = filterAndSortOffers(OFFERS, {
    priceBand: "acima-de-300",
    statusFilter: "ativas",
  });
  assert.equal(above300.length, 2);

  const redragonSearch = filterAndSortOffers(OFFERS, {
    query: "Redragon",
    statusFilter: "ativas",
  });
  assert.equal(redragonSearch.length, 2);

  const nonActive = filterAndSortOffers(OFFERS, { statusFilter: "nao-ativas" });
  assert.equal(nonActive.length, 3);

  const couponOnly = filterAndSortOffers(OFFERS, { statusFilter: "ativas", condition: "com-cupom" });
  assert.equal(couponOnly.length, 1);
  assert.equal(couponOnly[0].slug, "luminaria-barra-monitor-assimetrica-40cm");

  // AI finder states
  const match = runDeterministicFinder({ need: "notebook", maxBudgetBrl: 180, requirement: "qualquer", simulatedState: "normal" }, OFFERS);
  assert.equal(match.modeLabel, "Demonstração — sem consulta ao vivo");
  assert.equal(match.status, "match");
  assert.ok(match.matches.length >= 1 && match.matches.length <= 3);

  const noMatch = runDeterministicFinder({ need: "cadeira", maxBudgetBrl: 30, requirement: "qualquer", simulatedState: "normal" }, OFFERS);
  assert.equal(noMatch.status, "no-match");

  for (const state of ["source-stale", "service-unavailable", "budget-cap"]) {
    const res = runDeterministicFinder({ need: "notebook", maxBudgetBrl: 200, simulatedState: state }, OFFERS);
    assert.equal(res.status, state);
    assert.equal(res.matches.length, 0);
  }
});

test("6. Metadata, truthful JSON-LD, sitemap exclusions, and staging robots.txt", () => {
  const guideHtml = fs.readFileSync(
    path.join(SITE_DIR, "guias", GUIDES[0].slug, "index.html"),
    "utf8"
  );
  assert.match(guideHtml, /"@type": "Article"/);
  assert.match(guideHtml, /"@type": "BreadcrumbList"/);
  assert.doesNotMatch(guideHtml, /AggregateRating/);

  const sitemap = fs.readFileSync(path.join(SITE_DIR, "sitemap.xml"), "utf8");
  assert.match(sitemap, /https:\/\/procurali\.com\.br\/ofertas\//);
  assert.doesNotMatch(sitemap, /cms-preview/);
  assert.doesNotMatch(sitemap, /404/);

  const robots = fs.readFileSync(path.join(SITE_DIR, "robots.txt"), "utf8");
  assert.match(robots, /Disallow: \//);
});

test("7. WCAG contrast pairings, CSS responsiveness rules, and form/image accessibility", () => {
  const tokens = JSON.parse(fs.readFileSync(path.join(ROOT, "brand", "design-tokens.json"), "utf8"));
  assert.ok(contrastRatio(tokens.colors.petrol.value, tokens.colors.canvas.value) >= 4.5);
  assert.ok(contrastRatio(tokens.colors.ink.value, tokens.colors.canvas.value) >= 4.5);
  assert.ok(contrastRatio(tokens.colors.muted.value, tokens.colors.canvas.value) >= 4.5);
  assert.ok(contrastRatio(tokens.colors.inkOnCoral.value, tokens.colors.coral.value) >= 4.5);
  assert.ok(contrastRatio(tokens.colors.coralText.value, tokens.colors.canvas.value) >= 4.5);
  assert.ok(contrastRatio(tokens.colors.petrol.value, tokens.colors.lime.value) >= 4.5);

  const cssDir = path.join(SITE_DIR, "_next", "static");
  const cssFiles = [];
  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (entry.name.endsWith(".css")) cssFiles.push(full);
    }
  };
  walk(cssDir);
  assert.ok(cssFiles.length > 0, "Expected compiled CSS in the export");
  const css = cssFiles.map((f) => fs.readFileSync(f, "utf8")).join("\n");
  assert.match(css, /overflow-x:\s*clip/);
  assert.match(css, /:focus-visible/);
  assert.match(css, /prefers-reduced-motion:\s*reduce/);
  assert.match(css, /min-height:\s*(44px|2\.75rem|calc\(var\(--spacing\)\s*\*\s*11\))/);

  // Every img in generated HTML has alt attribute
  for (const file of collectHtmlFiles(SITE_DIR)) {
    const html = fs.readFileSync(file, "utf8");
    const imgs = [...visibleMarkup(html).matchAll(/<img\b[^>]*>/g)];
    for (const img of imgs) {
      assert.match(img[0], /\balt="[^"]+"/, `${file}: image missing non-empty alt`);
    }
  }
});

test("8. Brand exports (SVG + valid PNGs) and CMS schema structures", () => {
  const requiredPngs = [
    "favicon-16.png",
    "favicon-32.png",
    "apple-touch-icon-180.png",
    "avatar-512.png",
    "avatar-1080.png",
    "social-cover-1200x630.png",
    "logo-primary-light-800.png",
    "logo-primary-dark-800.png",
  ];
  for (const name of requiredPngs) {
    const filePath = path.join(ROOT, "brand", "png", name);
    assert.ok(fs.existsSync(filePath), `Missing PNG export ${name}`);
    const buf = fs.readFileSync(filePath);
    assert.ok(buf.length > 200, `PNG ${name} unexpectedly small`);
    assert.deepEqual([...buf.subarray(0, 4)], [0x89, 0x50, 0x4e, 0x47], `${name} invalid PNG signature`);
  }

  const schema = JSON.parse(fs.readFileSync(path.join(ROOT, "contracts", "cms-schemas.json"), "utf8"));
  for (const def of [
    "Offer",
    "PriceObservation",
    "Guide",
    "Community",
    "CreativeAsset",
    "EvidenceSource",
    "MediaRights",
    "EditorialApproval",
    "AiBudgetPolicy",
  ]) {
    assert.ok(schema.$defs[def], `Missing schema definition ${def}`);
  }
  assert.equal(AI_BUDGET_POLICY.monthlyCeilingUsd, 20);
  assert.equal(AI_BUDGET_POLICY.extraUsageDisabled, true);
  assert.equal(CATEGORIES.length, 5);
  assert.equal(COMMUNITIES.length, 4);
  assert.equal(MEDIA_RIGHTS.length, 10);
});

test("9. Hub API catalog snapshot ingestion preserves MinIO HTTPS URLs, specs, variants, meli.la links, and emits api/catalog.json", async () => {
  assert.equal(isSafePublicAssetPath("https://media.procurali.com.br/procurali-media/ml/MLB39410080/hero.jpg"), true);
  assert.equal(isSafePublicAssetPath("/brand/procurali-mark.svg"), true);
  assert.equal(isSafePublicAssetPath(".local/ml/hero.jpg"), false);
  assert.equal(isSafePublicAssetPath("/home/iiii/hero.jpg"), false);
  assert.equal(isSafePublicAssetPath("file:///tmp/hero.jpg"), false);
  assert.equal(isSafePublicAssetPath("javascript:alert(1)"), false);

  assert.equal(
    resolveMediaSrc("./", "https://media.procurali.com.br/procurali-media/ml/MLB39410080/hero.jpg"),
    "https://media.procurali.com.br/procurali-media/ml/MLB39410080/hero.jpg",
  );
  assert.equal(resolveMediaSrc("../", "/brand/procurali-mark.svg"), "../brand/procurali-mark.svg");

  const minioHero = "https://media.procurali.com.br/procurali-media/ml/MLB39410080/hero.jpg";
  const minioAlt = "https://media.procurali.com.br/procurali-media/ml/MLB39410080/gallery-1.jpg";
  const snapshotFixture = {
    generatedAt: "2026-11-15T18:00:00Z",
    categories: [
      {
        slug: "hardware-e-perifericos",
        name: "Hardware e periféricos",
        description: "SSDs, teclados, headsets e monitores verificados do Mercado Livre.",
        editableByOwner: true,
        intendedBudgetHint: "R$ 150 a R$ 1.800",
      },
    ],
    offers: [
      {
        id: "offer-ml-MLB39410080",
        slug: "ssd-kingston-nv3-1tb-nvme-m2",
        isDemoFixture: false,
        productId: "MLB39410080",
        brand: "Kingston",
        model: "SNV3S/1000G",
        title: "SSD Kingston NV3 1TB M.2 2280 NVMe PCIe 4.0",
        exactVariant: "SNV3S/1000G · 1 TB PCIe 4.0",
        categorySlug: "hardware-e-perifericos",
        merchant: "Mercado Livre · Loja oficial FAMILAR",
        marketplaceLabel: "Mercado Livre",
        sellerName: "Loja oficial FAMILAR",
        storeShort: "Loja oficial FAMILAR",
        program: "Mercado Livre Afiliados (ingestão validada via Hub)",
        officialAffiliateUrl: "https://meli.la/1iWQZbJ",
        canonicalUrl: "https://www.mercadolivre.com.br/p/MLB39410080",
        shareId: "1PBCGF-SMN5",
        shareText: "SSD Kingston NV3 1TB\nhttps://meli.la/1iWQZbJ",
        sponsored: false,
        commissionRate: "12%",
        marketplaceRating: "4,9 / 5",
        marketplaceSalesLabel: "+5 mil vendidos",
        cardBadge: "Mais vendido",
        discountPercentLabel: "22% OFF",
        installmentSummary: "Até 10x R$ 44,99 sem juros",
        returnDays: 30,
        availability: "InStock",
        priceValidUntil: "2026-11-15",
        imagePath: minioHero,
        imageAlt: "SSD Kingston NV3 1TB M.2 2280 NVMe",
        galleryPaths: [
          {
            path: minioHero,
            alt: "SSD Kingston NV3 1TB vista principal",
            bucket: "procurali-media",
            objectKey: "ml/MLB39410080/hero.jpg",
            sha256: "a".repeat(64),
          },
          {
            path: minioAlt,
            alt: "SSD Kingston NV3 1TB embalagem",
            bucket: "procurali-media",
            objectKey: "ml/MLB39410080/gallery-1.jpg",
            sha256: "b".repeat(64),
          },
        ],
        priceBrlCents: 44990,
        pixPriceCents: 44990,
        referencePriceCents: 57900,
        currency: "BRL",
        paymentConditions: "R$ 449,90 à vista ou em até 10x sem juros no anúncio",
        shortCondition: "10x s/ juros · devolução grátis 30d",
        shortUseLine: "SSD M.2 2280 NVMe PCIe 4.0 x4 com leitura sequencial até 6.000 MB/s.",
        shortCheckedAt: "15/11 às 14h00",
        ctaDestLabel: "Ver oferta no Mercado Livre",
        couponCondition: null,
        freightStatus: "Frete grátis · Devolução grátis 30d",
        observedAt: "2026-11-15T18:00:00Z",
        observedAtDisplay: "15/11/2026 às 14:00 (horário de Cuiabá)",
        validUntil: "2026-11-15T23:59:59-03:00",
        validUntilDisplay: "Validade registrada até 2026-11-15",
        freshnessTtlMinutes: 60,
        evidenceRefs: ["ev-ml-MLB39410080"],
        mediaRightsRef: "mr-ml-MLB39410080",
        state: "approved",
        approvalRevision: "ml-ingested-1234567890ab",
        oneLineReason: "Menor preço observado na vitrine oficial com envio Full.",
        forWhomItMakesSense: "Upgrades de notebook ou PC com slot M.2 2280 NVMe.",
        supportedBenefits: ["Interface PCIe 4.0 x4 NVMe.", "Garantia oficial Kingston."],
        limitations: ["Placa-mãe PCIe 3.0 limita a banda máxima."],
        unknowns: [],
        tags: ["ssd", "nvme", "hardware"],
        hasVerifiedSevenDaySample: false,
        sevenDayCoverageNote: "Ingestão validada via Hub Mercado Livre; série de 7 dias acumulada progressivamente.",
        priceObservations: [],
        specs: [
          { name: "Capacidade", value: "1 TB" },
          { name: "Interface", value: "M.2 2280 PCIe 4.0 NVMe" },
        ],
        variants: [
          {
            productId: "MLB39410080",
            sku: "SNV3S/500G",
            name: "SSD Kingston NV3 500GB",
            priceBrlCents: 31990,
            currency: "BRL",
          },
          {
            productId: "MLB39410080",
            sku: "SNV3S/1000G",
            name: "SSD Kingston NV3 1TB",
            priceBrlCents: 44990,
            currency: "BRL",
          },
        ],
        longDescription: "Unidade M.2 2280 PCIe 4.0 indicada para notebooks e desktops compatíveis.",
        priceIntelligence: {
          windowDays: 30,
          currentPriceCents: 44990,
          lowestPrice30dCents: 44990,
          averagePrice30dCents: 51900,
          highestPrice30dCents: 57900,
          referencePriceCents: 57900,
          discountPct: 22,
          savingsVsReferenceCents: 12910,
          savingsVsAverageCents: 6910,
          priceLevel: "lowest_price",
          priceLevelLabel: "Menor preço dos últimos 30 dias",
          isPriceDrop: true,
          verifiedAtDisplay: "Preços verificados em 15/11/2026, 14:00",
          svgSparkline:
            '<svg viewBox="0 0 300 80" width="300" height="80" role="img" aria-label="Histórico de preço 30 dias"><polyline fill="none" stroke="#16a34a" stroke-width="2.5" points="10,20 150,45 290,68"/></svg>',
        },
        dynamicFaq: [
          {
            question: "Este SSD Kingston NV3 1TB está no menor preço recente?",
            answer: "Sim: R$ 449,90 corresponde ao menor valor registrado na janela recente de 30 dias.",
          },
        ],
      },
    ],
    evidenceSources: [
      {
        id: "ev-ml-MLB39410080",
        sourceType: "hub-api-snapshot",
        title: "Ingestão Hub Mercado Livre · MLB39410080",
        publisher: "ZaturoBot Hub",
        sourceUrl: "https://www.mercadolivre.com.br/p/MLB39410080",
        retrievedAt: "2026-11-15T18:00:00Z",
        verifiedFacts: ["Preço à vista e variantes estruturadas validadas na ingestão."],
      },
    ],
    mediaRights: [
      {
        id: "mr-ml-MLB39410080",
        assetDescription: "Fotos públicas hospedadas em MinIO — MLB39410080",
        licenseType: "Mercado Livre Afiliados — Imagens públicas",
        rightsVerified: true,
        ownerPermittedUse: "Exibição vinculada ao anúncio oficial MLB39410080.",
        attributionText: "Fotos oficiais do anúncio no Mercado Livre (MLB39410080).",
      },
    ],
  };

  const validation = validateCatalogSnapshot(snapshotFixture);
  assert.equal(validation.valid, true, `Expected valid snapshot: ${validation.errors.join("; ")}`);

  const tmpRoot = nodeFs.mkdtempSync(nodePath.join(nodeOs.tmpdir(), "procurali-api-build-"));
  // A second, isolated static export built from the Hub snapshot (distDir is env-controlled).
  const outName = "out-snapshot-test";
  const outDir = nodePath.join(ROOT, outName);
  const nextBin = nodePath.join(ROOT, "node_modules", "next", "dist", "bin", "next");
  const run = (args, env = {}) =>
    execFileSync(process.execPath, args, { cwd: ROOT, env: { ...process.env, ...env }, stdio: "pipe" });
  try {
    const snapshotPath = nodePath.join(tmpRoot, "site-catalog.json");
    nodeFs.writeFileSync(snapshotPath, JSON.stringify(snapshotFixture, null, 2), "utf8");

    run(["scripts/prepare.mjs", "--api-snapshot", snapshotPath]);
    run([nextBin, "build"], { PROCURALI_DIST_DIR: outName });
    const pageCount = collectHtmlFiles(outDir).length;
    assert.ok(pageCount >= 10, `Expected full site output, got ${pageCount} pages`);

    const emittedCatalogPath = nodePath.join(outDir, "api", "catalog.json");
    assert.ok(nodeFs.existsSync(emittedCatalogPath), "Expected api/catalog.json to be emitted");
    const emittedCatalog = JSON.parse(nodeFs.readFileSync(emittedCatalogPath, "utf8"));
    assert.equal(emittedCatalog.offers.length, 1);
    assert.equal(emittedCatalog.offers[0].imagePath, minioHero);
    assert.equal(emittedCatalog.offers[0].galleryPaths[0].objectKey, "ml/MLB39410080/hero.jpg");

    const detailHtml = nodeFs.readFileSync(
      nodePath.join(outDir, "ofertas", "ssd-kingston-nv3-1tb-nvme-m2", "index.html"),
      "utf8",
    );
    assert.ok(detailHtml.includes(minioHero), "Detail HTML must reference MinIO HTTPS image URL directly");
    assert.ok(
      detailHtml.includes(`<meta property="og:image" content="${minioHero}"`),
      "Offer detail page must expose absolute HTTPS og:image for WhatsApp Web preview",
    );
    assert.ok(detailHtml.includes('href="https://meli.la/1iWQZbJ"'), "Detail HTML must preserve exact meli.la URL");
    assert.ok(detailHtml.includes("M.2 2280 PCIe 4.0 NVMe"), "Detail HTML must render technical specs");
    assert.ok(detailHtml.includes("SNV3S/500G"), "Detail HTML must render variant SKU options");
    assert.ok(detailHtml.includes('id="tg-bot-draft-ssd-kingston-nv3-1tb-nvme-m2"'), "Detail HTML must include Telegram bot HTML draft");
    assert.ok(detailHtml.includes('id="wa-web-draft-ssd-kingston-nv3-1tb-nvme-m2"'), "Detail HTML must include WhatsApp Web assisted draft");
    assert.ok(detailHtml.includes("Histórico e Inteligência de Preço (30 dias)"), "Detail HTML must render 30-day price intelligence panel");
    assert.ok(detailHtml.includes('viewBox="0 0 300 80"'), "Detail HTML must render zero-JS SVG sparkline");
    assert.ok(detailHtml.includes('"@type": "FAQPage"'), "Detail HTML must emit Schema.org FAQPage JSON-LD");
    assert.ok(detailHtml.includes("Perguntas frequentes sobre esta oferta"), "Detail HTML must render dynamic FAQ section");
    assert.equal(detailHtml.includes(".local/"), false, "Detail HTML must never leak .local paths");

    const waDraftsPath = nodePath.join(outDir, "api", "whatsapp-assisted-drafts.json");
    assert.ok(nodeFs.existsSync(waDraftsPath), "Expected api/whatsapp-assisted-drafts.json to be emitted");
    const waDrafts = JSON.parse(nodeFs.readFileSync(waDraftsPath, "utf8"));
    assert.equal(waDrafts.mode, "whatsapp_web_human_assisted_no_meta_api");
    assert.equal(waDrafts.draftCount, 1);
    assert.ok(waDrafts.drafts[0].whatsappText.includes("https://meli.la/1iWQZbJ"));
    assert.ok(waDrafts.drafts[0].whatsappText.includes("https://procurali.com.br/ofertas/ssd-kingston-nv3-1tb-nvme-m2/index.html"));

    const sitemapXml = nodeFs.readFileSync(nodePath.join(outDir, "sitemap.xml"), "utf8");
    assert.ok(sitemapXml.includes('xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"'));
  } finally {
    // Restore the default demo catalog for the main export and remove the snapshot build.
    run(["scripts/prepare.mjs"]);
    nodeFs.rmSync(outDir, { recursive: true, force: true });
    nodeFs.rmSync(tmpRoot, { recursive: true, force: true });
  }
});

test("10. VPS site-builder entrypoint (src/build.mjs) exports a full site from a Hub snapshot into --out-dir", () => {
  const tmpRoot = nodeFs.mkdtempSync(nodePath.join(nodeOs.tmpdir(), "procurali-entrypoint-"));
  const snapshotPath = nodePath.join(tmpRoot, "site-catalog.json");
  const outDir = nodePath.join(tmpRoot, "release", "site");
  // The main export's machine-readable catalog is a valid Hub-shaped snapshot.
  nodeFs.copyFileSync(nodePath.join(SITE_DIR, "api", "catalog.json"), snapshotPath);
  // The site sync calls the builder through the `/opt/procurali/current` symlink.
  const linkedBuilder = nodePath.join(tmpRoot, "current-build.mjs");
  nodeFs.symlinkSync(nodePath.join(ROOT, "src", "build.mjs"), linkedBuilder);
  try {
    const stdout = execFileSync(
      process.execPath,
      [linkedBuilder, "--api-snapshot", snapshotPath, "--out-dir", outDir],
      { cwd: tmpRoot, env: { ...process.env, NEXT_TELEMETRY_DISABLED: "1" }, encoding: "utf8" },
    );
    assert.ok(stdout.includes(`source=api-snapshot:site-catalog.json`), stdout);
    for (const rel of ["index.html", "ofertas/index.html", "api/catalog.json", "api/whatsapp-assisted-drafts.json", "sitemap.xml"]) {
      assert.ok(nodeFs.statSync(nodePath.join(outDir, rel)).size > 0, `Expected ${rel} in --out-dir`);
    }
    assert.ok(nodeFs.readdirSync(nodePath.join(outDir, "_next", "static")).length > 0, "Expected Next.js assets");
    const emitted = JSON.parse(nodeFs.readFileSync(nodePath.join(outDir, "api", "catalog.json"), "utf8"));
    assert.equal(emitted.offers.length, OFFERS.length);
    const leftovers = nodeFs.readdirSync(ROOT).filter((name) => name.startsWith("out-sitebuild-"));
    assert.deepEqual(leftovers, [], "Temporary distDir must be removed after the export is copied");
    assert.throws(
      () => execFileSync(process.execPath, [linkedBuilder, "--api-snapshot", snapshotPath], { stdio: "pipe" }),
      /--out-dir/,
      "The builder must refuse to run without an explicit output directory",
    );
  } finally {
    execFileSync(process.execPath, ["scripts/prepare.mjs"], { cwd: ROOT, stdio: "pipe" });
    nodeFs.rmSync(tmpRoot, { recursive: true, force: true });
  }
});
