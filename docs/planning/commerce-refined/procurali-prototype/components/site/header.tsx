"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useEffect, useState } from "react";
import { Menu, Search, X } from "lucide-react";
import { BrandLogo } from "@/components/site/brand";
import { SavedTrigger } from "@/components/site/saved";
import { cn } from "@/lib/utils";

const NAV = [
  { href: "/ofertas/", label: "Ofertas", match: "/ofertas" },
  { href: "/guias/", label: "Guias", match: "/guias" },
  { href: "/selecoes-da-semana/", label: "Semana", match: "/selecoes-da-semana" },
  { href: "/encontre-com-ia/", label: "Encontre com IA", match: "/encontre-com-ia" },
  { href: "/grupos/", label: "Grupos", match: "/grupos" },
];

const MORE = [
  { href: "/ofertas/?condicao=com-cupom", label: "Cupons" },
  { href: "/categorias/", label: "Categorias" },
  { href: "/como-selecionamos/", label: "Como selecionamos" },
  { href: "/transparencia/", label: "Transparência" },
  { href: "/sobre/", label: "Sobre" },
];

export function SiteHeader({ telegramUrl }: { telegramUrl: string }) {
  const pathname = usePathname() || "/";
  const [open, setOpen] = useState(false);

  useEffect(() => setOpen(false), [pathname]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open]);

  return (
    <header className="sticky top-0 z-50 border-b border-line bg-canvas/90 backdrop-blur-md">
      <div className="container-site flex h-16 items-center gap-8">
        <Link href="/" aria-label="Procurali — Página inicial" className="shrink-0">
          <BrandLogo />
        </Link>

        <nav aria-label="Navegação principal" className="hidden items-center gap-7 lg:flex">
          {NAV.map(({ href, label, match }) => {
            const active = pathname.startsWith(match);
            return (
              <Link
                key={href}
                href={href}
                aria-current={active ? "page" : undefined}
                className={cn("text-sm transition-colors", active ? "font-medium text-ink" : "text-muted hover:text-ink")}
              >
                {label}
              </Link>
            );
          })}
        </nav>

        <div className="ml-auto flex items-center gap-1">
          <form action="/ofertas/" method="get" role="search" aria-label="Buscar ofertas no catálogo" className="hidden md:block">
            <label className="sr-only" htmlFor="header-search-q">
              Buscar ofertas ou produtos
            </label>
            <div className="relative">
              <Search className="pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-muted" aria-hidden="true" />
              <input
                id="header-search-q"
                type="search"
                name="q"
                placeholder="Buscar"
                className="h-10 w-36 rounded-md border border-transparent bg-tile pr-3 pl-9 text-sm outline-none transition-all placeholder:text-muted focus:w-56 focus:border-ink focus:bg-white"
              />
            </div>
          </form>
          <SavedTrigger />
          <a
            href={telegramUrl}
            target="_blank"
            rel="noopener"
            data-analytics-event="community_link_click"
            data-analytics-detail="telegram-header"
            className="ml-2 hidden min-h-10 items-center rounded-md border border-line px-3.5 text-sm font-medium text-ink transition-colors hover:border-ink sm:inline-flex"
          >
            Telegram
            <span className="sr-only"> — entrar no canal oficial</span>
          </a>
          <button
            type="button"
            onClick={() => setOpen((v) => !v)}
            aria-expanded={open}
            aria-controls="mobile-drawer"
            aria-label={open ? "Fechar menu de navegação" : "Abrir menu de navegação"}
            className="grid h-11 w-11 place-items-center text-ink lg:hidden"
          >
            {open ? <X className="h-5 w-5" aria-hidden="true" /> : <Menu className="h-5 w-5" aria-hidden="true" />}
          </button>
        </div>
      </div>

      {open && (
        <div id="mobile-drawer" className="border-t border-line bg-canvas lg:hidden">
          <div className="container-site py-6">
            <form action="/ofertas/" method="get" role="search" aria-label="Buscar ofertas no menu" className="relative md:hidden">
              <label className="sr-only" htmlFor="drawer-search-q">
                Buscar no catálogo
              </label>
              <Search className="pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-muted" aria-hidden="true" />
              <input
                id="drawer-search-q"
                type="search"
                name="q"
                placeholder="Buscar produto, categoria ou oferta"
                className="h-11 w-full rounded-md bg-tile pr-3 pl-9 text-sm outline-none focus:ring-1 focus:ring-ink"
              />
            </form>
            <ul className="mt-4 divide-y divide-line">
              {NAV.map(({ href, label }) => (
                <li key={href}>
                  <Link href={href} className="flex min-h-12 items-center text-lg font-medium text-ink">
                    {label}
                  </Link>
                </li>
              ))}
            </ul>
            <p className="mt-5 flex flex-wrap gap-x-5 gap-y-2 text-sm text-muted">
              {MORE.map(({ href, label }) => (
                <Link key={href} href={href} className="hover:text-ink">
                  {label}
                </Link>
              ))}
            </p>
          </div>
        </div>
      )}
    </header>
  );
}
