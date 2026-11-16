import Link from "next/link";
import { GUIDE_VISUALS, GUIDES } from "@/data/demo-catalog.mjs";
import { breadcrumbLd, pageMetadata } from "@/lib/seo";
import { JsonLd, Notice, PageHero } from "@/components/site/primitives";
import { mediaSrc } from "@/components/site/offer";

export const metadata = pageMetadata({
  route: "/guias/",
  title: "Guias de compra e critérios",
  description: "Guias originais baseados em pesquisa de bancada para evitar erros de compatibilidade.",
});

type Visual = { thumbPath?: string; thumbAlt?: string; badgeLabel?: string; readingMinutes?: number };

export default function GuidesPage() {
  return (
    <>
      <JsonLd data={[breadcrumbLd([{ name: "Início", path: "/" }, { name: "Guias", path: "/guias/" }])]} />
      <PageHero
        crumbs={[{ label: "Início", href: "/" }, { label: "Guias" }]}
        kicker="Conteúdo editorial · Pesquisa de bancada"
        title="Guias para decidir sem pressa"
        lead="Critérios técnicos, comparações por cenário e erros comuns explicados em linguagem direta, sem inventar testes físicos."
      />
      <div className="container-site py-10">
        <div className="grid gap-x-8 gap-y-14 md:grid-cols-2 lg:grid-cols-3">
          {GUIDES.map((g) => {
            const vis = ((GUIDE_VISUALS as Record<string, Visual>)[g.slug] || {}) as Visual;
            return (
              <article key={g.slug} className="group flex flex-col">
                <Link href={`/guias/${g.slug}/`} tabIndex={-1} aria-hidden="true" className="block aspect-[3/2] overflow-hidden rounded-lg bg-tile">
                  <img
                    src={mediaSrc(vis.thumbPath || "/brand/templates/guide-thumb-ergonomia.svg")}
                    alt={vis.thumbAlt || g.title}
                    width={480}
                    height={300}
                    loading="lazy"
                    className="h-full w-full object-cover transition-transform duration-500 group-hover:scale-105"
                  />
                </Link>
                <div className="flex flex-1 flex-col pt-4">
                  <span className="text-xs text-muted">
                    {vis.badgeLabel || "Guia de compra"} · Pesquisa de bancada
                  </span>
                  <h2 className="mt-1.5 text-xl leading-snug font-medium text-ink">
                    <Link href={`/guias/${g.slug}/`} className="hover:underline">
                      {g.title}
                    </Link>
                  </h2>
                  <p className="mt-2 text-sm text-neutral-600">{g.summary}</p>
                  <p className="mt-4 flex flex-wrap gap-x-3 gap-y-1 text-xs text-muted">
                    <span className="inline-flex items-center gap-1">
                      Publicado: {g.publishedAt} · Atualizado: {g.updatedAt}
                    </span>
                    <span className="inline-flex items-center gap-1">
                      {vis.readingMinutes || 6} min
                    </span>
                  </p>
                  <Link href={`/guias/${g.slug}/`} className="mt-4 inline-flex min-h-11 items-center text-sm font-medium text-ink underline decoration-neutral-300 underline-offset-4 hover:decoration-ink">
                    Ler guia
                  </Link>
                </div>
              </article>
            );
          })}
        </div>
        <Notice className="mt-16">
          <p>
            <strong>Seleção semanal separada:</strong> veja também o recorte por data em{" "}
            <Link href="/selecoes-da-semana/" className="font-semibold underline">
              Seleções da semana
            </Link>
            .
          </p>
        </Notice>
      </div>
    </>
  );
}
