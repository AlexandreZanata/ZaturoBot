import { ArrowUpRight } from "lucide-react";
import { COMMUNITIES } from "@/data/demo-catalog.mjs";
import { breadcrumbLd, pageMetadata } from "@/lib/seo";
import { Badge, JsonLd, PageHero, buttonClass } from "@/components/site/primitives";

export const metadata = pageMetadata({
  route: "/grupos/",
  title: "Grupos e canal no Telegram",
  description:
    "Diretório voluntário das comunidades do Procurali: canal oficial @procurali no Telegram e prévia de grupos no WhatsApp.",
});

export default function GroupsPage() {
  return (
    <>
      <JsonLd data={[breadcrumbLd([{ name: "Início", path: "/" }, { name: "Grupos", path: "/grupos/" }])]} />
      <PageHero
        crumbs={[{ label: "Início", href: "/" }, { label: "Grupos e comunidades" }]}
        kicker="Comunidades oficiais do proprietário"
        title="Canais e grupos para acompanhar ofertas"
        lead="Diretório exclusivo das comunidades geridas pelo proprietário do Procurali. A entrada é sempre voluntária, sem notificações forçadas nem envio automático em massa."
      />
      <div className="container-site grid gap-x-12 gap-y-14 py-14 md:grid-cols-2">
        {COMMUNITIES.map((comm) => {
          const active = comm.network === "telegram" && comm.availabilityState === "active" && Boolean(comm.inviteUrl);
          const status = active
            ? "Canal oficial ativo"
            : comm.availabilityState === "pending_owner_invite"
              ? "Prévia — aguardando convite do proprietário"
              : comm.availabilityState === "revoked_rotating"
                ? "Convite em rotação"
                : "Grupo fechado / indisponível";
          const tone = active ? "emerald" : comm.availabilityState === "pending_owner_invite" ? "warning" : "neutral";
          return (
            <article key={comm.id} className="flex flex-col border-t border-line pt-6">
              <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
                <span className="text-xs font-medium tracking-[0.08em] text-muted uppercase">
                  {comm.network === "telegram" ? "Telegram" : "WhatsApp"}
                </span>
                <Badge tone={tone}>{status}</Badge>
              </div>
              <h2 className="mt-3 text-2xl font-semibold text-ink">{comm.name}</h2>
              <p className="mt-1 font-mono text-sm text-muted">{comm.handle}</p>
              <p className="mt-4 leading-relaxed text-neutral-700">{comm.purpose}</p>
              <dl className="mt-5 space-y-1.5 text-sm text-neutral-700">
                <div>
                  <dt className="inline text-muted">Tema: </dt>
                  <dd className="inline">{comm.categoryTopic}</dd>
                </div>
                <div>
                  <dt className="inline text-muted">Frequência prevista: </dt>
                  <dd className="inline">{comm.intendedFrequency}</dd>
                </div>
              </dl>
              <div className="mt-5 text-sm">
                <strong className="font-medium text-ink">Regras básicas:</strong>
                <ul className="mt-2 list-disc space-y-1.5 pl-5 text-neutral-700 marker:text-neutral-300">
                  {comm.rules.map((r) => (
                    <li key={r}>{r}</li>
                  ))}
                </ul>
              </div>
              <p className="mt-5 border-l-2 border-neutral-300 pl-4 text-sm leading-relaxed text-muted">
                <strong className="font-medium text-ink">Privacidade antes de entrar:</strong> {comm.privacyNotice}
              </p>
              <div className="mt-auto pt-7">
                {active ? (
                  <a
                    href={comm.inviteUrl as string}
                    target="_blank"
                    rel="noopener"
                    data-analytics-event="community_link_click"
                    data-analytics-detail={comm.slug}
                    className={buttonClass("primary")}
                  >
                    {comm.ctaLabel} <ArrowUpRight className="h-4 w-4" aria-hidden="true" />
                  </a>
                ) : (
                  <button type="button" disabled aria-disabled="true" className={buttonClass("disabled")}>
                    {comm.ctaLabel}
                  </button>
                )}
              </div>
            </article>
          );
        })}
      </div>
    </>
  );
}
