import { AI_BUDGET_POLICY, EDITORIAL_APPROVALS } from "@/data/demo-catalog.mjs";
import { getCatalog } from "@/lib/catalog";
import { formatBrl } from "@/lib/contracts.mjs";
import { pageMetadata } from "@/lib/seo";
import { Card, Notice, PageHero, SectionTitle } from "@/components/site/primitives";

export const metadata = pageMetadata({
  route: "/cms-preview/",
  title: "Mockup de fila editorial e contratos (não produtivo)",
  description: "Prévia visual não produtiva das aprovações, teto de IA e Search Console.",
  noindex: true,
});

export default function CmsPreviewPage() {
  const live = getCatalog().offers.filter((o) => !o.isDemoFixture);
  return (
    <>
      <PageHero
        crumbs={[{ label: "Início", href: "/" }, { label: "Prévia de contrato CMS (não produtivo)" }]}
        kicker="Área não produtiva · noindex"
        title="Fila de revisão e contratos de integração"
        lead="Visão ilustrativa do fluxo: fonte permitida → checagem de frescor (30 min) e direitos → aprovação exata por hash → publicação manual ou futura integração autenticada."
      />
      <div className="container-site space-y-12 py-10">
        <Notice variant="danger">
          <p>
            <strong>ÁREA NÃO PRODUTIVA — MOCKUP VISUAL DE CONTRATO EDITORIAL (NOINDEX):</strong> esta página estática apenas ilustra a fila
            editorial e os contratos de dados (<code>Offer</code>, <code>MediaRights</code>, <code>EditorialApproval</code> e teto de IA).
            Não existe autenticação aqui e nenhum endpoint de publicação foi exposto.
          </p>
        </Notice>

        <section>
          <SectionTitle
            title="Lote real de testes — Mercado Livre Afiliados (metadados do operador)"
            sub={
              <>
                Taxas de comissão estimadas e códigos internos ficam restritos a esta prévia de contrato editorial; links públicos usam a URL
                exata <code>meli.la</code> com <code>rel=&quot;sponsored noopener&quot;</code>.
              </>
            }
          />
          <div className="overflow-x-auto rounded-lg border border-line bg-white">
            <table className="w-full min-w-[860px] text-left text-sm">
              <thead className="border-b border-line text-xs tracking-wide text-muted uppercase">
                <tr>
                  {["ID ML", "Produto", "Vendedor", "Preço", "Comissão hub", "Share ID", "Link oficial"].map((h) => (
                    <th key={h} scope="col" className="px-4 py-3 font-semibold">
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody className="divide-y divide-line">
                {live.map((o) => (
                  <tr key={o.id}>
                    <td className="px-4 py-3 font-mono text-xs">{o.productId || o.id}</td>
                    <td className="px-4 py-3">{o.title}</td>
                    <td className="px-4 py-3">{o.sellerName || o.merchant}</td>
                    <td className="px-4 py-3 font-semibold">{formatBrl(o.priceBrlCents)}</td>
                    <td className="px-4 py-3 font-mono text-xs">{o.commissionRate || "-"}</td>
                    <td className="px-4 py-3 font-mono text-xs">{o.shareId || "-"}</td>
                    <td className="px-4 py-3 font-mono text-xs">{o.officialAffiliateUrl || "-"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        <section>
          <SectionTitle title="Registros de aprovação (EditorialApproval)" />
          <div className="overflow-x-auto rounded-lg border border-line bg-white">
            <table className="w-full min-w-[720px] text-left text-sm">
              <thead className="border-b border-line text-xs tracking-wide text-muted uppercase">
                <tr>
                  <th className="px-4 py-3">Revisão</th>
                  <th className="px-4 py-3">Alvo</th>
                  <th className="px-4 py-3">Status</th>
                  <th className="px-4 py-3">Bloqueadores atuais</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-line">
                {EDITORIAL_APPROVALS.map((ap) => (
                  <tr key={ap.revision} className="align-top">
                    <td className="px-4 py-3 font-mono text-xs">{ap.revision}</td>
                    <td className="px-4 py-3 font-mono text-xs">
                      {ap.targetType} · {ap.targetId}
                    </td>
                    <td className="px-4 py-3 font-semibold">{ap.status}</td>
                    <td className="px-4 py-3">
                      {ap.blockers.map((b: string) => (
                        <div key={b}>• {b}</div>
                      ))}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        <section className="grid gap-6 md:grid-cols-2">
          <Card>
            <h2 className="text-xl font-semibold text-ink">Teto mensal de IA planejado</h2>
            <ul className="mt-3 list-disc space-y-1.5 pl-5 text-sm text-neutral-700 marker:text-neutral-300">
              <li>
                <strong>Teto combinado:</strong> US$ {AI_BUDGET_POLICY.monthlyCeilingUsd}/mês
              </li>
              <li>
                <strong>Extra Usage:</strong> {AI_BUDGET_POLICY.extraUsageDisabled ? "Desligado (sem cobrança extra automática)" : "Ativo"}
              </li>
              <li>
                <strong>Limite diário de chamadas:</strong> Sem teto diário fixo; sujeito ao teto monetário mensal
              </li>
              <li>
                <strong>Rotas autorizadas:</strong> {AI_BUDGET_POLICY.allowedRoutes.join(" | ")}
              </li>
              <li>
                <strong>Reconciliação:</strong> Pausa novas chamadas se a telemetria de uso estiver desconhecida
              </li>
            </ul>
          </Card>
          <Card>
            <h2 className="text-xl font-semibold text-ink">Configuração do Google Search Console (nota ao proprietário)</h2>
            <ul className="mt-3 list-disc space-y-1.5 pl-5 text-sm text-neutral-700 marker:text-neutral-300">
              <li>Nenhuma propriedade conectada nem token de verificação foi inserido neste protótipo.</li>
              <li>
                As páginas públicas já são indexáveis; <code>/cms-preview/</code>, <code>/criativos/</code>, fixtures de demonstração e
                ofertas indisponíveis ficam com <code>noindex</code> e fora do <code>sitemap.xml</code>.
              </li>
              <li>
                Verifique a propriedade do domínio no Search Console após configurar DNS/TLS e envie <code>sitemap.xml</code>.
              </li>
            </ul>
          </Card>
        </section>
      </div>
    </>
  );
}
