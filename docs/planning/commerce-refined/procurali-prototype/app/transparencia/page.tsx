import { SITE_META } from "@/data/demo-catalog.mjs";
import { pageMetadata } from "@/lib/seo";
import { TrustPage } from "@/components/site/trust-page";

export const metadata = pageMetadata({
  route: "/transparencia/",
  title: "Transparência de afiliados",
  description: "Como o Procurali financia o projeto, uso de #publi e links rel=sponsored.",
});

export default function TransparencyPage() {
  return (
    <TrustPage
      route="/transparencia/"
      crumb="Transparência de afiliados"
      ldName="Transparência de afiliados"
      kicker="Modelo comercial e publicidade"
      title="Transparência de afiliados e divulgação (#publi)"
      lead="O acesso aos guias, seleções e canais do Procurali é gratuito para o leitor. A operação planejada é financiada por comissões de programas de afiliados quando um leitor decide comprar na loja oficial através de um link aprovado."
    >
      <ul>
        <li>
          <strong>Sem custo adicional:</strong> o link de afiliado não acrescenta taxa no seu pedido.
        </li>
        <li>
          <strong>Identificação próxima ao clique:</strong> exibimos “{SITE_META.affiliateDisclosure}” junto ao botão de saída e
          utilizamos <code>#publi</code> ou aviso equivalente em postagens sociais e vídeos.
        </li>
        <li>
          <strong>Qualificação técnica dos links:</strong> âncoras de afiliado usam <code>rel=&quot;sponsored noopener&quot;</code> e
          preservam integralmente a URL oficial do programa, sem encurtadores opacos nem parâmetros inventados.
        </li>
        <li>
          <strong>Estado atual do piloto:</strong> a inscrição no programa de afiliados da Shopee com o canal no Telegram segue sob
          análise; as ofertas reais do <strong>Mercado Livre Afiliados</strong> publicadas aqui utilizam links
          oficiais <code>meli.la</code> para validação de fluxo, enquanto os demais itens fictícios de demonstração mantêm o botão
          desativado.
        </li>
      </ul>
    </TrustPage>
  );
}
