import { pageMetadata } from "@/lib/seo";
import { TrustPage } from "@/components/site/trust-page";

export const metadata = pageMetadata({
  route: "/termos/",
  title: "Termos de uso (rascunho)",
  description: "Rascunho dos termos informativos e limites de responsabilidade sobre pedidos.",
});

export default function TermsPage() {
  return (
    <TrustPage
      route="/termos/"
      crumb="Termos de uso"
      ldName="Termos de uso (rascunho)"
      kicker="Rascunho para revisão do proprietário antes do lançamento"
      title="Termos de Uso (rascunho do protótipo)"
    >
      <ul>
        <li>
          <strong>Natureza informativa:</strong> o Procurali publica guias e observações de oferta baseadas em evidências datadas. Não
          garantimos disponibilidade contínua, estoque, entrega grátis universal nem manutenção do preço na loja parceira.
        </li>
        <li>
          <strong>Relação de compra:</strong> pagamentos, prazos de entrega, trocas, devoluções e garantias seguem os termos e a
          responsabilidade do vendedor e da plataforma onde você concluir o pedido.
        </li>
        <li>
          <strong>Comunidades externas:</strong> a adesão ao Telegram ou a futuros grupos de WhatsApp é voluntária e regida pelas regras
          do respectivo aplicativo.
        </li>
      </ul>
    </TrustPage>
  );
}
