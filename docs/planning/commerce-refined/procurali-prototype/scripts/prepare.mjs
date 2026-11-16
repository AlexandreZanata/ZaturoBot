/**
 * Procurali — build-time data preparation for the Next.js static export.
 *
 * Resolves the catalog (static demo fixtures, a Hub snapshot file, or the authenticated Hub
 * site-catalog API), validates it against the shared contracts, and writes:
 *   - .generated/catalog.json         (read by the Next.js pages at build time)
 *   - public/api/catalog.json         (machine-readable snapshot)
 *   - public/api/whatsapp-assisted-drafts.json
 *   - public/sitemap.xml, public/robots.txt (staging keeps indexing blocked)
 *   - public/brand/**, public/fonts/** (copied brand pack)
 *
 * Usage: node scripts/prepare.mjs [--api-snapshot file.json] [--hub-url https://hub]
 * Environment: PROCURALI_API_SNAPSHOT, PROCURALI_HUB_API_URL, PROCURALI_HUB_TOKEN.
 */

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
import { buildWhatsappWebAssistedPackage, validateCatalogSnapshot } from "../lib/contracts.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function copyTree(srcDir, dstDir) {
  fs.mkdirSync(dstDir, { recursive: true });
  for (const entry of fs.readdirSync(srcDir, { withFileTypes: true })) {
    const srcPath = path.join(srcDir, entry.name);
    const dstPath = path.join(dstDir, entry.name);
    if (entry.isDirectory()) copyTree(srcPath, dstPath);
    else if (entry.isFile()) fs.copyFileSync(srcPath, dstPath);
  }
}

export function canonicalUrl(routePath) {
  const clean = routePath === "/" ? "/" : `/${routePath.replace(/^\/+|\/+$/g, "")}/`;
  return `https://${SITE_META.domain}${clean}`;
}

function mergeCatalogSnapshot(snapshot, sourceLabel) {
  const snapshotCats = Array.isArray(snapshot.categories) ? snapshot.categories : [];
  const seenCats = new Set(snapshotCats.map((c) => c.slug));
  return {
    schemaVersion: snapshot.schemaVersion || 1,
    generatedAtUtc: snapshot.generatedAtUtc || new Date().toISOString(),
    sourceLabel,
    offers: snapshot.offers || [],
    categories: [...snapshotCats, ...CATEGORIES.filter((c) => !seenCats.has(c.slug))],
    mediaRights:
      Array.isArray(snapshot.mediaRights) && snapshot.mediaRights.length > 0
        ? snapshot.mediaRights
        : MEDIA_RIGHTS,
    evidenceSources:
      Array.isArray(snapshot.evidenceSources) && snapshot.evidenceSources.length > 0
        ? snapshot.evidenceSources
        : EVIDENCE_SOURCES,
  };
}

function checked(merged, what) {
  const check = validateCatalogSnapshot(merged);
  if (!check.valid) throw new Error(`Invalid ${what}: ${check.errors.join("; ")}`);
  return merged;
}

export async function resolveCatalog(options = {}) {
  if (options.apiSnapshotPath) {
    const raw = JSON.parse(fs.readFileSync(options.apiSnapshotPath, "utf8"));
    return checked(
      mergeCatalogSnapshot(raw, `api-snapshot:${path.basename(options.apiSnapshotPath)}`),
      "API catalog snapshot file"
    );
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
    if (!res.ok) throw new Error(`Hub site-catalog fetch failed with HTTP ${res.status}`);
    return checked(mergeCatalogSnapshot(await res.json(), "hub-api"), "Hub site-catalog payload");
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

/** Public routes listed in the sitemap (excludes /cms-preview/, /criativos/ and 404). */
export function sitemapRoutes(catalog) {
  return [
    "/",
    "/ofertas/",
    ...catalog.offers.map((o) => `/ofertas/${o.slug}/`),
    "/categorias/",
    ...catalog.categories.map((c) => `/categorias/${c.slug}/`),
    "/guias/",
    ...GUIDES.map((g) => `/guias/${g.slug}/`),
    "/selecoes-da-semana/",
    "/encontre-com-ia/",
    "/grupos/",
    "/sobre/",
    "/como-selecionamos/",
    "/transparencia/",
    "/privacidade/",
    "/termos/",
    "/contato/",
  ];
}

const ROBOTS_TXT = `# Procurali — Ambiente de protótipo / staging
# Em staging de demonstração, manter indexação bloqueada.
# Antes do lançamento em procurali.com.br, substituir por:
# User-agent: *
# Allow: /
# Disallow: /cms-preview/
# Sitemap: https://procurali.com.br/sitemap.xml

User-agent: *
Disallow: /
`;

export async function prepare(options = {}) {
  const generatedDir = options.generatedDir ?? path.join(ROOT, ".generated");
  const publicDir = options.publicDir ?? path.join(ROOT, "public");
  const catalog = await resolveCatalog(options);

  fs.mkdirSync(generatedDir, { recursive: true });
  fs.writeFileSync(path.join(generatedDir, "catalog.json"), JSON.stringify(catalog), "utf8");

  const apiDir = path.join(publicDir, "api");
  fs.rmSync(apiDir, { recursive: true, force: true });
  fs.mkdirSync(apiDir, { recursive: true });
  fs.writeFileSync(
    path.join(apiDir, "catalog.json"),
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
    path.join(apiDir, "whatsapp-assisted-drafts.json"),
    JSON.stringify(
      buildWhatsappWebAssistedPackage(catalog.offers, catalog.generatedAtUtc, `https://${SITE_META.domain}`),
      null,
      2
    ),
    "utf8"
  );

  const sitemap = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${sitemapRoutes(catalog)
  .map((r) => `  <url>\n    <loc>${canonicalUrl(r)}</loc>\n    <lastmod>2026-11-15</lastmod>\n  </url>`)
  .join("\n")}
</urlset>
`;
  fs.writeFileSync(path.join(publicDir, "sitemap.xml"), sitemap, "utf8");
  fs.writeFileSync(path.join(publicDir, "robots.txt"), ROBOTS_TXT, "utf8");

  fs.rmSync(path.join(publicDir, "brand"), { recursive: true, force: true });
  copyTree(path.join(ROOT, "brand"), path.join(publicDir, "brand"));

  console.log(
    `Prepared Procurali catalog (source=${catalog.sourceLabel}, ${catalog.offers.length} offers) and public assets.`
  );
  return catalog;
}

function parseCliArgs(argv) {
  const opts = {
    apiSnapshotPath: process.env.PROCURALI_API_SNAPSHOT
      ? path.resolve(process.env.PROCURALI_API_SNAPSHOT)
      : null,
    hubApiUrl: process.env.PROCURALI_HUB_API_URL || null,
    hubToken: process.env.PROCURALI_HUB_TOKEN || null,
  };
  for (let i = 0; i < argv.length; i += 1) {
    if (argv[i] === "--api-snapshot" && argv[i + 1]) {
      opts.apiSnapshotPath = path.resolve(argv[(i += 1)]);
    } else if (argv[i] === "--hub-url" && argv[i + 1]) {
      opts.hubApiUrl = argv[(i += 1)];
    }
  }
  return opts;
}

if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) {
  prepare(parseCliArgs(process.argv.slice(2))).catch((err) => {
    console.error(err);
    process.exitCode = 1;
  });
}
