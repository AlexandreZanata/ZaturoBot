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
import {
  buildOfficialAffiliateAnchorProps,
  checkForbiddenClaims,
  contrastRatio,
  filterAndSortOffers,
  formatBrl,
  isOfferFresh,
  runDeterministicFinder,
  verifySevenDaySample,
} from "../src/validate-contracts.mjs";

const TEST_DIR = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(TEST_DIR, "..");
const SITE_DIR = path.join(ROOT, "site");

function collectHtmlFiles(dir) {
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      out.push(...collectHtmlFiles(full));
    } else if (entry.isFile() && entry.name.endsWith(".html")) {
      out.push(full);
    }
  }
  return out.sort();
}

test("1. All required static pages exist, use pt-BR, single H1, unique metadata, and valid relative links", () => {
  const htmlFiles = collectHtmlFiles(SITE_DIR);
  assert.equal(htmlFiles.length, 29, "Expected 29 static HTML pages including 404.html");

  const titles = new Set();
  const descriptions = new Set();

  for (const file of htmlFiles) {
    const html = fs.readFileSync(file, "utf8");
    const rel = path.relative(SITE_DIR, file);

    assert.match(html, /<html lang="pt-BR"/, `${rel}: missing pt-BR lang attribute`);
    assert.match(html, /<meta name="robots" content="noindex, nofollow">/, `${rel}: demo staging must stay noindex`);
    assert.match(html, /href="#conteudo-principal"/, `${rel}: missing skip link`);
    assert.match(html, /<main id="conteudo-principal"/, `${rel}: missing main landmark`);

    const h1Matches = html.match(/<h1[\s>]/g) || [];
    assert.equal(h1Matches.length, 1, `${rel}: must contain exactly one <h1>`);

    const titleMatch = html.match(/<title>([^<]+)<\/title>/);
    assert.ok(titleMatch, `${rel}: missing title`);
    assert.ok(!titles.has(titleMatch[1]), `${rel}: duplicate title ${titleMatch[1]}`);
    titles.add(titleMatch[1]);

    const descMatch = html.match(/<meta name="description" content="([^"]+)">/);
    assert.ok(descMatch, `${rel}: missing meta description`);
    assert.ok(!descriptions.has(descMatch[1]), `${rel}: duplicate description`);
    descriptions.add(descMatch[1]);

    // Check relative href / src targets resolve to real files in site/
    const attrMatches = [...html.matchAll(/(?:href|src)="([^"]+)"/g)];
    for (const m of attrMatches) {
      const target = m[1];
      if (
        target.startsWith("http://") ||
        target.startsWith("https://") ||
        target.startsWith("#")
      ) {
        continue;
      }
      const resolved = path.resolve(path.dirname(file), target.split("#")[0].split("?")[0]);
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
  assert.match(home, /Como o Procurali seleciona/);
  assert.match(home, /Achados que fazem sentido\./);
  assert.match(home, /href="https:\/\/t\.me\/procurali"/);

  for (const offer of OFFERS) {
    const detailPath = path.join(SITE_DIR, "ofertas", offer.slug, "index.html");
    const detail = fs.readFileSync(detailPath, "utf8");
    assert.match(detail, /Link de afiliado: podemos receber comissão se você comprar\./);
    assert.match(detail, /Preço e disponibilidade podem mudar\. Confirme as condições na loja\./);
    assert.match(detail, /<button type="button" class="btn" disabled aria-disabled="true">/);
    assert.doesNotMatch(detail, /shopee\.com\.br/);
  }

  const groups = fs.readFileSync(path.join(SITE_DIR, "grupos", "index.html"), "utf8");
  assert.match(groups, /href="https:\/\/t\.me\/procurali"/);
  assert.doesNotMatch(groups, /chat\.whatsapp\.com/);
  assert.match(groups, /expor seu número de telefone/);
  assert.match(groups, /disabled aria-disabled="true"/);

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
  assert.equal(verifySevenDaySample(OFFERS[0]).eligible, true);
  assert.equal(verifySevenDaySample(OFFERS[1]).eligible, false);

  // Mismatched variant inside sample breaks 7-day eligibility
  const tampered = structuredClone(OFFERS[0]);
  tampered.priceObservations[3].variantId = "outra-variante";
  assert.equal(verifySevenDaySample(tampered).eligible, false);

  // Freshness checks against prototype reference timestamp
  assert.equal(isOfferFresh(OFFERS[0], SITE_META.prototypeReferenceTimeUtc).fresh, true);
  const staleOffer = OFFERS.find((o) => o.state === "stale");
  assert.equal(isOfferFresh(staleOffer, SITE_META.prototypeReferenceTimeUtc).fresh, false);

  // Filters & sorting
  const activeOnly = filterAndSortOffers(OFFERS, { statusFilter: "ativas", sort: "price_asc" });
  assert.equal(activeOnly.length, 3);
  assert.ok(activeOnly[0].priceBrlCents <= activeOnly[1].priceBrlCents);
  assert.ok(activeOnly[1].priceBrlCents <= activeOnly[2].priceBrlCents);

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

  const css = fs.readFileSync(path.join(SITE_DIR, "assets", "styles.css"), "utf8");
  assert.match(css, /overflow-x:\s*clip/);
  assert.match(css, /:focus-visible/);
  assert.match(css, /prefers-reduced-motion:\s*reduce/);
  assert.match(css, /min-height:\s*44px/);

  // Every img in generated HTML has alt attribute
  for (const file of collectHtmlFiles(SITE_DIR)) {
    const html = fs.readFileSync(file, "utf8");
    const imgs = [...html.matchAll(/<img\b[^>]*>/g)];
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
  assert.equal(CATEGORIES.length, 4);
  assert.equal(COMMUNITIES.length, 4);
  assert.equal(MEDIA_RIGHTS.length, 6);
});
