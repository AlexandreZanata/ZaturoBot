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

/**
 * Product states the public site may render. Hub products still waiting for the owner's
 * approval (`candidate`) or withdrawn are never published, not even as unavailable pages.
 */
const PUBLISHABLE_STATES = new Set(["approved", "stale", "expired", "missing_rights"]);

function mergeCatalogSnapshot(snapshot, sourceLabel) {
  const snapshotCats = Array.isArray(snapshot.categories) ? snapshot.categories : [];
  const seenCats = new Set(snapshotCats.map((c) => c.slug));
  return {
    schemaVersion: snapshot.schemaVersion || 1,
    generatedAtUtc: snapshot.generatedAtUtc || new Date().toISOString(),
    sourceLabel,
    offers: (snapshot.offers || []).filter((o) => PUBLISHABLE_STATES.has(o?.state)),
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

/** Offer pages that search engines may index: real, approved Mercado Livre listings only. */
export function isIndexableOffer(offer) {
  return !offer.isDemoFixture && offer.state === "approved";
}

function isoDate(value, fallback) {
  const ms = Date.parse(value || "");
  return Number.isFinite(ms) ? new Date(ms).toISOString().slice(0, 10) : fallback;
}

/**
 * Sitemap entries with real last-modified dates. Excludes /cms-preview/, /criativos/, 404,
 * demo fixtures, unavailable offers and categories without any indexable offer.
 */
export function sitemapEntries(catalog) {
  const catalogDate = isoDate(catalog.generatedAtUtc, new Date().toISOString().slice(0, 10));
  const offers = catalog.offers.filter(isIndexableOffer);
  const offerDate = (o) => isoDate(o.observedAt, catalogDate);
  const newestIn = (list) => list.map(offerDate).sort().at(-1) || catalogDate;
  const listed = new Set(offers.map((o) => o.categorySlug));
  const page = (route, lastmod = catalogDate, priority = "0.5", changefreq = "weekly") => ({ route, lastmod, priority, changefreq });
  return [
    page("/", newestIn(offers), "1.0", "daily"),
    page("/ofertas/", newestIn(offers), "0.9", "daily"),
    ...offers.map((o) => page(`/ofertas/${o.slug}/`, offerDate(o), "0.8", "daily")),
    page("/categorias/", catalogDate, "0.6"),
    ...catalog.categories
      .filter((c) => listed.has(c.slug))
      .map((c) => page(`/categorias/${c.slug}/`, newestIn(offers.filter((o) => o.categorySlug === c.slug)), "0.7", "daily")),
    page("/guias/", catalogDate, "0.7"),
    ...GUIDES.map((g) => page(`/guias/${g.slug}/`, catalogDate, "0.7", "monthly")),
    page("/selecoes-da-semana/", catalogDate, "0.6"),
    page("/encontre-com-ia/", catalogDate, "0.4", "monthly"),
    page("/grupos/", catalogDate, "0.4", "monthly"),
    ...["/sobre/", "/como-selecionamos/", "/transparencia/", "/privacidade/", "/termos/", "/contato/"].map((r) =>
      page(r, catalogDate, "0.3", "yearly")
    ),
  ];
}

/** Public routes listed in the sitemap. */
export function sitemapRoutes(catalog) {
  return sitemapEntries(catalog).map((e) => e.route);
}

const ROBOTS_TXT = `# Procurali — guia independente de compras
User-agent: *
Allow: /
Disallow: /api/
# /cms-preview/ and /criativos/ stay crawlable so engines can read their noindex tag.

Sitemap: https://${SITE_META.domain}/sitemap.xml
`;

function xmlEscape(value) {
  return String(value).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

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
${sitemapEntries(catalog)
  .map(
    (e) =>
      `  <url>\n    <loc>${xmlEscape(canonicalUrl(e.route))}</loc>\n    <lastmod>${e.lastmod}</lastmod>\n    <changefreq>${e.changefreq}</changefreq>\n    <priority>${e.priority}</priority>\n  </url>`
  )
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
