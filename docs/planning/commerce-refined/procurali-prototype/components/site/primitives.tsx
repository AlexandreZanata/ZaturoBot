import Link from "next/link";
import { ArrowRight } from "lucide-react";
import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

export type Tone = "emerald" | "petrol" | "lime" | "warning" | "danger" | "neutral" | "action";

const dotClasses: Record<Tone, string> = {
  emerald: "bg-emerald-600",
  petrol: "bg-petrol-700",
  lime: "bg-emerald-600",
  warning: "bg-amber-500",
  danger: "bg-red-600",
  neutral: "bg-neutral-400",
  action: "bg-action",
};

/** Quiet status label: a small colored dot followed by plain text. */
export function Badge({ tone = "neutral", className, children }: { tone?: Tone; className?: string; children: ReactNode }) {
  return (
    <span className={cn("inline-flex items-center gap-1.5 text-xs text-neutral-700", className)}>
      <span className={cn("h-1.5 w-1.5 shrink-0 rounded-full", dotClasses[tone])} aria-hidden="true" />
      <span>{children}</span>
    </span>
  );
}

const noticeBorder = {
  info: "border-neutral-300",
  warning: "border-amber-500",
  danger: "border-red-600",
  live: "border-emerald-600",
} as const;

export function Notice({
  variant = "info",
  className,
  children,
}: {
  variant?: keyof typeof noticeBorder;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={cn("border-l-2 py-1 pl-4 text-sm leading-relaxed text-neutral-700", noticeBorder[variant], className)}>
      <div className="max-w-3xl space-y-1.5">{children}</div>
    </div>
  );
}

export type Crumb = { label: string; href?: string };

export function Breadcrumbs({ items }: { items: Crumb[]; light?: boolean }) {
  return (
    <nav aria-label="Caminho de navegação" className="mb-6">
      <ol className="flex flex-wrap items-center gap-x-2 gap-y-1 text-[0.8125rem] text-muted">
        {items.map((item, idx) => {
          const last = idx === items.length - 1;
          return (
            <li key={`${item.label}-${idx}`} className="flex min-w-0 items-center gap-2">
              {last || !item.href ? (
                <span aria-current="page" className="truncate text-ink">
                  {item.label}
                </span>
              ) : (
                <Link href={item.href} className="hover:text-ink">
                  {item.label}
                </Link>
              )}
              {!last && <span aria-hidden="true" className="text-neutral-300">/</span>}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

export function Eyebrow({ className, children }: { className?: string; children: ReactNode }) {
  return <p className={cn("text-xs font-medium tracking-[0.08em] text-muted uppercase", className)}>{children}</p>;
}

export function PageHero({
  crumbs,
  kicker,
  title,
  lead,
  children,
}: {
  crumbs: Crumb[];
  kicker?: ReactNode;
  title: ReactNode;
  lead?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <section className="border-b border-line">
      <div className="container-site pt-8 pb-12 sm:pt-10 sm:pb-16">
        <Breadcrumbs items={crumbs} />
        {kicker && <Eyebrow className="mb-4 flex items-center gap-1.5">{kicker}</Eyebrow>}
        <h1 className="max-w-3xl text-3xl leading-[1.1] font-semibold text-balance text-ink sm:text-[2.75rem]">{title}</h1>
        {lead && <p className="mt-5 max-w-2xl text-lg leading-relaxed text-muted">{lead}</p>}
        {children}
      </div>
    </section>
  );
}

export function SectionTitle({
  id,
  title,
  sub,
  action,
}: {
  id?: string;
  icon?: ReactNode;
  title: string;
  sub?: ReactNode;
  action?: ReactNode;
}) {
  return (
    <div className="mb-8 flex flex-wrap items-end justify-between gap-x-6 gap-y-2">
      <div className="min-w-0">
        <h2 id={id} className="text-2xl font-semibold text-ink">
          {title}
        </h2>
        {sub && <p className="mt-1.5 max-w-2xl text-sm text-muted">{sub}</p>}
      </div>
      {action}
    </div>
  );
}

export function MoreLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <Link href={href} className="group inline-flex min-h-11 items-center gap-1.5 text-sm font-medium text-ink">
      <span className="underline decoration-neutral-300 underline-offset-4 group-hover:decoration-ink">{children}</span>
      <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-0.5" aria-hidden="true" />
    </Link>
  );
}

export function Card({ className, children }: { className?: string; children: ReactNode }) {
  return <div className={cn("rounded-lg border border-line bg-white p-6", className)}>{children}</div>;
}

const buttonVariants = {
  primary: "bg-ink text-white hover:bg-petrol-800",
  action: "bg-petrol-800 text-white hover:bg-petrol-900",
  lime: "bg-ink text-white hover:bg-petrol-800",
  secondary: "border border-line bg-white text-ink hover:border-ink",
  ghost: "text-ink hover:bg-tile",
  disabled: "cursor-not-allowed border border-dashed border-neutral-300 text-neutral-400",
} as const;

export type ButtonVariant = keyof typeof buttonVariants;

export function buttonClass(variant: ButtonVariant = "primary", className?: string) {
  return cn(
    "tap inline-flex min-h-11 items-center justify-center gap-2 rounded-md px-4 py-2.5 text-sm font-medium transition-colors",
    buttonVariants[variant],
    className
  );
}

export function JsonLd({ data }: { data: object[] }) {
  return (
    <>
      {data.map((obj, idx) => (
        <script
          key={idx}
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(obj, null, 2).replace(/</g, "\\u003c") }}
        />
      ))}
    </>
  );
}
