import type { Metadata } from "next";
import { SITE_META } from "@/data/demo-catalog.mjs";

export function canonicalUrl(routePath: string) {
  const clean = routePath === "/" ? "/" : `/${routePath.replace(/^\/+|\/+$/g, "")}/`;
  return `https://${SITE_META.domain}${clean}`;
}

export function absoluteOgImage(ogImage?: string | null) {
  if (ogImage && /^https?:\/\//.test(ogImage)) return ogImage;
  if (ogImage && ogImage.trim()) return `https://${SITE_META.domain}/${ogImage.replace(/^\/+/, "")}`;
  return `https://${SITE_META.domain}/brand/png/social-cover-1200x630.png`;
}

/** Per-route metadata with canonical URL and Open Graph (absolute og:image for WhatsApp previews). */
export function pageMetadata({
  route,
  title,
  description,
  ogImage,
}: {
  route: string;
  title: string;
  description: string;
  ogImage?: string | null;
}): Metadata {
  const fullTitle = `${title} — Procurali`;
  return {
    title: { absolute: fullTitle },
    description,
    alternates: { canonical: canonicalUrl(route) },
    openGraph: {
      type: "website",
      locale: "pt_BR",
      title: fullTitle,
      description,
      url: canonicalUrl(route),
      images: [absoluteOgImage(ogImage)],
    },
  };
}

export function breadcrumbLd(items: { name: string; path: string }[]) {
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

export const organizationLd = {
  "@context": "https://schema.org",
  "@type": "Organization",
  name: "Procurali (Procura Ali)",
  url: `https://${SITE_META.domain}/`,
  sameAs: [SITE_META.telegramUrl],
};
