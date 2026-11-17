import Link from "next/link";
import { notFound } from "next/navigation";
import { ExternalLink } from "lucide-react";
import { GUIDES, SITE_META } from "@/data/demo-catalog.mjs";
import { getCatalog } from "@/lib/catalog";
import { breadcrumbLd, pageMetadata } from "@/lib/seo";
import { JsonLd, PageHero } from "@/components/site/primitives";
import { MiniOfferCard } from "@/components/site/offer";
import { TracingBeam } from "@/components/ui/tracing-beam";

type Params = { params: Promise<{ slug: string }> };
export const dynamicParams = false;

export function generateStaticParams() {
  return GUIDES.map((g) => ({ slug: g.slug }));
}

export async function generateMetadata({ params }: Params) {
  const { slug } = await params;
  const guide = GUIDES.find((g) => g.slug === slug);
  if (!guide) return {};
  return pageMetadata({ route: `/guias/${guide.slug}/`, title: guide.title, description: guide.metaDescription });
}

export default async function GuidePage({ params }: Params) {
  const { slug } = await params;
  const guide = GUIDES.find((g) => g.slug === slug);
  if (!guide) notFound();
  const related = getCatalog().offers.filter((o) => guide.relatedOfferSlugs.includes(o.slug));
  const route = `/guias/${guide.slug}/`;

  return (
    <>
      <JsonLd
        data={[
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
        ]}
      />
      <article>
        <PageHero
          crumbs={[{ label: "Início", href: "/" }, { label: "Guias", href: "/guias/" }, { label: guide.title }]}
          kicker={guide.methodologyLabel}
          title={guide.title}
          lead={guide.summary}
        >
          <div className="mt-8 flex flex-wrap gap-x-6 gap-y-1 text-sm text-muted">
            <span className="inline-flex items-center gap-1.5">
              <strong>Autor:</strong> {guide.author}
            </span>
            <span className="inline-flex items-center gap-1.5">
              <strong>Revisor:</strong> {guide.reviewer}
            </span>
            <span className="inline-flex items-center gap-1.5">
              <strong>Publicado:</strong> {guide.publishedAt} ·{" "}
              <strong>Atualizado:</strong> {guide.updatedAt}
            </span>
          </div>
        </PageHero>

        <div className="container-site grid gap-10 py-12 lg:grid-cols-[260px_1fr]">
          <aside aria-label="Índice do guia" className="h-fit lg:sticky lg:top-24">
            <h2 className="text-xs font-medium tracking-[0.08em] text-muted uppercase">Neste guia</h2>
            <ol className="mt-3 space-y-1">
              {guide.tableOfContents.map((item, idx) => (
                <li key={item.id}>
                  <a
                    href={`#${item.id}`}
                    className="flex gap-3 py-1.5 text-sm text-muted hover:text-ink"
                  >
                    <span className="font-mono text-xs leading-5 text-neutral-400">{String(idx + 1).padStart(2, "0")}</span>
                    {item.label}
                  </a>
                </li>
              ))}
            </ol>
            <p className="mt-4 border-t border-line pt-4 text-xs text-muted">{SITE_META.affiliateDisclosure}</p>
          </aside>

          <TracingBeam className="max-w-3xl px-2 md:pl-10">
            <div className="prose-site">
              {guide.sections.map((sec) => (
                <section key={sec.id} id={sec.id} className="scroll-mt-28">
                  <h2>{sec.heading}</h2>
                  {sec.paragraphs.map((p, idx) => (
                    <p key={idx}>{p}</p>
                  ))}
                </section>
              ))}

              <section id="comparacao" className="scroll-mt-28">
                <h2>Comparação de cenários e contrapontos</h2>
                <div className="not-prose overflow-x-auto">
                  <table className="w-full min-w-[520px] text-left text-sm">
                    <thead className="border-b border-line text-xs tracking-wide text-muted uppercase">
                      <tr>
                        <th scope="col" className="px-4 py-3">Critério</th>
                        <th scope="col" className="px-4 py-3">Opção A</th>
                        <th scope="col" className="px-4 py-3">Opção B</th>
                      </tr>
                    </thead>
                    <tbody className="divide-y divide-line">
                      {guide.comparisonRows.map((r) => (
                        <tr key={r.criterio} className="align-top">
                          <th scope="row" className="px-4 py-3 font-medium text-ink">
                            {r.criterio}
                          </th>
                          <td className="px-4 py-3 text-neutral-700">{r.opcaoA}</td>
                          <td className="px-4 py-3 text-neutral-700">{r.opcaoB}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </section>

              <section id="fontes" className="scroll-mt-28">
                <h2>Fontes, metodologia e histórico</h2>
                <p>
                  <strong>Metodologia:</strong> {guide.methodologyLabel}.
                </p>
                <ul>
                  {guide.sourceLinks.map((s) => (
                    <li key={s.url}>
                      <a href={s.url} target="_blank" rel="noopener">
                        {s.label} <ExternalLink className="inline h-3.5 w-3.5" aria-hidden="true" />
                      </a>{" "}
                      (consultado em {s.consultedAt})
                    </li>
                  ))}
                </ul>
                <details className="border-t border-line pt-4 text-sm">
                  <summary className="cursor-pointer font-semibold text-ink">Histórico de atualizações materiais</summary>
                  <ul className="mt-3 mb-0">
                    {guide.updateHistory.map((h) => (
                      <li key={h}>{h}</li>
                    ))}
                  </ul>
                </details>
              </section>

              {related.length > 0 && (
                <section>
                  <h2>Registros demonstrativos relacionados</h2>
                  <div className="not-prose grid gap-x-8 sm:grid-cols-2">
                    {related.map((o) => (
                      <MiniOfferCard key={o.slug} offer={o} />
                    ))}
                  </div>
                </section>
              )}
              <p className="mt-10">
                <Link href="/guias/">← Todos os guias</Link>
              </p>
            </div>
          </TracingBeam>
        </div>
      </article>
    </>
  );
}
