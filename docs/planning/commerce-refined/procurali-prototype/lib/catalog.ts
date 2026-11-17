import fs from "node:fs";
import path from "node:path";
import { HOME_SHOWCASE_META } from "@/data/demo-catalog.mjs";
import type { Catalog, Offer, ShowcaseCard } from "@/lib/types";

let cached: Catalog | null = null;

/** Catalog resolved by scripts/prepare.mjs (demo fixtures or a validated Hub snapshot). */
export function getCatalog(): Catalog {
  if (!cached) {
    const file = path.join(process.cwd(), ".generated", "catalog.json");
    if (!fs.existsSync(file)) {
      throw new Error("Missing .generated/catalog.json — run `node scripts/prepare.mjs` first.");
    }
    cached = JSON.parse(fs.readFileSync(file, "utf8")) as Catalog;
  }
  return cached;
}

export function getOffer(slug: string): Offer | undefined {
  return getCatalog().offers.find((o) => o.slug === slug);
}

function cardsFromCatalog(offers: Offer[]): ShowcaseCard[] {
  return offers
    .filter((o) => o.state === "approved" || o.state === "stale")
    .slice(0, 8)
    .map((o, idx) => {
      const isStale = o.state === "stale";
      return {
        slug: o.slug,
        marketplaceLabel: o.marketplaceLabel || "Mercado Livre",
        topBadge: o.cardBadge || (idx === 0 ? "Oferta em destaque" : "Achado do dia"),
        badgeTone: isStale ? "warning" : idx === 0 ? "emerald" : "neutral",
        isFeaturedCard: idx === 0 && !isStale,
        isDemoFixture: Boolean(o.isDemoFixture),
        isStale,
        sponsored: Boolean(o.sponsored),
        imagePath: o.imagePath,
        imageAlt: o.imageAlt,
        shortTitle: o.title,
        brandModelLine: [o.brand, o.model].filter(Boolean).join(" · "),
        ratingText: o.marketplaceRating || "",
        salesLabel: o.marketplaceSalesLabel || "",
        shortUse: o.oneLineReason || "",
        priceCents: o.priceBrlCents,
        oldPriceCents: o.referencePriceCents || null,
        installmentSummary: o.installmentSummary || "",
        shortCondition: o.shortCondition || o.paymentConditions || "",
        mintText: o.discountPercentLabel || o.supportedBenefits?.[0] || "",
        couponCode: o.couponCode || null,
        deadlineText: o.priceValidUntil ? `Válido até ${o.priceValidUntil}` : null,
        checkedLabel: "Conferido hoje",
        storeLabel: o.storeShort || o.merchant || "Mercado Livre",
        officialAffiliateUrl: !o.isDemoFixture && !isStale ? o.officialAffiliateUrl : null,
        ctaLabel: isStale
          ? "Conferir status do item"
          : o.marketplaceLabel === "Mercado Livre"
            ? "Ver oferta no Mercado Livre"
            : "Ver oferta na Shopee",
      };
    });
}

/** Home showcase: curated demo cards, or cards derived from a Hub snapshot. */
export function getShowcaseCards(): ShowcaseCard[] {
  const catalog = getCatalog();
  return catalog.sourceLabel === "static-demo"
    ? (HOME_SHOWCASE_META.dailyCards as ShowcaseCard[])
    : cardsFromCatalog(catalog.offers);
}
