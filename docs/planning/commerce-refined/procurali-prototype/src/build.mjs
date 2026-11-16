/**
 * Procurali Static Prototype Builder (Zero-dependency Node.js SSG)
 * Outputs clean static HTML, CSS, JS, SVG, PNG, sitemap.xml, and robots.txt into ./site/
 */

import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import {
  CATEGORIES,
  EVIDENCE_SOURCES,
  GUIDES,
  MEDIA_RIGHTS,
  OFFERS,
  SITE_META,
} from "../data/demo-catalog.mjs";
import {
  buildWhatsappWebAssistedPackage,
  checkForbiddenClaims,
  validateCatalogSnapshot,
} from "./validate-contracts.mjs";
import {
  esc,
  renderAiFinderPage,
  renderCategoriesIndexPage,
  renderCategoryPage,
  renderCmsPreviewPage,
  renderCreativeKitPage,
  renderGroupsPage,
  renderGuideDetailPage,
  renderGuidesIndexPage,
  renderHomePage,
  renderNotFoundPage,
  renderOfferDetailPage,
  renderOffersListPage,
  renderTrustPage,
  renderWeeklySelectionPage,
  setActiveCatalog,
} from "./pages.mjs";
import { BRAND_SYMBOL_SVG, icon } from "./icons.mjs";

const SRC_DIR = path.dirname(fileURLToPath(import.meta.url));
const PROTOTYPE_ROOT = path.resolve(SRC_DIR, "..");
const DEFAULT_SITE_DIR = path.join(PROTOTYPE_ROOT, "site");

function ensureCleanDir(dirPath) {
  fs.rmSync(dirPath, { recursive: true, force: true });
  fs.mkdirSync(dirPath, { recursive: true });
}

function copyTree(srcDir, dstDir) {
  fs.mkdirSync(dstDir, { recursive: true });
  for (const entry of fs.readdirSync(srcDir, { withFileTypes: true })) {
    const srcPath = path.join(srcDir, entry.name);
    const dstPath = path.join(dstDir, entry.name);
    if (entry.isDirectory()) {
      copyTree(srcPath, dstPath);
    } else if (entry.isFile()) {
      fs.copyFileSync(srcPath, dstPath);
    }
  }
}

function rootPrefixForRoute(routePath) {
  const clean = routePath.replace(/^\/+|\/+$/g, "");
  if (!clean || clean === "404.html") return "./";
  const depth = clean.split("/").length;
  return "../".repeat(depth);
}

function canonicalUrl(routePath) {
  const clean = routePath === "/" ? "/" : `/${routePath.replace(/^\/+|\/+$/g, "")}/`;
  return `https://${SITE_META.domain}${clean}`;
}

function buildJsonLd(routePath, extraLd = []) {
  const baseOrg = {
    "@context": "https://schema.org",
    "@type": "Organization",
    name: "Procurali (Procura Ali)",
    url: `https://${SITE_META.domain}/`,
    sameAs: [SITE_META.telegramUrl],
  };
  const blocks = routePath === "/" || routePath === "/sobre/" ? [baseOrg, ...extraLd] : extraLd;
  return blocks
    .map(
      (obj) =>
        `<script type="application/ld+json">\n${JSON.stringify(obj, null, 2).replace(/</g, "\\u003c")}\n</script>`
    )
    .join("\n  ");
}

function renderShell({
  routePath,
  activeNav = "",
  title,
  description,
  bodyHtml,
  jsonLd = [],
  includeCatalogJson = false,
  ogImage = null,
  assetVersions = null,
  catalogData = { offers: OFFERS, categories: CATEGORIES, mediaRights: MEDIA_RIGHTS },
}) {
  const root = rootPrefixForRoute(routePath);
  const fullTitle = `${title} — Procurali`;
  const robotsDirective = SITE_META.isDemoStaging ? "noindex, nofollow" : "index, follow";
  const resolvedOgImage =
    typeof ogImage === "string" && (ogImage.startsWith("https://") || ogImage.startsWith("http://"))
      ? ogImage
      : typeof ogImage === "string" && ogImage.trim()
        ? `https://${SITE_META.domain}/${ogImage.replace(/^\/+/, "")}`
        : `https://${SITE_META.domain}/brand/png/social-cover-1200x630.png`;
  const catalogScript = includeCatalogJson
    ? `<script type="application/json" id="procurali-catalog-data">${JSON.stringify({
        offers: catalogData.offers,
        categories: catalogData.categories,
        mediaRights: catalogData.mediaRights,
      }).replace(/</g, "\\u003c")}</script>`
    : "";

  const navLink = (key, href, iconName, label, extraClass = "") => {
    const current = activeNav === key ? ' aria-current="page"' : "";
    const cls = extraClass ? ` class="${extraClass}"` : "";
    return `<a href="${esc(root + href)}"${cls}${current}>${icon(iconName, "nav-icon")}<span>${esc(label)}</span></a>`;
  };

  return `<!DOCTYPE html>
<html lang="pt-BR" data-root="${esc(root)}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${esc(fullTitle)}</title>
  <meta name="description" content="${esc(description)}">
  <meta name="robots" content="${esc(robotsDirective)}">
  <link rel="canonical" href="${esc(canonicalUrl(routePath))}">
  <meta property="og:type" content="website">
  <meta property="og:locale" content="pt_BR">
  <meta property="og:title" content="${esc(fullTitle)}">
  <meta property="og:description" content="${esc(description)}">
  <meta property="og:url" content="${esc(canonicalUrl(routePath))}">
  <meta property="og:image" content="${esc(resolvedOgImage)}">
  <link rel="icon" type="image/svg+xml" href="${esc(root)}brand/procurali_icone_branco_transparente.svg">
  <link rel="alternate icon" type="image/svg+xml" href="${esc(root)}brand/favicon.svg">
  <link rel="icon" type="image/png" sizes="32x32" href="${esc(root)}brand/png/favicon-32.png">
  <link rel="apple-touch-icon" sizes="180x180" href="${esc(root)}brand/png/apple-touch-icon-180.png">
  <meta name="theme-color" content="#143D2B">
  <link rel="stylesheet" href="${esc(root)}assets/tokens.css${assetVersions?.tokens ? `?v=${assetVersions.tokens}` : ""}">
  <link rel="stylesheet" href="${esc(root)}assets/styles.css${assetVersions?.styles ? `?v=${assetVersions.styles}` : ""}">
  ${buildJsonLd(routePath, jsonLd)}
</head>
<body class="page-${esc(activeNav || "geral")}">
  <a class="skip-link" href="#conteudo-principal">Pular para o conteúdo principal</a>

  <div class="staging-bar" role="note" aria-label="Aviso de ambiente de testes">
    <div class="container staging-bar-inner">
      <span class="staging-pill">Protótipo · Testes</span>
      <span class="staging-text"><strong>Ambiente de testes (não indexa)</strong><span class="hide-sm"> · 4 ofertas reais do Mercado Livre + fixtures de estado</span></span>
      <span class="staging-link">Canal oficial: <a href="${esc(SITE_META.telegramUrl)}" target="_blank" rel="noopener">${esc(SITE_META.telegramHandle)}</a></span>
    </div>
  </div>

  <header class="site-header" data-site-header>
    <div class="container header-inner">
      <div class="header-brand-group">
        <a class="brand-link" href="${esc(root)}index.html" aria-label="Procurali — Página inicial">
          ${BRAND_SYMBOL_SVG}
          <span class="brand-word">procurali</span>
        </a>
      </div>

      <nav class="nav-primary" aria-label="Navegação principal">
        ${navLink("ofertas", "ofertas/index.html", "tag", "Ofertas")}
        ${navLink("cupons", "ofertas/index.html?condicao=com-cupom", "ticket", "Cupons")}
        ${navLink("guias", "guias/index.html", "book", "Guias")}
        ${navLink("grupos", "grupos/index.html", "users", "Grupos")}
      </nav>

      <form class="header-search-form" action="${esc(root)}ofertas/index.html" method="get" role="search" aria-label="Buscar ofertas no catálogo">
        <label class="sr-only" for="header-search-q">Buscar ofertas ou produtos</label>
        ${icon("search", "icon header-search-icon")}
        <input id="header-search-q" type="search" name="q" placeholder="Buscar produto, modelo ou loja...">
        <button type="submit" class="header-search-submit">Buscar</button>
      </form>

      <div class="header-actions">
        <button type="button" class="header-icon-btn saved-btn" data-open-saved aria-haspopup="dialog" aria-controls="saved-drawer" aria-label="Abrir itens salvos">
          ${icon("bookmark")}
          <span class="saved-count" data-saved-count hidden>0</span>
        </button>
        <a class="btn header-tg-btn" href="${esc(SITE_META.telegramUrl)}" target="_blank" rel="noopener" data-analytics-event="community_link_click" data-analytics-detail="telegram-header">
          ${icon("send")}
          <span>Grupos</span>
          <span class="sr-only"> Entrar no Telegram</span>
        </a>
        <button type="button" class="menu-toggle" data-menu-toggle aria-expanded="false" aria-controls="mobile-drawer" aria-label="Abrir menu de navegação">
          <span class="menu-icon-open">${icon("menu")}</span>
          <span class="menu-icon-close">${icon("close")}</span>
        </button>
      </div>
    </div>

    <div id="mobile-drawer" class="mobile-drawer" hidden>
      <div class="container">
        <form class="drawer-search" action="${esc(root)}ofertas/index.html" method="get" role="search" aria-label="Buscar ofertas no menu">
          <label class="sr-only" for="drawer-search-q">Buscar no catálogo</label>
          ${icon("search", "icon search-icon")}
          <input id="drawer-search-q" type="search" name="q" placeholder="Busque por produto, categoria ou oferta...">
          <button type="submit" class="btn btn-petrol">Buscar</button>
        </form>
        <ul class="mobile-nav-list">
          <li><a href="${esc(root)}ofertas/index.html">${icon("tag")}<span>Ofertas</span></a></li>
          <li><a href="${esc(root)}ofertas/index.html?condicao=com-cupom">${icon("ticket")}<span>Cupons</span></a></li>
          <li><a href="${esc(root)}guias/index.html">${icon("book")}<span>Guias</span></a></li>
          <li><a href="${esc(root)}grupos/index.html">${icon("users")}<span>Grupos</span></a></li>
          <li><a href="${esc(root)}selecoes-da-semana/index.html">${icon("zap")}<span>Achados do dia (semana)</span></a></li>
          <li><a href="${esc(root)}encontre-com-ia/index.html">${icon("sparkle")}<span>Encontre com IA (demo)</span></a></li>
          <li><a href="${esc(root)}categorias/index.html">${icon("grid")}<span>Todas as categorias</span></a></li>
        </ul>
        <p class="drawer-foot">
          <a href="${esc(root)}como-selecionamos/index.html">Como selecionamos</a> ·
          <a href="${esc(root)}transparencia/index.html">Transparência</a> ·
          <a href="${esc(root)}sobre/index.html">Sobre</a>
        </p>
      </div>
    </div>
  </header>

  <main id="conteudo-principal" tabindex="-1">
    ${bodyHtml}
  </main>

  <footer class="site-footer">
    <div class="container">
      <div class="footer-grid">
        <div class="footer-brand">
          <a class="brand-link brand-link--light" href="${esc(root)}index.html" aria-label="Procurali — voltar ao início">
            ${BRAND_SYMBOL_SVG}
            <span class="brand-word">procurali</span>
          </a>
          <p class="footer-tagline">“${esc(SITE_META.proposedTagline)}” <span class="footer-chip">proposta em aprovação</span></p>
          <p class="footer-about">Guia independente de compras e curadoria de ofertas com foco em contexto, transparência e decisão consciente. Não vendemos estoque nem processamos pagamentos.</p>
          <a class="btn btn-lime btn-sm" href="${esc(SITE_META.telegramUrl)}" target="_blank" rel="noopener" data-analytics-event="community_link_click" data-analytics-detail="telegram-footer">${icon("send")}<span>Seguir ${esc(SITE_META.telegramHandle)}</span></a>
        </div>
        <div>
          <h2 class="footer-heading">Explorar</h2>
          <ul>
            <li><a href="${esc(root)}ofertas/index.html">Ofertas (demo)</a></li>
            <li><a href="${esc(root)}categorias/index.html">Categorias iniciais</a></li>
            <li><a href="${esc(root)}guias/index.html">Guias de compra</a></li>
            <li><a href="${esc(root)}selecoes-da-semana/index.html">Seleções da semana</a></li>
            <li><a href="${esc(root)}encontre-com-ia/index.html">Encontre com IA (demo)</a></li>
            <li><a href="${esc(root)}grupos/index.html">Diretório de grupos</a></li>
          </ul>
        </div>
        <div>
          <h2 class="footer-heading">Confiança e regras</h2>
          <ul>
            <li><a href="${esc(root)}sobre/index.html">Sobre o Procurali</a></li>
            <li><a href="${esc(root)}como-selecionamos/index.html">Como selecionamos</a></li>
            <li><a href="${esc(root)}transparencia/index.html">Transparência de afiliados</a></li>
            <li><a href="${esc(root)}privacidade/index.html">Privacidade (rascunho)</a></li>
            <li><a href="${esc(root)}termos/index.html">Termos de uso (rascunho)</a></li>
            <li><a href="${esc(root)}contato/index.html">Contato</a></li>
          </ul>
        </div>
        <div>
          <h2 class="footer-heading">Kit e handoff</h2>
          <ul>
            <li><a href="${esc(root)}criativos/index.html">Kit criativo e templates SVG</a></li>
            <li><a href="${esc(root)}cms-preview/index.html">Mockup CMS (não produtivo)</a></li>
            <li><a href="${esc(SITE_META.telegramUrl)}" target="_blank" rel="noopener">Canal oficial @procurali</a></li>
          </ul>
          <p class="mono footer-domain">Domínio reservado: ${esc(SITE_META.domain)}</p>
        </div>
      </div>
      <div class="footer-bottom">
        <span>${esc(SITE_META.affiliateDisclosure)}</span>
        <span>Ambiente de testes sem coleta oculta · ofertas reais sinalizadas como link de afiliado.</span>
      </div>
    </div>
  </footer>

  <nav class="bottom-nav" aria-label="Navegação rápida">
    <a href="${esc(root)}index.html"${activeNav === "home" ? ' aria-current="page"' : ""}>${icon("home")}<span>Início</span></a>
    <a href="${esc(root)}ofertas/index.html"${activeNav === "ofertas" ? ' aria-current="page"' : ""}>${icon("tag")}<span>Ofertas</span></a>
    <a href="${esc(root)}guias/index.html"${activeNav === "guias" ? ' aria-current="page"' : ""}>${icon("book")}<span>Guias</span></a>
    <a href="${esc(root)}encontre-com-ia/index.html"${activeNav === "ia" ? ' aria-current="page"' : ""}>${icon("sparkle")}<span>IA</span></a>
    <a href="${esc(root)}grupos/index.html"${activeNav === "grupos" ? ' aria-current="page"' : ""}>${icon("users")}<span>Grupos</span></a>
  </nav>

  <div id="saved-drawer" class="saved-drawer" role="dialog" aria-modal="true" aria-labelledby="saved-drawer-title" hidden>
    <div class="saved-drawer-panel">
      <div class="saved-drawer-head">
        <h2 id="saved-drawer-title">${icon("bookmark")} Itens salvos</h2>
        <button type="button" class="icon-btn" data-close-saved aria-label="Fechar lista de salvos">${icon("close")}</button>
      </div>
      <p class="muted saved-drawer-note">Salvos apenas neste navegador (<code>localStorage</code>) para comparar durante a visita. Nada é enviado a servidores.</p>
      <div id="saved-items-list" class="saved-items-list"></div>
      <div class="saved-drawer-foot">
        <button type="button" class="btn btn-secondary btn-block" data-clear-saved>Limpar itens salvos</button>
      </div>
    </div>
  </div>

  ${catalogScript}
  <script defer src="${esc(root)}assets/app.js${assetVersions?.app ? `?v=${assetVersions.app}` : ""}"></script>
</body>
</html>
`;
}

function writePage(siteDir, routePath, options) {
  const html = renderShell({ routePath, ...options });
  const violations = checkForbiddenClaims(html);
  if (violations.length > 0) {
    throw new Error(`Forbidden marketing claim in ${routePath}: ${violations.join(", ")}`);
  }
  let targetFile;
  if (routePath === "/") {
    targetFile = path.join(siteDir, "index.html");
  } else if (routePath === "/404.html") {
    targetFile = path.join(siteDir, "404.html");
  } else {
    targetFile = path.join(siteDir, routePath.replace(/^\/+|\/+$/g, ""), "index.html");
  }
  fs.mkdirSync(path.dirname(targetFile), { recursive: true });
  fs.writeFileSync(targetFile, html, "utf8");
  return routePath;
}

function buildBrowserBundle() {
  const validatorsSource = fs
    .readFileSync(path.join(SRC_DIR, "validate-contracts.mjs"), "utf8")
    .replace(/^export\s+/gm, "");
  const domSource = fs.readFileSync(path.join(SRC_DIR, "assets", "app.js"), "utf8");
  return `"use strict";\n(function () {\n${validatorsSource}\n${domSource}\n})();\n`;
}

function breadcrumbLd(items) {
  return {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: items.map((item, idx) => ({
      "@type": "ListItem",
      position: idx + 1,
      name: item.name,
      item: `https://${SITE_META.domain}${item.path}`,
    })),
  };
}

function parseCliArgs(argv) {
  const opts = {
    apiSnapshotPath: process.env.PROCURALI_API_SNAPSHOT || null,
    hubApiUrl: process.env.PROCURALI_HUB_API_URL || null,
    hubToken: process.env.PROCURALI_HUB_TOKEN || null,
    outDir: process.env.PROCURALI_SITE_DIR ? path.resolve(process.env.PROCURALI_SITE_DIR) : DEFAULT_SITE_DIR,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--api-snapshot" && argv[i + 1]) {
      opts.apiSnapshotPath = path.resolve(argv[i + 1]);
      i += 1;
    } else if (arg === "--hub-url" && argv[i + 1]) {
      opts.hubApiUrl = argv[i + 1];
      i += 1;
    } else if (arg === "--out-dir" && argv[i + 1]) {
      opts.outDir = path.resolve(argv[i + 1]);
      i += 1;
    }
  }
  return opts;
}

function mergeCatalogSnapshot(snapshot, sourceLabel) {
  const snapshotCats = Array.isArray(snapshot.categories) ? snapshot.categories : [];
  const seenCats = new Set(snapshotCats.map((c) => c.slug));
  const mergedCategories = [
    ...snapshotCats,
    ...CATEGORIES.filter((c) => !seenCats.has(c.slug)),
  ];
  const mergedMediaRights =
    Array.isArray(snapshot.mediaRights) && snapshot.mediaRights.length > 0
      ? snapshot.mediaRights
      : MEDIA_RIGHTS;
  const mergedEvidence =
    Array.isArray(snapshot.evidenceSources) && snapshot.evidenceSources.length > 0
      ? snapshot.evidenceSources
      : EVIDENCE_SOURCES;

  return {
    schemaVersion: snapshot.schemaVersion || 1,
    generatedAtUtc: snapshot.generatedAtUtc || new Date().toISOString(),
    sourceLabel,
    offers: snapshot.offers || [],
    categories: mergedCategories,
    mediaRights: mergedMediaRights,
    evidenceSources: mergedEvidence,
  };
}

async function resolveCatalog(options) {
  if (options.snapshotData) {
    const merged = mergeCatalogSnapshot(options.snapshotData, options.sourceLabel || "api-snapshot-memory");
    const check = validateCatalogSnapshot(merged);
    if (!check.valid) {
      throw new Error(`Invalid API catalog snapshot: ${check.errors.join("; ")}`);
    }
    return merged;
  }
  if (options.apiSnapshotPath) {
    const raw = JSON.parse(fs.readFileSync(options.apiSnapshotPath, "utf8"));
    const merged = mergeCatalogSnapshot(raw, `api-snapshot:${path.basename(options.apiSnapshotPath)}`);
    const check = validateCatalogSnapshot(merged);
    if (!check.valid) {
      throw new Error(`Invalid API catalog snapshot file: ${check.errors.join("; ")}`);
    }
    return merged;
  }
  if (options.hubApiUrl) {
    const token = String(options.hubToken || "").trim();
    if (!token) {
      throw new Error("PROCURALI_HUB_TOKEN is required when PROCURALI_HUB_API_URL / --hub-url is set");
    }
    const endpoint = `${options.hubApiUrl.replace(/\/+$/, "")}/v1/commerce/mercadolivre/site-catalog`;
    const res = await fetch(endpoint, {
      headers: { Authorization: `Bearer ${token}` },
      redirect: "error",
    });
    if (!res.ok) {
      throw new Error(`Hub site-catalog fetch failed with HTTP ${res.status}`);
    }
    const raw = await res.json();
    const merged = mergeCatalogSnapshot(raw, "hub-api");
    const check = validateCatalogSnapshot(merged);
    if (!check.valid) {
      throw new Error(`Invalid Hub site-catalog payload: ${check.errors.join("; ")}`);
    }
    return merged;
  }
  return {
    schemaVersion: 1,
    generatedAtUtc: SITE_META.prototypeReferenceTimeUtc,
    sourceLabel: "static-demo",
    offers: OFFERS,
    categories: CATEGORIES,
    mediaRights: MEDIA_RIGHTS,
    evidenceSources: EVIDENCE_SOURCES,
  };
}

export async function buildSite(options = {}) {
  const siteDir = options.outDir ? path.resolve(options.outDir) : DEFAULT_SITE_DIR;
  const catalog = await resolveCatalog(options);
  setActiveCatalog(catalog);

  try {
    ensureCleanDir(siteDir);

    // Copy static assets, brand pack, and machine-readable catalog snapshot
    fs.mkdirSync(path.join(siteDir, "assets"), { recursive: true });
    fs.mkdirSync(path.join(siteDir, "api"), { recursive: true });
    const tokensCssBytes = fs.readFileSync(path.join(PROTOTYPE_ROOT, "brand", "tokens.css"));
    const stylesCssBytes = fs.readFileSync(path.join(SRC_DIR, "assets", "styles.css"));
    const appJsBundle = buildBrowserBundle();
    const shortHash = (buf) => crypto.createHash("sha256").update(buf).digest("hex").slice(0, 10);
    const assetVersions = {
      tokens: shortHash(tokensCssBytes),
      styles: shortHash(stylesCssBytes),
      app: shortHash(appJsBundle),
    };
    const write = (routePath, pageOpts) =>
      writePage(siteDir, routePath, { ...pageOpts, assetVersions, catalogData: catalog });

    fs.writeFileSync(path.join(siteDir, "assets", "tokens.css"), tokensCssBytes);
    fs.writeFileSync(path.join(siteDir, "assets", "styles.css"), stylesCssBytes);
    fs.writeFileSync(path.join(siteDir, "assets", "app.js"), appJsBundle, "utf8");
    fs.writeFileSync(
      path.join(siteDir, "api", "catalog.json"),
      JSON.stringify(
        {
          schemaVersion: catalog.schemaVersion,
          generatedAtUtc: catalog.generatedAtUtc,
          source: catalog.sourceLabel,
          offers: catalog.offers,
          categories: catalog.categories,
          mediaRights: catalog.mediaRights,
          evidenceSources: catalog.evidenceSources,
        },
        null,
        2
      ),
      "utf8"
    );
    fs.writeFileSync(
      path.join(siteDir, "api", "whatsapp-assisted-drafts.json"),
      JSON.stringify(
        buildWhatsappWebAssistedPackage(
          catalog.offers,
          catalog.generatedAtUtc,
          `https://${SITE_META.domain}`
        ),
        null,
        2
      ),
      "utf8"
    );
    copyTree(path.join(PROTOTYPE_ROOT, "brand"), path.join(siteDir, "brand"));
    copyTree(path.join(SRC_DIR, "assets", "fonts"), path.join(siteDir, "assets", "fonts"));

    const publicRoutes = [];

    // 1. Home
    publicRoutes.push(
      write("/", {
        activeNav: "home",
        title: "Procura Ali · Ofertas e guias para escolher com mais contexto",
        description:
          "Procurali (Procura Ali): guias de compra, curadoria de ofertas com condições explicadas e comunidade no Telegram em português.",
        bodyHtml: renderHomePage("./"),
      })
    );

    // 2. Offers listing & detail pages
    publicRoutes.push(
      write("/ofertas/", {
        activeNav: "ofertas",
        title: "Ofertas e registros demonstrativos",
        description:
          "Filtre ofertas de demonstração por categoria, faixa de preço e condição com transparência de observação.",
        bodyHtml: renderOffersListPage("../"),
        includeCatalogJson: true,
        jsonLd: [breadcrumbLd([{ name: "Início", path: "/" }, { name: "Ofertas", path: "/ofertas/" }])],
      })
    );

    for (const offer of catalog.offers) {
      const route = `/ofertas/${offer.slug}/`;
      const isLiveTest = !offer.isDemoFixture && offer.state === "approved";
      const offerJsonLd = [
        breadcrumbLd([
          { name: "Início", path: "/" },
          { name: "Ofertas", path: "/ofertas/" },
          { name: offer.title, path: route },
        ]),
      ];
      if (Array.isArray(offer.dynamicFaq) && offer.dynamicFaq.length > 0) {
        offerJsonLd.push({
          "@context": "https://schema.org",
          "@type": "FAQPage",
          mainEntity: offer.dynamicFaq.map((item) => ({
            "@type": "Question",
            name: item.question,
            acceptedAnswer: {
              "@type": "Answer",
              text: item.answer,
            },
          })),
        });
      }
      publicRoutes.push(
        write(route, {
          activeNav: "ofertas",
          title: isLiveTest
            ? offer.title
            : `${offer.title} (${offer.state === "approved" ? "demo" : offer.state})`,
          description: isLiveTest
            ? `${offer.oneLineReason} Registro real do Mercado Livre observado em ${offer.observedAtDisplay}.`
            : `${offer.oneLineReason} Dados demonstrativos fictícios com horário de observação e limitações.`,
          bodyHtml: renderOfferDetailPage("../../", offer),
          ogImage: offer.imagePath,
          jsonLd: offerJsonLd,
        })
      );
    }

    // 3. Categories
    publicRoutes.push(
      write("/categorias/", {
        activeNav: "categorias",
        title: "Categorias iniciais (editáveis)",
        description: "Explore categorias iniciais de organização, ergonomia, energia e iluminação de mesa.",
        bodyHtml: renderCategoriesIndexPage("../"),
        jsonLd: [breadcrumbLd([{ name: "Início", path: "/" }, { name: "Categorias", path: "/categorias/" }])],
      })
    );

    for (const cat of catalog.categories) {
      const route = `/categorias/${cat.slug}/`;
      publicRoutes.push(
        write(route, {
          activeNav: `cat-${cat.slug}`,
          title: `${cat.name} — Categoria`,
          description: cat.description,
          bodyHtml: renderCategoryPage("../../", cat),
          jsonLd: [
            breadcrumbLd([
              { name: "Início", path: "/" },
              { name: "Categorias", path: "/categorias/" },
              { name: cat.name, path: route },
            ]),
          ],
        })
      );
    }

    // 4. Guides & Weekly selection
    publicRoutes.push(
      write("/guias/", {
        activeNav: "guias",
        title: "Guias de compra e critérios",
        description: "Guias originais baseados em pesquisa de bancada para evitar erros de compatibilidade.",
        bodyHtml: renderGuidesIndexPage("../"),
        jsonLd: [breadcrumbLd([{ name: "Início", path: "/" }, { name: "Guias", path: "/guias/" }])],
      })
    );

    for (const guide of GUIDES) {
      const route = `/guias/${guide.slug}/`;
      publicRoutes.push(
        write(route, {
          activeNav: "guias",
          title: guide.title,
          description: guide.metaDescription,
          bodyHtml: renderGuideDetailPage("../../", guide),
          jsonLd: [
            breadcrumbLd([
              { name: "Início", path: "/" },
              { name: "Guias", path: "/guias/" },
              { name: guide.title, path: route },
            ]),
            {
              "@context": "https://schema.org",
              "@type": "Article",
              headline: guide.title,
              description: guide.metaDescription,
              datePublished: guide.publishedAt,
              dateModified: guide.updatedAt,
              inLanguage: "pt-BR",
              author: { "@type": "Organization", name: guide.author },
              publisher: { "@type": "Organization", name: "Procurali (Procura Ali)" },
            },
          ],
        })
      );
    }

    publicRoutes.push(
      write("/selecoes-da-semana/", {
        activeNav: "semanal",
        title: "Seleções da semana (demonstração)",
        description: "Recorte semanal datado com escolhas distintas e separação de históricos encerrados.",
        bodyHtml: renderWeeklySelectionPage("../"),
        jsonLd: [
          breadcrumbLd([
            { name: "Início", path: "/" },
            { name: "Seleções da semana", path: "/selecoes-da-semana/" },
          ]),
        ],
      })
    );

    // 5. Guided AI Finder
    publicRoutes.push(
      write("/encontre-com-ia/", {
        activeNav: "ia",
        title: "Encontre com IA (Demonstração local)",
        description:
          "Seletor guiado por necessidade e orçamento em demonstração local determinística, sem consulta ao vivo.",
        bodyHtml: renderAiFinderPage("../"),
        includeCatalogJson: true,
        jsonLd: [
          breadcrumbLd([
            { name: "Início", path: "/" },
            { name: "Encontre com IA", path: "/encontre-com-ia/" },
          ]),
        ],
      })
    );

    // 6. Community Directory
    publicRoutes.push(
      write("/grupos/", {
        activeNav: "grupos",
        title: "Grupos e canal no Telegram",
        description:
          "Diretório voluntário das comunidades do Procurali: canal oficial @procurali no Telegram e prévia de grupos no WhatsApp.",
        bodyHtml: renderGroupsPage("../"),
        jsonLd: [breadcrumbLd([{ name: "Início", path: "/" }, { name: "Grupos", path: "/grupos/" }])],
      })
    );

    // 7. Trust & Contact pages
    const trustPages = [
      ["sobre", "Sobre o Procurali", "Conheça o papel editorial do Procurali e sua separação das lojas parceiras."],
      ["como-selecionamos", "Como selecionamos", "Política editorial, janela de frescor de 30 minutos e regra de amostra de 7 dias."],
      ["transparencia", "Transparência de afiliados", "Como o Procurali financia o projeto, uso de #publi e links rel=sponsored."],
      ["privacidade", "Privacidade (rascunho)", "Rascunho de política de privacidade e controle opcional de medição local."],
      ["termos", "Termos de uso (rascunho)", "Rascunho dos termos informativos e limites de responsabilidade sobre pedidos."],
      ["contato", "Contato", "Rotas de contato e formulário demonstrativo não conectado do Procurali."],
    ];
    for (const [slug, title, description] of trustPages) {
      const route = `/${slug}/`;
      publicRoutes.push(
        write(route, {
          title,
          description,
          bodyHtml: renderTrustPage("../", slug),
          jsonLd: [breadcrumbLd([{ name: "Início", path: "/" }, { name: title, path: route }])],
        })
      );
    }

    // 8. Creative Kit & Non-production CMS mockup (excluded from sitemap)
    publicRoutes.push(
      write("/criativos/", {
        title: "Kit criativo e identidade visual",
        description: "Logotipos, templates SVG determinísticos, roteiro vertical e rascunhos Telegram/WhatsApp.",
        bodyHtml: renderCreativeKitPage("../"),
      })
    );

    write("/cms-preview/", {
      title: "Mockup de fila editorial e contratos (não produtivo)",
      description: "Prévia visual não produtiva das aprovações, teto de IA e Search Console.",
      bodyHtml: renderCmsPreviewPage("../"),
    });

    write("/404.html", {
      title: "Página não encontrada (404)",
      description: "Página não encontrada neste protótipo do Procurali.",
      bodyHtml: renderNotFoundPage("./"),
    });

    // Sitemap (excludes /cms-preview/ and /404.html)
    const sitemapXml = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${publicRoutes
  .filter((r) => r !== "/criativos/")
  .map((r) => `  <url>\n    <loc>${canonicalUrl(r)}</loc>\n    <lastmod>2026-11-15</lastmod>\n  </url>`)
  .join("\n")}
</urlset>
`;
    fs.writeFileSync(path.join(siteDir, "sitemap.xml"), sitemapXml, "utf8");

    // Staging robots.txt keeps demo staging out of search indexes until production launch
    const robotsTxt = `# Procurali — Ambiente de protótipo / staging
# Em staging de demonstração, manter indexação bloqueada.
# Antes do lançamento em procurali.com.br, substituir por:
# User-agent: *
# Allow: /
# Disallow: /cms-preview/
# Sitemap: https://procurali.com.br/sitemap.xml

User-agent: *
Disallow: /
`;
    fs.writeFileSync(path.join(siteDir, "robots.txt"), robotsTxt, "utf8");

    console.log(
      `Built Procurali prototype cleanly into ${siteDir} (${publicRoutes.length + 2} HTML pages, source=${catalog.sourceLabel}).`
    );
    return { siteDir, pageCount: publicRoutes.length + 2, catalog };
  } finally {
    setActiveCatalog(null);
  }
}

const isEntrypoint =
  Boolean(process.argv[1]) &&
  pathToFileURL(fs.realpathSync(path.resolve(process.argv[1]))).href === import.meta.url;

if (isEntrypoint) {
  buildSite(parseCliArgs(process.argv.slice(2))).catch((err) => {
    console.error(err);
    process.exitCode = 1;
  });
}
