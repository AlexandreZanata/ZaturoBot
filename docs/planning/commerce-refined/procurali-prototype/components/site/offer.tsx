import Link from "next/link";
import { ArrowUpRight } from "lucide-react";
import { SITE_META } from "@/data/demo-catalog.mjs";
import { buildOfficialAffiliateAnchorProps, formatBrl, resolveMediaSrc, verifySevenDaySample } from "@/lib/contracts.mjs";
import type { MediaRight, Offer, ShowcaseCard } from "@/lib/types";
import { Badge, buttonClass } from "@/components/site/primitives";
import { SaveButton } from "@/components/site/saved";
import { CopyButton } from "@/components/site/copy-button";
import { cn } from "@/lib/utils";

/** Media paths are site-absolute ("/brand/...") or validated https URLs. */
export function mediaSrc(path: string) {
  return resolveMediaSrc("/", path);
}

export function offerHref(slug: string) {
  return `/ofertas/${slug}/`;
}

export function StatusBadge({ offer }: { offer: Offer }) {
  if (offer.state === "approved") {
    const sevenDay = verifySevenDaySample(offer);
    const suffix = sevenDay.eligible ? " · Série 7 dias" : "";
    return offer.isDemoFixture ? (
      <Badge tone="petrol">Ativa (demo){suffix}</Badge>
    ) : (
      <Badge tone="emerald">
        Ativa · {offer.marketplaceLabel}
        {suffix}
      </Badge>
    );
  }
  if (offer.state === "stale") return <Badge tone="warning">Preço em atualização (&gt;30 min)</Badge>;
  if (offer.state === "expired") return <Badge tone="neutral">Prazo encerrado (histórico)</Badge>;
  if (offer.state === "missing_rights") return <Badge tone="danger">Direitos de mídia pendentes</Badge>;
  return <Badge tone="neutral">Indisponível</Badge>;
}

export function PriceBlock({ offer, size = "md" }: { offer: Offer; size?: "md" | "lg" }) {
  const ref = offer.referencePriceCents ? formatBrl(offer.referencePriceCents) : "";
  const savings =
    offer.referencePriceCents && offer.referencePriceCents > offer.priceBrlCents
      ? offer.referencePriceCents - offer.priceBrlCents
      : 0;
  return (
    <div>
      <p className="flex flex-wrap items-baseline gap-x-2.5">
        <span className={cn("font-semibold tracking-tight text-ink tabular-nums", size === "lg" ? "text-4xl" : "text-xl")}>
          {formatBrl(offer.priceBrlCents)}
        </span>
        {ref && <span className="text-sm text-muted line-through tabular-nums">De {ref}</span>}
      </p>
      {(offer.discountPercentLabel || savings > 0) && (
        <p className="mt-1 text-sm text-emerald-700">
          {offer.discountPercentLabel}
          {offer.discountPercentLabel && savings > 0 && size === "lg" ? " · " : ""}
          {savings > 0 && (size === "lg" || !offer.discountPercentLabel)
            ? `Economia ${size === "lg" ? "observada " : ""}de ${formatBrl(savings)}`
            : ""}
        </p>
      )}
    </div>
  );
}

/** Outbound purchase action. Only approved real offers with verified media rights get a link. */
export function AffiliateCta({
  offer,
  mediaRights,
  className,
}: {
  offer: Offer;
  mediaRights: MediaRight[];
  className?: string;
  withIcon?: boolean;
}) {
  const cta = buildOfficialAffiliateAnchorProps(offer, mediaRights);
  if (cta.enabled && cta.href) {
    return (
      <a
        href={cta.href}
        rel={cta.rel ?? "sponsored noopener"}
        target={cta.target ?? "_blank"}
        data-cta="affiliate"
        data-analytics-event="outbound_offer_click"
        data-analytics-detail={offer.slug}
        className={buttonClass("action", className)}
      >
        {cta.label}
        <ArrowUpRight className="h-4 w-4" aria-hidden="true" />
      </a>
    );
  }
  return (
    <button type="button" disabled aria-disabled="true" data-cta="disabled" className={buttonClass("disabled", className)}>
      {cta.label}
    </button>
  );
}

function Thumb({ src, alt, href, muted, className }: { src: string; alt: string; href: string; muted?: boolean; className?: string }) {
  return (
    <Link href={href} tabIndex={-1} aria-hidden="true" className={cn("block overflow-hidden rounded-lg bg-tile", className)}>
      <img
        src={src}
        alt={alt}
        width={560}
        height={560}
        loading="lazy"
        className={cn(
          "h-full w-full object-contain p-[12%] mix-blend-multiply transition-transform duration-500 ease-out group-hover:scale-[1.04]",
          muted && "opacity-60 grayscale"
        )}
      />
    </Link>
  );
}

/** Offer as a list row separated by hairlines (listings, categories, filters). */
export function OfferRow({
  offer,
  mediaRights,
  headingLevel = 3,
}: {
  offer: Offer;
  mediaRights: MediaRight[];
  headingLevel?: 2 | 3;
}) {
  const Heading = headingLevel === 2 ? "h2" : "h3";
  const sevenDay = verifySevenDaySample(offer);
  const cta = buildOfficialAffiliateAnchorProps(offer, mediaRights);
  const price = formatBrl(offer.priceBrlCents);
  const muted = offer.state !== "approved";
  return (
    <article className="group grid gap-x-6 gap-y-4 border-t border-line py-7 sm:grid-cols-[150px_1fr] lg:grid-cols-[170px_1fr_220px]">
      <Thumb src={mediaSrc(offer.imagePath)} alt={offer.imageAlt} href={offerHref(offer.slug)} muted={muted} className="aspect-square w-32 sm:w-auto" />
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
          <StatusBadge offer={offer} />
          {offer.sponsored && <Badge tone="warning">Patrocinado no ML</Badge>}
          <span className="text-xs text-muted">{offer.merchant}</span>
        </div>
        <Heading className="mt-2 text-lg leading-snug font-semibold text-ink">
          <Link href={offerHref(offer.slug)} className="hover:underline hover:decoration-neutral-300 hover:underline-offset-4">
            {offer.title}
          </Link>
        </Heading>
        <p className="mt-1.5 line-clamp-2 max-w-xl text-sm leading-relaxed text-muted">{offer.oneLineReason}</p>
        <p className="mt-3 text-xs text-muted">
          {offer.brand && offer.model ? `${offer.brand} · ${offer.model} · ` : ""}Variante: {offer.exactVariant}
        </p>
        {offer.marketplaceRating && (
          <p className="mt-1 text-xs text-muted">
            Anúncio ML: {offer.marketplaceRating} ★{offer.marketplaceSalesLabel ? ` (${offer.marketplaceSalesLabel})` : ""}
            {offer.returnDays ? ` · Devolução grátis ${offer.returnDays}d` : ""}
          </p>
        )}
        {sevenDay.eligible && <p className="mt-1 text-xs text-emerald-700">{sevenDay.label}</p>}
        {offer.couponCode && (
          <div className="mt-3">
            <CopyButton pill text={offer.couponCode} label={<>Copiar cupom: <strong>{offer.couponCode}</strong></>} />
          </div>
        )}
      </div>
      <div className="flex flex-col gap-1.5 sm:col-start-2 lg:col-start-3">
        <PriceBlock offer={offer} />
        <p className="text-xs text-muted">{offer.installmentSummary || offer.shortCondition || offer.paymentConditions}</p>
        {offer.validUntilDisplay && <p className="text-xs text-amber-800">{offer.validUntilDisplay}</p>}
        <p className="text-xs text-muted">Obs. {offer.observedAtDisplay} · frete não incluído</p>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          {cta.enabled ? (
            <AffiliateCta offer={offer} mediaRights={mediaRights} className="flex-1" />
          ) : (
            <Link href={offerHref(offer.slug)} className={buttonClass("secondary", "flex-1")}>
              {offer.state === "approved" ? "Ver ficha" : "Conferir status do item"}
            </Link>
          )}
          <SaveButton slug={offer.slug} title={offer.title} priceLabel={price} variant="icon" />
        </div>
        {cta.enabled && (
          <p className="text-[0.7rem] text-muted">
            Link de afiliado · podemos receber comissão.{" "}
            <Link href={offerHref(offer.slug)} className="text-ink underline decoration-neutral-300 underline-offset-2">
              Ficha técnica
            </Link>
          </p>
        )}
      </div>
    </article>
  );
}

/** Showcase card for the home page: image tile, title, price and one action. */
export function ShowcaseOfferCard({ card }: { card: ShowcaseCard }) {
  const href = offerHref(card.slug);
  const price = formatBrl(card.priceCents);
  const live =
    !card.isDemoFixture && !card.isStale && typeof card.officialAffiliateUrl === "string" && card.officialAffiliateUrl.startsWith("https://");
  return (
    <article className="group flex h-full flex-col">
      <div className="relative">
        <Thumb src={mediaSrc(card.imagePath)} alt={card.imageAlt} href={href} muted={card.isStale} className="aspect-[4/5]" />
        <SaveButton slug={card.slug} title={card.shortTitle} priceLabel={price} variant="icon" className="absolute top-1.5 right-1.5 bg-transparent" />
      </div>
      <p className="mt-4 text-xs text-muted">
        {card.marketplaceLabel || "Shopee"}
        {card.topBadge ? ` · ${card.topBadge}` : ""}
        {card.sponsored ? " · Patrocinado" : ""}
      </p>
      <h3 className="mt-1 line-clamp-2 text-[0.95rem] leading-snug font-medium text-ink">
        <Link href={href} className="hover:underline hover:decoration-neutral-300 hover:underline-offset-4">
          {card.shortTitle}
        </Link>
      </h3>
      <p className="mt-2 flex flex-wrap items-baseline gap-x-2">
        <span className="text-lg font-semibold tracking-tight text-ink tabular-nums">{price}</span>
        {card.oldPriceCents ? <span className="text-sm text-muted line-through tabular-nums">{formatBrl(card.oldPriceCents)}</span> : null}
      </p>
      <p className="mt-0.5 text-xs text-muted">{card.installmentSummary || card.shortCondition || card.storeLabel}</p>
      {card.ratingText && (
        <p className="mt-0.5 text-xs text-muted" title="Avaliação no anúncio do Mercado Livre">
          ★ {card.ratingText}
          {card.salesLabel ? ` · ${card.salesLabel}` : ""}
        </p>
      )}
      {card.couponCode && (
        <div className="mt-2">
          <CopyButton pill text={card.couponCode} label={<>Copiar: <strong>{card.couponCode}</strong></>} />
        </div>
      )}
      {card.deadlineText && <p className="mt-1 text-xs text-amber-800">{card.deadlineText}</p>}
      <div className="mt-auto pt-4">
        {live ? (
          <>
            <a
              href={card.officialAffiliateUrl!}
              rel="sponsored noopener"
              target="_blank"
              data-cta="affiliate"
              data-analytics-event="outbound_offer_click"
              data-analytics-detail={card.slug}
              className="inline-flex min-h-11 items-center gap-1 text-sm font-medium text-ink underline decoration-neutral-300 underline-offset-4 hover:decoration-ink"
            >
              {card.ctaLabel || "Ver oferta no Mercado Livre"}
              <ArrowUpRight className="h-4 w-4" aria-hidden="true" />
            </a>
            <p className="text-[0.7rem] text-muted">
              Link de afiliado · {card.checkedLabel || "Conferido hoje"}
            </p>
          </>
        ) : (
          <Link
            href={href}
            className="inline-flex min-h-11 items-center text-sm font-medium text-ink underline decoration-neutral-300 underline-offset-4 hover:decoration-ink"
          >
            {card.ctaLabel || "Ver oferta"}
          </Link>
        )}
      </div>
    </article>
  );
}

export function MiniOfferCard({ offer, children }: { offer: Offer; children?: React.ReactNode }) {
  return (
    <article className="group flex gap-4 border-t border-line py-5">
      <Thumb src={mediaSrc(offer.imagePath)} alt={offer.imageAlt} href={offerHref(offer.slug)} className="h-20 w-20 shrink-0" />
      <div className="min-w-0 flex-1">
        <StatusBadge offer={offer} />
        <h3 className="mt-1 line-clamp-2 font-medium text-ink">
          <Link href={offerHref(offer.slug)} className="hover:underline hover:decoration-neutral-300 hover:underline-offset-4">
            {offer.title}
          </Link>
        </h3>
        <p className="line-clamp-1 text-xs text-muted">{offer.exactVariant}</p>
        <p className="mt-1 font-semibold text-ink tabular-nums">{formatBrl(offer.priceBrlCents)}</p>
        {children}
      </div>
    </article>
  );
}

export const AFFILIATE_DISCLOSURE = SITE_META.affiliateDisclosure;
