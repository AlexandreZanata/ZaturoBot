import { breadcrumbLd, pageMetadata } from "@/lib/seo";
import { JsonLd, PageHero } from "@/components/site/primitives";
import { Timeline } from "@/components/ui/timeline";

export const metadata = pageMetadata({
  route: "/como-selecionamos/",
  title: "Como selecionamos",
  description: "Política editorial, janela de frescor de 30 minutos e regra de amostra de 7 dias.",
});

const STEPS = [
  {
    title: "Uso real",
    heading: "Aderência a uso real",
    body: "Priorizamos problemas concretos (postura na mesa, falta de portas USB-C, proteção elétrica) em vez de volume diário de links.",
  },
  {
    title: "Evidência",
    heading: "Evidência e restrição de testes",
    body: "Quando comparamos apenas fichas técnicas e documentação pública, rotulamos explicitamente como pesquisa de bancada. Nunca simulamos testes de bancada física não realizados.",
  },
  {
    title: "30 minutos",
    heading: "Frescor de preço (janela de 30 minutos)",
    body: "Em produção, preços anunciados exigem verificação próxima da publicação; registros vencidos passam ao estado desatualizado e bloqueiam o botão de compra.",
  },
  {
    title: "7 dias",
    heading: "Amostra mínima de 7 dias para comparação semanal",
    body: "Só exibimos “Menor preço observado pelo Procurali nesta semana” quando há série documentada de 7 dias na mesma variante, vendedor e condição de pagamento.",
  },
  {
    title: "Revisão",
    heading: "Direitos de mídia e revisão humana",
    body: "Ausência de licença de imagem ou alteração de texto invalida a publicação até nova aprovação do proprietário.",
  },
];

export default function SelectionPage() {
  return (
    <>
      <JsonLd data={[breadcrumbLd([{ name: "Início", path: "/" }, { name: "Como selecionamos", path: "/como-selecionamos/" }])]} />
      <PageHero
        crumbs={[{ label: "Início", href: "/" }, { label: "Como selecionamos" }]}
        kicker="Critérios editoriais"
        title="Como o Procurali seleciona ofertas e guias"
        lead="Cinco regras que valem para toda oferta, guia e postagem — da coleta à publicação."
      />
      <div className="container-site">
        <Timeline
          data={STEPS.map((step, idx) => ({
            title: step.title,
            content: (
              <div>
                <p className="font-mono text-xs text-muted">{String(idx + 1).padStart(2, "0")}</p>
                <h2 className="mt-2 text-2xl font-semibold text-ink">{step.heading}</h2>
                <p className="mt-3 max-w-xl leading-relaxed text-muted">{step.body}</p>
              </div>
            ),
          }))}
        />
      </div>
    </>
  );
}
