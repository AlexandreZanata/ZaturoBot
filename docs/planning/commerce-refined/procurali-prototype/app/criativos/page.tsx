import { CREATIVE_ASSETS, SHORT_VIDEO_STORYBOARD, TELEGRAM_POST_TEMPLATE, WHATSAPP_MANUAL_DIGEST_TEMPLATE } from "@/data/demo-catalog.mjs";
import { pageMetadata } from "@/lib/seo";
import { Badge, PageHero, SectionTitle } from "@/components/site/primitives";
import { CopyButton } from "@/components/site/copy-button";
import { cn } from "@/lib/utils";

export const metadata = pageMetadata({
  route: "/criativos/",
  title: "Kit criativo e identidade visual",
  description: "Logotipos, templates SVG determinísticos, roteiro vertical e rascunhos Telegram/WhatsApp.",
  noindex: true,
});

const LOGOS = [
  { src: "/brand/logo-primary-light.svg", alt: "Logotipo principal Procurali em fundo claro", note: "Versão principal clara (#FFFFFF / #143D2B / CTA #C2410C)", dark: false, w: 360, h: 88 },
  { src: "/brand/logo-primary-dark.svg", alt: "Logotipo principal Procurali em fundo verde escuro", note: "Versão principal escura (#143D2B / #FFFFFF / destaque #A3E635)", dark: true, w: 360, h: 88 },
  { src: "/brand/procurali_icone_branco_transparente.svg", alt: "Ícone oficial Procurali branco transparente (favicon do navegador)", note: "Ícone oficial do navegador (brand/procurali_icone_branco_transparente.svg)", dark: true, w: 96, h: 96 },
  { src: "/brand/symbol-mark.svg", alt: "Símbolo oficial Procurali verde escuro", note: "Símbolo oficial sobre fundo claro (#143D2B)", dark: false, w: 96, h: 96 },
  { src: "/brand/logo-monochrome-dark.svg", alt: "Logotipo monocromático escuro Procurali", note: "Monocromático escuro para carimbos e impressão", dark: false, w: 360, h: 88 },
  { src: "/brand/logo-monochrome-light.svg", alt: "Logotipo monocromático claro Procurali", note: "Monocromático claro sobre superfície escura", dark: true, w: 360, h: 88 },
];

export default function CreativeKitPage() {
  return (
    <>
      <PageHero
        crumbs={[{ label: "Início", href: "/" }, { label: "Kit criativo e identidade" }]}
        kicker="Identidade visual e templates reutilizáveis"
        title="Sistema visual Procurali e peças de demonstração"
        lead="Logotipos em SVG, paleta acessível, cards determinísticos, carrossel de 5 slides, roteiro de vídeo vertical e pacotes manuais para Telegram e WhatsApp."
      />
      <div className="container-site space-y-16 py-12">
        <section>
          <SectionTitle title="Variantes de marca e arquivos vetoriais" />
          <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
            {LOGOS.map((logo) => (
              <figure key={logo.src} className={cn("flex flex-col rounded-lg border p-6", logo.dark ? "border-petrol-700 bg-petrol-900" : "border-line bg-white")}>
                <div className="grid min-h-28 place-items-center">
                  <img src={logo.src} alt={logo.alt} width={logo.w} height={logo.h} className="max-h-24 w-auto" />
                </div>
                <figcaption className={cn("mt-4 text-sm", logo.dark ? "text-petrol-100" : "text-muted")}>{logo.note}</figcaption>
              </figure>
            ))}
          </div>
        </section>

        <section>
          <SectionTitle title="Templates visuais reutilizáveis (SVG determinístico)" />
          <div className="grid gap-6 md:grid-cols-2">
            {CREATIVE_ASSETS.map((asset) => (
              <article key={asset.assetPath} className="overflow-hidden rounded-lg border border-line bg-white">
                <div className="grid place-items-center bg-canvas p-6">
                  <img src={asset.assetPath} alt={asset.title} loading="lazy" className="max-h-80 w-auto rounded-md border border-line" />
                </div>
                <div className="p-6">
                  <Badge>{asset.format}</Badge>
                  <h3 className="mt-3 text-lg font-semibold text-ink">{asset.title}</h3>
                  <p className="mt-2 text-sm text-neutral-700">{asset.notes}</p>
                  <p className="mt-3 text-xs text-muted">
                    <strong>Divulgação obrigatória:</strong> {asset.disclosureText}
                  </p>
                </div>
              </article>
            ))}
          </div>
        </section>

        <section>
          <SectionTitle
            title="Roteiro de vídeo curto vertical (25–40 s · Storyboard)"
            sub="Referência conceitual de etapas separadas (brief → roteiro → arte → revisão → publicação futura), sem alterar perfis nem automações existentes do YoutubeBot."
          />
          <div className="overflow-x-auto rounded-lg border border-line bg-white">
            <table className="w-full min-w-[640px] text-left text-sm">
              <thead className="border-b border-line text-xs tracking-wide text-muted uppercase">
                <tr>
                  <th className="px-4 py-3">Janela</th>
                  <th className="px-4 py-3">Visual / Zona segura</th>
                  <th className="px-4 py-3">Fala e legenda (pt-BR)</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-line">
                {SHORT_VIDEO_STORYBOARD.map((row) => (
                  <tr key={row.window} className="align-top">
                    <td className="px-4 py-3 font-mono font-semibold text-ink">{row.window}</td>
                    <td className="px-4 py-3 text-neutral-700">{row.visual}</td>
                    <td className="px-4 py-3 text-neutral-700">{row.voiceAndSubtitles}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>

        <section className="grid gap-6 lg:grid-cols-2">
          {[
            { id: "tpl-telegram", title: "Formatação para post no Telegram", text: TELEGRAM_POST_TEMPLATE, label: "Copiar rascunho Telegram" },
            { id: "tpl-whatsapp", title: "Pacote manual para WhatsApp do proprietário", text: WHATSAPP_MANUAL_DIGEST_TEMPLATE, label: "Copiar resumo manual WhatsApp" },
          ].map((tpl) => (
            <article key={tpl.id} className="rounded-lg border border-line bg-white p-6">
              <h2 className="text-xl font-semibold text-ink">{tpl.title}</h2>
              <pre id={tpl.id} className="mt-4 overflow-x-auto rounded-lg bg-tile p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap [overflow-wrap:anywhere] text-ink">
                {tpl.text}
              </pre>
              <CopyButton className="mt-3" targetId={tpl.id} label={tpl.label} />
            </article>
          ))}
        </section>
      </div>
    </>
  );
}
