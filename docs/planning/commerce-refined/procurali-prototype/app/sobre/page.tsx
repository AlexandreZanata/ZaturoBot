import { SITE_META } from "@/data/demo-catalog.mjs";
import { organizationLd, pageMetadata } from "@/lib/seo";
import { JsonLd } from "@/components/site/primitives";
import { TrustPage } from "@/components/site/trust-page";

export const metadata = pageMetadata({
  route: "/sobre/",
  title: "Sobre o Procurali",
  description: "Conheça o papel editorial do Procurali e sua separação das lojas parceiras.",
});

export default function AboutPage() {
  return (
    <>
      <JsonLd data={[organizationLd]} />
      <TrustPage
        route="/sobre/"
        crumb="Sobre o Procurali"
        ldName="Sobre o Procurali"
        kicker="Identidade e papel do serviço"
        title="Sobre o Procurali (Procura Ali)"
        lead="O Procurali é um guia editorial de compras e curadoria de ofertas criado para ajudar compradores no Brasil a comparar especificações, condições e limitações antes de decidir."
      >
        <h2>O que somos — e o que não somos</h2>
        <ul>
          <li>
            <strong>Não somos loja nem marketplace:</strong> não mantemos estoque, não processamos pagamentos, não emitimos nota fiscal
            de venda e não fornecemos garantia de fabricante.
          </li>
          <li>
            <strong>A compra acontece na loja oficial:</strong> todo pedido, frete, troca e atendimento pós-venda é responsabilidade
            exclusiva do vendedor e da plataforma identificada.
          </li>
          <li>
            <strong>Independência visual:</strong> embora utilize links de programas de afiliados quando aprovados, o Procurali (Procura
            Ali) é um projeto independente sem vínculo societário com Shopee, AliExpress ou outros varejistas.
          </li>
        </ul>
        <p className="text-muted">
          <strong>Nota de protótipo:</strong> os dados cadastrais finais, e-mail de contato dedicado e aprovação do slogan (“
          {SITE_META.proposedTagline}”) serão inseridos pelo proprietário antes do lançamento público.
        </p>
      </TrustPage>
    </>
  );
}
