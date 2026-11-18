import Link from "next/link";
import { ArrowRight, ArrowUpRight, Check } from "lucide-react";
import { formatBrl } from "@/lib/contracts.mjs";
import { GUIDE_VISUALS, GUIDES, SITE_META } from "@/data/demo-catalog.mjs";
import { getCatalog, getShowcaseCards } from "@/lib/catalog";
import { organizationLd, pageMetadata, websiteLd } from "@/lib/seo";
import { Eyebrow, JsonLd, MoreLink, SectionTitle } from "@/components/site/primitives";
import { ShowcaseOfferCard, mediaSrc } from "@/components/site/offer";
import { PlaceholdersAndVanishInput } from "@/components/ui/placeholders-and-vanish-input";
import { HoverEffect } from "@/components/ui/card-hover-effect";

export const metadata = pageMetadata({
  route: "/",
  title: "Procura Ali · Ofertas e guias para escolher com mais contexto",
  description:
    "Procurali (Procura Ali): guias de compra, curadoria de ofertas com condições explicadas e comunidade no Telegram em português.",
});

const QUICK_SEARCHES: [string, string][] = [
  ["SSD NVMe", "ssd nvme"],
  ["Teclado mecânico", "teclado mecânico"],
  ["Monitor 200 Hz", "monitor 200hz"],
  ["Headset", "headset"],
];

const HERO_FACTS = ["Preço conferido com horário", "Link de afiliado sempre sinalizado", "Sem cadastro"];

const PRINCIPLES = [
  ["Pesquisa e critério técnico", "Especificações conferidas com foco em uso real."],
  ["Horário visível no card", "Itens com mais de 30 min entram em atualização e pausam o clique."],
  ["Afiliado transparente", "Você sabe antes do clique quando há comissão possível."],
  ["Sem pressão fabricada", "Prazos e cupons só aparecem quando realmente confirmados."],
];

type Visual = { thumbPath?: string; thumbAlt?: string; badgeLabel?: string; homeTitle?: string; readingMinutes?: number };

export default function HomePage() {
  const catalog = getCatalog();
  const cards = getShowcaseCards();
  const liveCount = catalog.offers.filter((o) => !o.isDemoFixture && o.state === "approved").length;
  const radar = catalog.offers
    .filter((o) => o.state === "approved")
    .map((offer) => ({
      offer,
      pct:
        offer.referencePriceCents && offer.referencePriceCents > offer.priceBrlCents
          ? Math.round((1 - offer.priceBrlCents / offer.referencePriceCents) * 100)
          : 0,
    }))
    .sort((a, b) => Number(a.offer.isDemoFixture) - Number(b.offer.isDemoFixture) || b.pct - a.pct)
    .slice(0, 4);
  const radarStamp = radar[0] ? `Observado ${radar[0].offer.observedAtDisplay.split(" (")[0]}` : "";

  return (
    <>
      <JsonLd data={[websiteLd, organizationLd]} />

      <section aria-labelledby="hero-heading" className="container-site pt-10 pb-14 sm:pt-16 sm:pb-20">
        <div className="grid items-center gap-x-16 gap-y-12 lg:grid-cols-[minmax(0,1fr)_420px] xl:grid-cols-[minmax(0,1fr)_460px]">
          <div className="min-w-0">
            <Eyebrow className="flex items-center gap-2">
              <span className="h-1.5 w-1.5 rounded-full bg-petrol-600" aria-hidden="true" />
              Guia independente de compras
            </Eyebrow>
            <h1
              id="hero-heading"
              className="mt-5 max-w-2xl text-[2.25rem] leading-[1.04] font-semibold tracking-[-0.035em] text-balance text-ink sm:text-5xl lg:text-[3.5rem]"
            >
              Encontre o que vale a pena para você.
            </h1>
            <p className="mt-5 max-w-lg text-[1.0625rem] leading-relaxed text-muted">
              Ofertas e guias para escolher com mais contexto. Preço conferido, condição explicada e comissão sempre
              sinalizada.
            </p>
            <div className="mt-8 max-w-xl">
              <PlaceholdersAndVanishInput
                action="/ofertas/"
                inputId="opening-search-input"
                label="Busque por produto, categoria ou oferta"
                placeholders={["SSD NVMe PCIe 4.0", "Teclado mecânico ABNT2", "Monitor 27 polegadas 200 Hz", "Headset com microfone"]}
              />
            </div>
            <div className="mt-4 flex max-w-xl flex-wrap items-center gap-x-2 gap-y-2 text-sm">
              <span className="mr-1 text-muted">Buscas comuns</span>
              {QUICK_SEARCHES.map(([label, q]) => (
                <Link
                  key={q}
                  href={`/ofertas/?q=${encodeURIComponent(q)}`}
                  className="inline-flex min-h-9 items-center rounded-full border border-line bg-white px-3 text-[0.8125rem] text-neutral-700 transition-colors hover:border-ink hover:text-ink"
                >
                  {label}
                </Link>
              ))}
            </div>
            <ul className="mt-9 flex flex-wrap gap-x-7 gap-y-2 text-[0.8125rem] text-muted">
              {HERO_FACTS.map((fact) => (
                <li key={fact} className="flex items-center gap-2">
                  <Check className="h-3.5 w-3.5 text-petrol-600" aria-hidden="true" />
                  {fact}
                </li>
              ))}
            </ul>
          </div>

          {radar.length > 0 && (
            <aside aria-labelledby="radar-heading" className="hidden rounded-2xl border border-line bg-white lg:block">
              <div className="flex items-baseline justify-between gap-4 px-6 pt-5 pb-4">
                <h2 id="radar-heading" className="text-sm font-semibold text-ink">
                  Radar de preços
                </h2>
                <p className="text-xs text-muted">{radarStamp}</p>
              </div>
              <ul className="divide-y divide-line border-t border-line">
                {radar.map(({ offer, pct }) => (
                  <li key={offer.slug}>
                    <Link
                      href={`/ofertas/${offer.slug}/`}
                      className="group flex items-center gap-4 px-6 py-3.5 transition-colors hover:bg-canvas"
                    >
                      <span className="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-tile">
                        <img
                          src={mediaSrc(offer.imagePath)}
                          alt={offer.imageAlt}
                          width={48}
                          height={48}
                          loading="lazy"
                          className="h-full w-full object-contain p-1.5 mix-blend-multiply"
                        />
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="block truncate text-sm font-medium text-ink group-hover:underline group-hover:decoration-neutral-300 group-hover:underline-offset-4">
                          {offer.title}
                        </span>
                        <span className="mt-0.5 block truncate text-xs text-muted">
                          {[offer.brand, offer.storeShort || offer.merchant].filter(Boolean).join(" · ")}
                        </span>
                      </span>
                      <span className="shrink-0 text-right">
                        <span className="block text-sm font-semibold text-ink tabular-nums">
                          {formatBrl(offer.priceBrlCents)}
                        </span>
                        {pct > 0 && (
                          <span className="mt-0.5 block text-xs text-emerald-700 tabular-nums">
                            <span className="sr-only">Desconto de </span>−{pct}%
                            <span className="ml-1.5 text-muted line-through">{formatBrl(offer.referencePriceCents as number)}</span>
                          </span>
                        )}
                      </span>
                    </Link>
                  </li>
                ))}
              </ul>
              <div className="flex items-center justify-between gap-4 border-t border-line px-6 py-4">
                <p className="text-[0.7rem] leading-snug text-muted">
                  Desconto sobre o preço “De” do anúncio.
                  <br />
                  Frete não incluído.
                </p>
                <MoreLink href="/ofertas/?ordem=price_asc">Comparar</MoreLink>
              </div>
            </aside>
          )}
        </div>
      </section>

      <section aria-labelledby="vitrine-heading" className="container-site pb-24">
        <SectionTitle
          id="vitrine-heading"
          title="Achados do dia"
          sub={`${liveCount} promoções reais do Mercado Livre em teste, observadas em 15/11.`}
          action={<MoreLink href="/ofertas/">Ver catálogo completo</MoreLink>}
        />
        <div className="grid grid-cols-2 gap-x-5 gap-y-12 lg:grid-cols-4 lg:gap-x-8">
          {cards.map((card) => (
            <ShowcaseOfferCard key={card.slug} card={card} />
          ))}
        </div>
      </section>

      <section aria-labelledby="pilares-heading" className="border-y border-line">
        <div className="container-site grid gap-x-12 gap-y-10 py-20 lg:grid-cols-[1fr_2fr]">
          <div>
            <Eyebrow>Como trabalhamos</Eyebrow>
            <h2 id="pilares-heading" className="mt-4 text-3xl leading-tight font-semibold text-ink sm:text-4xl">
              Achados que fazem sentido.
            </h2>
            <div className="mt-5">
              <MoreLink href="/como-selecionamos/">Como selecionamos</MoreLink>
            </div>
          </div>
          <dl className="grid gap-x-10 gap-y-8 sm:grid-cols-2">
            {PRINCIPLES.map(([title, sub], idx) => (
              <div key={title} className="border-t border-line pt-4">
                <dt className="flex gap-3 font-medium text-ink">
                  <span className="font-mono text-xs leading-6 text-muted">{String(idx + 1).padStart(2, "0")}</span>
                  {title}
                </dt>
                <dd className="mt-1.5 pl-7 text-sm leading-relaxed text-muted">{sub}</dd>
              </div>
            ))}
          </dl>
        </div>
      </section>

      <section aria-labelledby="categorias-heading" className="container-site py-24">
        <SectionTitle
          id="categorias-heading"
          title="Categorias"
          sub="Nichos iniciais do piloto, editáveis antes do lançamento."
          action={<MoreLink href="/ofertas/?condicao=com-cupom">Com cupom ativo</MoreLink>}
        />
        <HoverEffect
          items={catalog.categories.map((c) => ({
            title: c.name,
            description: c.description,
            link: `/categorias/${c.slug}/`,
            meta: `${catalog.offers.filter((o) => o.categorySlug === c.slug).length} registros`,
          }))}
        />
      </section>

      <section aria-labelledby="guias-heading" className="container-site pb-24">
        <SectionTitle
          id="guias-heading"
          title="Guias"
          sub="Comparações práticas antes de decidir a compra."
          action={<MoreLink href="/guias/">Ver todos os guias</MoreLink>}
        />
        <div className="grid gap-x-8 gap-y-12 md:grid-cols-3">
          {GUIDES.map((g) => {
            const vis = (GUIDE_VISUALS as Record<string, Visual>)[g.slug] || {};
            const title = vis.homeTitle || g.title;
            return (
              <article key={g.slug} className="group">
                <Link href={`/guias/${g.slug}/`} tabIndex={-1} aria-hidden="true" className="block aspect-[3/2] overflow-hidden rounded-lg bg-tile">
                  <img
                    src={mediaSrc(vis.thumbPath || "/brand/templates/guide-photo-ergonomia.jpg")}
                    alt={vis.thumbAlt || title}
                    width={480}
                    height={320}
                    loading="lazy"
                    className="h-full w-full object-cover transition-transform duration-700 ease-out group-hover:scale-[1.03]"
                  />
                </Link>
                <p className="mt-4 text-xs text-muted">
                  {vis.badgeLabel || "Guia de compra"} · Leitura: {vis.readingMinutes || 6} min
                </p>
                <h3 className="mt-1.5 text-lg leading-snug font-medium text-ink">
                  <Link href={`/guias/${g.slug}/`} className="hover:underline hover:decoration-neutral-300 hover:underline-offset-4">
                    {title}
                  </Link>
                </h3>
              </article>
            );
          })}
        </div>
      </section>

      <section aria-labelledby="comunidade-heading" className="container-site">
        <div className="grid items-end gap-8 border-t border-line pt-12 lg:grid-cols-[1fr_auto]">
          <div className="max-w-xl">
            <h2 id="comunidade-heading" className="text-3xl font-semibold text-ink">
              Acompanhe pelos grupos
            </h2>
            <p className="mt-3 leading-relaxed text-muted">
              Canal opcional para quem prefere alertas resumidos de ofertas e novos guias sem abrir o site todo dia.
            </p>
          </div>
          <div className="flex flex-wrap gap-3">
            <a
              href={SITE_META.telegramUrl}
              target="_blank"
              rel="noopener"
              data-analytics-event="community_link_click"
              data-analytics-detail="telegram-banner"
              className="inline-flex min-h-11 items-center gap-2 rounded-md bg-ink px-5 text-sm font-medium text-white transition-colors hover:bg-petrol-800"
            >
              Receber achados no Telegram <ArrowUpRight className="h-4 w-4" aria-hidden="true" />
            </a>
            <Link
              href="/grupos/"
              className="inline-flex min-h-11 items-center gap-2 rounded-md border border-line px-5 text-sm font-medium text-ink transition-colors hover:border-ink"
            >
              Ver grupos no WhatsApp <ArrowRight className="h-4 w-4" aria-hidden="true" />
            </Link>
          </div>
        </div>
      </section>
    </>
  );
}
