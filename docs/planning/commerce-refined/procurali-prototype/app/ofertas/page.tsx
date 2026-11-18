import { getCatalog } from "@/lib/catalog";
import { breadcrumbLd, pageMetadata } from "@/lib/seo";
import { JsonLd, PageHero } from "@/components/site/primitives";
import { OffersExplorer } from "@/components/pages/offers-explorer";

export const metadata = pageMetadata({
  route: "/ofertas/",
  title: "Ofertas do Mercado Livre com preço conferido",
  description: "Compare ofertas do Mercado Livre por categoria, faixa de preço e condição, com horário de observação e link de afiliado sinalizado.",
});

export default function OffersPage() {
  const catalog = getCatalog();
  const liveCount = catalog.offers.filter((o) => !o.isDemoFixture && o.state === "approved").length;
  return (
    <>
      <JsonLd data={[breadcrumbLd([{ name: "Início", path: "/" }, { name: "Ofertas", path: "/ofertas/" }])]} />
      <PageHero
        crumbs={[{ label: "Início", href: "/" }, { label: "Ofertas" }]}
        kicker={`${liveCount} ${liveCount === 1 ? "oferta ativa" : "ofertas ativas"} · Mercado Livre`}
        title="Ofertas com preço conferido"
        lead="Filtre por categoria, faixa de preço e condição. Cada oferta mostra quando o preço foi observado, e o link de afiliado vem sempre sinalizado."
      />
      <div className="container-site py-10">
        <OffersExplorer offers={catalog.offers} categories={catalog.categories} mediaRights={catalog.mediaRights} />
      </div>
    </>
  );
}
