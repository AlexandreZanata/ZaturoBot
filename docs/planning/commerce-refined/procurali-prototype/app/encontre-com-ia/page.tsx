import { AI_BUDGET_POLICY } from "@/data/demo-catalog.mjs";
import { getCatalog } from "@/lib/catalog";
import { breadcrumbLd, pageMetadata } from "@/lib/seo";
import { JsonLd, Notice, PageHero } from "@/components/site/primitives";
import { AiFinder } from "@/components/pages/ai-finder";

export const metadata = pageMetadata({
  route: "/encontre-com-ia/",
  title: "Encontre com IA (Demonstração local)",
  description: "Seletor guiado por necessidade e orçamento em demonstração local determinística, sem consulta ao vivo.",
});

export default function AiFinderPage() {
  return (
    <>
      <JsonLd data={[breadcrumbLd([{ name: "Início", path: "/" }, { name: "Encontre com IA", path: "/encontre-com-ia/" }])]} />
      <PageHero
        crumbs={[{ label: "Início", href: "/" }, { label: "Encontre com IA" }]}
        kicker="Seletor guiado · Protótipo determinístico"
        title="Encontre opções para sua necessidade"
      >
        <div className="mt-6 max-w-2xl border-l-2 border-neutral-300 pl-4 text-sm leading-relaxed text-muted">
          <p className="font-medium text-ink">Demonstração — sem consulta ao vivo.</p>
          <p className="mt-1">
            As respostas abaixo são calculadas localmente sobre o catálogo fictício do protótipo. Nenhuma chave de API ou chamada de
            modelo é executada no navegador, e seus textos não são salvos.
          </p>
        </div>
      </PageHero>
      <div className="container-site space-y-8 py-10">
        <AiFinder offers={getCatalog().offers} />
        <details className="border-t border-line pt-5 text-sm">
          <summary className="cursor-pointer font-semibold text-ink">Arquitetura planejada para produção (ainda não conectada)</summary>
          <ul className="mt-3 list-disc space-y-1.5 pl-5 text-neutral-700 marker:text-neutral-300">
            <li>
              <strong>Adaptador no servidor:</strong> chaves protegidas fora do navegador; validação e sanitização de entrada curta.
            </li>
            <li>
              <strong>Catálogo fechado:</strong> recomendações baseadas apenas em ofertas e guias aprovados com timestamp.
            </li>
            <li>
              <strong>Teto agregado:</strong> US$ 20/mês combinados entre {AI_BUDGET_POLICY.allowedRoutes.join(" e ")}, Extra Usage
              desligado e pausa imediata se a telemetria falhar.
            </li>
          </ul>
        </details>
        <Notice>
          <p>O seletor não substitui a leitura da ficha: confira sempre condições, frete e compatibilidade na página da oferta.</p>
        </Notice>
      </div>
    </>
  );
}
