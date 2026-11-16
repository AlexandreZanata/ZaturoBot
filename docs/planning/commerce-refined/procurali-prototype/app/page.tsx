import Link from "next/link";
import { ArrowRight, ArrowUpRight } from "lucide-react";
import { GUIDE_VISUALS, GUIDES, SITE_META } from "@/data/demo-catalog.mjs";
import { getCatalog, getShowcaseCards } from "@/lib/catalog";
import { organizationLd, pageMetadata } from "@/lib/seo";
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

  return (
    <>
      <JsonLd data={[organizationLd]} />

      <section className="container-site pt-10 pb-12 sm:pt-14 sm:pb-16">
        <Eyebrow>Guia independente de compras</Eyebrow>
        <h1 className="mt-4 max-w-3xl text-[2rem] leading-[1.08] font-semibold tracking-[-0.03em] text-balance text-ink sm:text-[2.75rem] lg:text-5xl">
          Encontre o que vale a pena para você.{" "}
          <span className="text-neutral-400">Ofertas e guias para escolher com mais contexto.</span>
        </h1>
        <div className="mt-7 max-w-xl">
          <PlaceholdersAndVanishInput
            action="/ofertas/"
            inputId="opening-search-input"
            label="Busque por produto, categoria ou oferta"
            placeholders={["SSD NVMe PCIe 4.0", "Teclado mecânico ABNT2", "Monitor 27 polegadas 200 Hz", "Headset com microfone removível"]}
          />
        </div>
        <div className="mt-4 flex flex-wrap items-center gap-x-8 gap-y-1 text-sm">
          <MoreLink href="/ofertas/">Explorar ofertas</MoreLink>
          <a
            href={SITE_META.telegramUrl}
            target="_blank"
            rel="noopener"
            data-analytics-event="community_link_click"
            data-analytics-detail="telegram-hero"
            className="group inline-flex min-h-11 items-center gap-1.5 font-medium text-ink"
          >
            <span className="underline decoration-neutral-300 underline-offset-4 group-hover:decoration-ink">
              Receber achados no Telegram
            </span>
            <ArrowUpRight className="h-4 w-4" aria-hidden="true" />
          </a>
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
