import Link from "next/link";
import { notFound } from "next/navigation";
import { GUIDES } from "@/data/demo-catalog.mjs";
import { getCatalog } from "@/lib/catalog";
import { breadcrumbLd, pageMetadata, isIndexableOffer } from "@/lib/seo";
import { JsonLd, PageHero, SectionTitle } from "@/components/site/primitives";
import { OfferRow } from "@/components/site/offer";

type Params = { params: Promise<{ slug: string }> };
export const dynamicParams = false;

export function generateStaticParams() {
  return getCatalog().categories.map((c) => ({ slug: c.slug }));
}

export async function generateMetadata({ params }: Params) {
  const { slug } = await params;
  const catalog = getCatalog();
  const cat = catalog.categories.find((c) => c.slug === slug);
  if (!cat) return {};
  // Categories holding only demo fixtures stay out of search results (and of sitemap.xml).
  const hasLiveOffer = catalog.offers.some((o) => o.categorySlug === cat.slug && isIndexableOffer(o));
  return pageMetadata({
    route: `/categorias/${cat.slug}/`,
    title: `${cat.name} — Categoria`,
    description: cat.description,
    noindex: !hasLiveOffer,
  });
}

export default async function CategoryPage({ params }: Params) {
  const { slug } = await params;
  const catalog = getCatalog();
  const category = catalog.categories.find((c) => c.slug === slug);
  if (!category) notFound();
  const offers = catalog.offers.filter((o) => o.categorySlug === category.slug);
  const guides = GUIDES.filter((g) => g.categorySlug === category.slug);
  const route = `/categorias/${category.slug}/`;
  return (
    <>
      <JsonLd
        data={[
          breadcrumbLd([
            { name: "Início", path: "/" },
            { name: "Categorias", path: "/categorias/" },
            { name: category.name, path: route },
          ]),
        ]}
      />
      <PageHero
        crumbs={[{ label: "Início", href: "/" }, { label: "Categorias", href: "/categorias/" }, { label: category.name }]}
        kicker="Categoria · Nicho inicial editável"
        title={category.name}
        lead={category.description}
      >
        <p className="mt-4 text-sm text-muted">
          Faixa indicativa inicial: {category.intendedBudgetHint}
        </p>
      </PageHero>
      <div className="container-site space-y-14 py-10">
        <section>
          <SectionTitle title={`Registros demonstrativos nesta categoria (${offers.length})`} />
          <div className="border-b border-line">
            {offers.map((offer) => (
              <OfferRow key={offer.slug} offer={offer} mediaRights={catalog.mediaRights} />
            ))}
          </div>
        </section>
        {guides.length > 0 && (
          <section>
            <SectionTitle title="Guias relacionados" />
            <div className="grid gap-4 md:grid-cols-2">
              {guides.map((g) => (
                <article key={g.slug} className="border-t border-line pt-5">
                  <h3 className="text-lg font-semibold text-ink">
                    <Link href={`/guias/${g.slug}/`} className="hover:underline">
                      {g.title}
                    </Link>
                  </h3>
                  <p className="mt-2 text-sm text-neutral-600">{g.summary}</p>
                </article>
              ))}
            </div>
          </section>
        )}
      </div>
    </>
  );
}
