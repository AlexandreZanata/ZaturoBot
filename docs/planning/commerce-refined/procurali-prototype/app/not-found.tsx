import Link from "next/link";
import { buttonClass } from "@/components/site/primitives";

export const metadata = {
  title: { absolute: "Página não encontrada (404) — Procurali" },
  description: "Página não encontrada neste protótipo do Procurali.",
};

export default function NotFound() {
  return (
    <section className="container-site grid min-h-[60vh] place-items-center py-20 text-center">
      <div className="max-w-xl">
        <p className="font-mono text-sm text-muted">Erro 404</p>
        <h1 className="mt-2 text-5xl font-semibold text-ink">Página não encontrada</h1>
        <p className="mt-3 text-lg text-muted">O endereço acessado não existe ou foi movido neste protótipo do Procurali.</p>
        <div className="mt-8 flex flex-wrap justify-center gap-3">
          <Link href="/" className={buttonClass("primary")}>
            Voltar para a página inicial
          </Link>
          <Link href="/ofertas/" className={buttonClass("secondary")}>
            Explorar ofertas (demo)
          </Link>
          <Link href="/guias/" className={buttonClass("secondary")}>
            Ver guias de compra
          </Link>
        </div>
      </div>
    </section>
  );
}
