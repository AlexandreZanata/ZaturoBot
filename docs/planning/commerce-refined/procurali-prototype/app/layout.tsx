import type { Metadata, Viewport } from "next";
import localFont from "next/font/local";
import { SITE_META } from "@/data/demo-catalog.mjs";
import { getCatalog } from "@/lib/catalog";
import { SiteHeader } from "@/components/site/header";
import { SiteFooter } from "@/components/site/footer";
import { BottomNav } from "@/components/site/bottom-nav";
import { SavedProvider } from "@/components/site/saved";
import { AnalyticsListener } from "@/components/site/analytics";
import "./globals.css";

const firaSans = localFont({
  src: [
    { path: "./fonts/FiraSans-Regular.woff2", weight: "400" },
    { path: "./fonts/FiraSans-Medium.woff2", weight: "500" },
    { path: "./fonts/FiraSans-SemiBold.woff2", weight: "600" },
    { path: "./fonts/FiraSans-Bold.woff2", weight: "700" },
    { path: "./fonts/FiraSans-ExtraBold.woff2", weight: "800" },
  ],
  variable: "--font-fira-sans",
  display: "swap",
});

const firaMono = localFont({
  src: "./fonts/FiraMono-Regular.woff2",
  variable: "--font-fira-mono",
  display: "swap",
});

export const metadata: Metadata = {
  metadataBase: new URL(`https://${SITE_META.domain}`),
  robots: {
    index: true,
    follow: true,
    googleBot: { index: true, follow: true, "max-image-preview": "large", "max-snippet": -1 },
  },
  icons: {
    icon: [
      { url: "/brand/procurali_icone_branco_transparente.svg", type: "image/svg+xml" },
      { url: "/brand/png/favicon-32.png", sizes: "32x32", type: "image/png" },
    ],
    apple: [{ url: "/brand/png/apple-touch-icon-180.png", sizes: "180x180" }],
  },
};

export const viewport: Viewport = { themeColor: "#143D2B", width: "device-width", initialScale: 1 };

export default function RootLayout({ children }: { children: React.ReactNode }) {
  const offers = getCatalog().offers;
  const liveCount = offers.filter((o) => !o.isDemoFixture && o.state === "approved").length;
  const demoCount = offers.filter((o) => o.isDemoFixture).length;
  return (
    <html lang="pt-BR" className={`${firaSans.variable} ${firaMono.variable}`}>
      <body>
        <a
          href="#conteudo-principal"
          className="sr-only z-[70] rounded-md bg-ink px-4 py-2 text-sm font-medium text-white focus:not-sr-only focus:fixed focus:top-3 focus:left-3"
        >
          Pular para o conteúdo principal
        </a>
        <SavedProvider>
          <div role="note" aria-label="Aviso de ambiente de testes" className="border-b border-line text-xs text-muted">
            <div className="container-site flex min-h-8 items-center justify-between gap-4 py-1.5">
              <span>
                <strong className="font-medium text-ink">Catálogo em teste</strong>
                <span className="hidden sm:inline">
                  {" "}
                  — {liveCount} {liveCount === 1 ? "oferta real" : "ofertas reais"} do Mercado Livre
                  {demoCount > 0 ? " + fixtures de estado" : ""}
                </span>
              </span>
              <span className="hidden md:inline">
                Canal oficial:{" "}
                <a href={SITE_META.telegramUrl} target="_blank" rel="noopener" className="text-ink underline decoration-neutral-300 underline-offset-2">
                  {SITE_META.telegramHandle}
                </a>
              </span>
            </div>
          </div>
          <SiteHeader telegramUrl={SITE_META.telegramUrl} />
          <main id="conteudo-principal" tabIndex={-1} className="outline-none">
            {children}
          </main>
          <SiteFooter />
          <BottomNav />
          <AnalyticsListener />
        </SavedProvider>
      </body>
    </html>
  );
}
