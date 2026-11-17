import Link from "next/link";
import { WEEKLY_SELECTION } from "@/data/demo-catalog.mjs";
import { getCatalog } from "@/lib/catalog";
import { formatBrl } from "@/lib/contracts.mjs";
import { breadcrumbLd, pageMetadata } from "@/lib/seo";
import { JsonLd, Notice, PageHero, buttonClass } from "@/components/site/primitives";
import { mediaSrc } from "@/components/site/offer";

export const metadata = pageMetadata({
  route: "/selecoes-da-semana/",
  title: "Seleções da semana (demonstração)",
  description: "Recorte semanal datado com escolhas distintas e separação de históricos encerrados.",
});

export default function WeeklyPage() {
  const offers = getCatalog().offers;
  const expired = offers.find((o) => o.slug === WEEKLY_SELECTION.expiredHistoricalReference.offerSlug);
  return (
    <>
      <JsonLd
        data={[breadcrumbLd([{ name: "Início", path: "/" }, { name: "Seleções da semana", path: "/selecoes-da-semana/" }])]}
      />
      <PageHero
        crumbs={[{ label: "Início", href: "/" }, { label: "Seleções da semana" }]}
        kicker={WEEKLY_SELECTION.dateRangeLabel}
        title={WEEKLY_SELECTION.title}
        lead={WEEKLY_SELECTION.scopeExplanation}
      />
      <div className="container-site space-y-12 py-10">
        <ol className="grid gap-x-8 gap-y-14 md:grid-cols-3">
          {WEEKLY_SELECTION.picks.map((pick, idx) => {
            const offer = offers.find((o) => o.slug === pick.offerSlug);
            if (!offer) return null;
            return (
              <li key={pick.offerSlug} className="group relative flex flex-col">
                <span className="absolute top-3 left-4 z-10 font-mono text-xs text-muted">{String(idx + 1).padStart(2, "0")}</span>
                <div className="aspect-square overflow-hidden rounded-lg bg-tile">
                  <img src={mediaSrc(offer.imagePath)} alt={offer.imageAlt} width={480} height={360} loading="lazy" className="h-full w-full object-contain p-[14%] mix-blend-multiply" />
                </div>
                <div className="flex flex-1 flex-col pt-4">
                  <span className="text-xs text-muted">{pick.roleLabel}</span>
                  <h2 className="mt-1.5 text-lg leading-snug font-medium text-ink">
                    <Link href={`/ofertas/${offer.slug}/`} className="hover:underline">
                      {offer.title}
                    </Link>
                  </h2>
                  <p className="mt-1 text-xs text-muted">{offer.exactVariant}</p>
                  <p className="mt-3 text-xl font-semibold text-ink tabular-nums">{formatBrl(offer.priceBrlCents)}</p>
                  <p className="mt-1 inline-flex items-center gap-1 text-xs text-muted">
                    Obs.: {offer.observedAtDisplay}
                  </p>
                  <p className="mt-3 text-sm text-neutral-700">{pick.note}</p>
                  <Link href={`/ofertas/${offer.slug}/`} className={buttonClass("secondary", "mt-5 w-full")}>
                    Ver critérios e ficha
                  </Link>
                </div>
              </li>
            );
          })}
        </ol>
        {expired && (
          <section>
            <h2 className="mb-4 flex items-center gap-2 text-xl font-semibold text-ink">
              {WEEKLY_SELECTION.expiredHistoricalReference.label}
            </h2>
            <Notice variant="warning">
              <p>
                <strong>{expired.title}</strong> ({formatBrl(expired.priceBrlCents)} em {expired.observedAtDisplay}) —{" "}
                {WEEKLY_SELECTION.expiredHistoricalReference.note}
              </p>
              <Link href={`/ofertas/${expired.slug}/`} className="font-semibold underline">
                Consultar registro histórico separado
              </Link>
            </Notice>
          </section>
        )}
      </div>
    </>
  );
}
