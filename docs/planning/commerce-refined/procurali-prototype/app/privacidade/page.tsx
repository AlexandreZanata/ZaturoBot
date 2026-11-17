import { pageMetadata } from "@/lib/seo";
import { TrustPage } from "@/components/site/trust-page";
import { AnalyticsConsent } from "@/components/pages/consent";

export const metadata = pageMetadata({
  route: "/privacidade/",
  title: "Privacidade (rascunho)",
  description: "Rascunho de política de privacidade e controle opcional de medição local.",
});

export default function PrivacyPage() {
  return (
    <TrustPage
      route="/privacidade/"
      crumb="Privacidade"
      ldName="Privacidade (rascunho)"
      kicker="Rascunho de política para revisão do proprietário antes da publicação"
      title="Política de Privacidade (rascunho do protótipo)"
      lead={
        <>
          Este texto descreve o comportamento técnico planejado e implementado neste protótipo. Deve ser revisado e completado com os
          dados finais do controlador antes do lançamento em procurali.com.br.
        </>
      }
    >
      <h2>1. Sem cadastro obrigatório e sem pixels ocultos</h2>
      <p>
        A navegação pelas ofertas, guias e canais é pública e não exige criação de conta. Este protótipo não carrega pixels de
        publicidade, rastreadores de terceiros nem scripts externos ocultos.
      </p>
      <h2>2. Itens salvos e preferências no navegador</h2>
      <p>
        Ao clicar em “Salvar item” ou ativar a medição demonstrativa abaixo, os dados ficam exclusivamente no <code>localStorage</code>{" "}
        do seu próprio navegador.
      </p>
      <h2>3. Consultas ao Encontre com IA</h2>
      <p>
        No protótipo, o seletor funciona localmente sem enviar consultas pela rede. Em produção, a política planejada é processar
        consultas de forma efêmera, sem reter histórico de conversa nem solicitar CPF, senha, endereço completo ou dados bancários.
      </p>
      <AnalyticsConsent />
    </TrustPage>
  );
}
