import { getCatalog } from "@/lib/catalog";
import { breadcrumbLd, pageMetadata } from "@/lib/seo";
import { JsonLd, PageHero } from "@/components/site/primitives";
import { HoverEffect } from "@/components/ui/card-hover-effect";

export const metadata = pageMetadata({
  route: "/categorias/",
  title: "Categorias iniciais (editáveis)",
  description: "Explore categorias iniciais de organização, ergonomia, energia e iluminação de mesa.",
});

export default function CategoriesPage() {
  const catalog = getCatalog();
  return (
    <>
      <JsonLd data={[breadcrumbLd([{ name: "Início", path: "/" }, { name: "Categorias", path: "/categorias/" }])]} />
      <PageHero
        crumbs={[{ label: "Início", href: "/" }, { label: "Categorias" }]}
        kicker="Estrutura editável pelo proprietário"
        title="Categorias do piloto"
        lead="Estas categorias representam a hipótese inicial de nicho (casa, mesa e tecnologia prática entre R$ 30 e R$ 300) e podem ser editadas antes do lançamento."
      />
      <div className="container-site py-10">
        <HoverEffect
          items={catalog.categories.map((cat) => ({
            title: cat.name,
            description: cat.description,
            link: `/categorias/${cat.slug}/`,
            meta: `${catalog.offers.filter((o) => o.categorySlug === cat.slug).length} registro(s) demonstrativo(s)`,
          }))}
        />
      </div>
    </>
  );
}
