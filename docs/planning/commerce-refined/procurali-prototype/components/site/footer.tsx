import Link from "next/link";
import { Send } from "lucide-react";
import { SITE_META } from "@/data/demo-catalog.mjs";
import { BrandLogo } from "@/components/site/brand";

const COLUMNS = [
  {
    title: "Explorar",
    links: [
      ["/ofertas/", "Ofertas (demo)"],
      ["/categorias/", "Categorias iniciais"],
      ["/guias/", "Guias de compra"],
      ["/selecoes-da-semana/", "Seleções da semana"],
      ["/encontre-com-ia/", "Encontre com IA (demo)"],
      ["/grupos/", "Diretório de grupos"],
    ],
  },
  {
    title: "Confiança e regras",
    links: [
      ["/sobre/", "Sobre o Procurali"],
      ["/como-selecionamos/", "Como selecionamos"],
      ["/transparencia/", "Transparência de afiliados"],
      ["/privacidade/", "Privacidade (rascunho)"],
      ["/termos/", "Termos de uso (rascunho)"],
      ["/contato/", "Contato"],
    ],
  },
  {
    title: "Kit e handoff",
    links: [
      ["/criativos/", "Kit criativo e templates SVG"],
      ["/cms-preview/", "Mockup CMS (não produtivo)"],
    ],
  },
];

export function SiteFooter() {
  return (
    <footer className="mt-24 border-t border-line pb-24 md:pb-0">
      <div className="container-site grid gap-12 py-16 md:grid-cols-2 lg:grid-cols-[1.5fr_1fr_1fr_1fr]">
        <div className="max-w-xs">
          <Link href="/" aria-label="Procurali — voltar ao início">
            <BrandLogo />
          </Link>
          <p className="mt-5 text-sm leading-relaxed text-muted">
            Guia independente de compras e curadoria de ofertas com foco em contexto, transparência e decisão consciente.
            Não vendemos estoque nem processamos pagamentos.
          </p>
          <p className="mt-4 text-sm text-ink">
            “{SITE_META.proposedTagline}” <span className="text-muted">— proposta em aprovação</span>
          </p>
          <a
            href={SITE_META.telegramUrl}
            target="_blank"
            rel="noopener"
            data-analytics-event="community_link_click"
            data-analytics-detail="telegram-footer"
            className="mt-5 inline-flex min-h-11 items-center gap-2 text-sm font-medium text-ink underline decoration-neutral-300 underline-offset-4 hover:decoration-ink"
          >
            <Send className="h-4 w-4" aria-hidden="true" /> Seguir {SITE_META.telegramHandle}
          </a>
        </div>
        {COLUMNS.map((col) => (
          <div key={col.title}>
            <h2 className="text-xs font-medium tracking-[0.08em] text-muted uppercase">{col.title}</h2>
            <ul className="mt-4 space-y-3 text-sm">
              {col.links.map(([href, label]) => (
                <li key={href}>
                  <Link href={href} className="text-ink hover:text-petrol-600">
                    {label}
                  </Link>
                </li>
              ))}
              {col.title === "Kit e handoff" && (
                <li>
                  <a href={SITE_META.telegramUrl} target="_blank" rel="noopener" className="text-ink hover:text-petrol-600">
                    Canal oficial {SITE_META.telegramHandle}
                  </a>
                </li>
              )}
            </ul>
          </div>
        ))}
      </div>
      <div className="border-t border-line">
        <div className="container-site flex flex-col gap-2 py-6 text-xs text-muted sm:flex-row sm:justify-between">
          <span>{SITE_META.affiliateDisclosure}</span>
          <span>
            Ambiente de testes sem coleta oculta · ofertas reais sinalizadas como link de afiliado · {SITE_META.domain}
          </span>
        </div>
      </div>
    </footer>
  );
}
