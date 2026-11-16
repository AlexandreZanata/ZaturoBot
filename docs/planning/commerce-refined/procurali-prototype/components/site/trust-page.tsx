import { breadcrumbLd } from "@/lib/seo";
import { JsonLd, PageHero } from "@/components/site/primitives";
import type { ReactNode } from "react";

/** Shared layout for the editorial/trust pages (about, policies, contact). */
export function TrustPage({
  route,
  crumb,
  ldName,
  kicker,
  title,
  lead,
  children,
}: {
  route: string;
  crumb: string;
  ldName: string;
  kicker: ReactNode;
  title: ReactNode;
  lead?: ReactNode;
  children: ReactNode;
}) {
  return (
    <>
      <JsonLd data={[breadcrumbLd([{ name: "Início", path: "/" }, { name: ldName, path: route }])]} />
      <PageHero crumbs={[{ label: "Início", href: "/" }, { label: crumb }]} kicker={kicker} title={title} lead={lead} />
      <div className="container-site py-12">
        <div className="prose-site">{children}</div>
      </div>
    </>
  );
}
