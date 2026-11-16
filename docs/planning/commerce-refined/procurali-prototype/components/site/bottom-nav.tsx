"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { BookOpen, Home, Sparkles, Tag, Users } from "lucide-react";
import { cn } from "@/lib/utils";

const ITEMS = [
  { href: "/", label: "Início", icon: Home, match: (p: string) => p === "/" },
  { href: "/ofertas/", label: "Ofertas", icon: Tag, match: (p: string) => p.startsWith("/ofertas") },
  { href: "/guias/", label: "Guias", icon: BookOpen, match: (p: string) => p.startsWith("/guias") },
  { href: "/encontre-com-ia/", label: "IA", icon: Sparkles, match: (p: string) => p.startsWith("/encontre-com-ia") },
  { href: "/grupos/", label: "Grupos", icon: Users, match: (p: string) => p.startsWith("/grupos") },
];

export function BottomNav() {
  const pathname = usePathname() || "/";
  return (
    <nav
      aria-label="Navegação rápida"
      className="fixed inset-x-0 bottom-0 z-40 border-t border-line bg-canvas/95 pb-[env(safe-area-inset-bottom)] backdrop-blur-md md:hidden"
    >
      <ul className="grid grid-cols-5">
        {ITEMS.map(({ href, label, icon: Icon, match }) => {
          const active = match(pathname);
          return (
            <li key={href}>
              <Link
                href={href}
                aria-current={active ? "page" : undefined}
                className={cn(
                  "flex min-h-14 flex-col items-center justify-center gap-1 text-[0.68rem] transition-colors",
                  active ? "font-medium text-ink" : "text-muted"
                )}
              >
                <Icon className="h-5 w-5" strokeWidth={active ? 2 : 1.5} aria-hidden="true" />
                {label}
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
