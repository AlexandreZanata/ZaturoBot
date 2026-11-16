import { getCatalog } from "@/lib/catalog";
import { breadcrumbLd, pageMetadata } from "@/lib/seo";
import { JsonLd, Notice, PageHero } from "@/components/site/primitives";
import { OffersExplorer } from "@/components/pages/offers-explorer";

export const metadata = pageMetadata({
  route: "/ofertas/",
  title: "Ofertas e registros demonstrativos",
  description: "Filtre ofertas de demonstração por categoria, faixa de preço e condição com transparência de observação.",
});

export default function OffersPage() {
  const catalog = getCatalog();
  return (
    <>
      <JsonLd data={[breadcrumbLd([{ name: "Início", path: "/" }, { name: "Ofertas", path: "/ofertas/" }])]} />
      <PageHero
        crumbs={[{ label: "Início", href: "/" }, { label: "Ofertas" }]}
        kicker="Catálogo de testes · 4 ofertas reais Mercado Livre + fixtures"
        title="Ofertas e registros observados"
        lead={
          <>
            Filtre por categoria, faixa de preço e condições documentadas. Itens reais do Mercado Livre trazem link oficial{" "}
            <code>meli.la</code> sinalizado; fixtures de demonstração mantêm o botão desativado.
          </>
        }
      />
      <div className="container-site py-10">
        <Notice variant="warning" className="mb-8">
          <p>
            <strong>Composição do catálogo de testes:</strong> 4 ofertas reais de hardware/periféricos do Mercado Livre Afiliados
            (links oficiais <code>meli.la</code>) + 6 fixtures demonstrativos para testar estados reais do sistema (ativos demo,
            desatualizado &gt;30 min, expirado histórico e bloqueado por mídia).
          </p>
        </Notice>
        <OffersExplorer offers={catalog.offers} categories={catalog.categories} mediaRights={catalog.mediaRights} />
      </div>
    </>
  );
}
