"use client";

import { useEffect, useMemo, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { RotateCcw, Search } from "lucide-react";
import { filterAndSortOffers } from "@/lib/contracts.mjs";
import type { Category, MediaRight, Offer } from "@/lib/types";
import { OfferRow } from "@/components/site/offer";
import { Notice } from "@/components/site/primitives";
import { cn } from "@/lib/utils";

type FilterState = {
  query: string;
  category: string;
  priceBand: string;
  condition: string;
  sort: string;
  statusFilter: string;
};

const DEFAULTS: FilterState = {
  query: "",
  category: "todas",
  priceBand: "todas",
  condition: "todas",
  sort: "relevance",
  statusFilter: "ativas",
};

const PARAMS: [keyof FilterState, string][] = [
  ["query", "q"],
  ["category", "categoria"],
  ["priceBand", "faixa"],
  ["condition", "condicao"],
  ["sort", "ordem"],
  ["statusFilter", "status"],
];

const SORT_LABELS: Record<string, string> = {
  relevance: "Relevância editorial",
  price_asc: "Menor preço informado (frete não incluído)",
  recent: "Observação mais recente",
};

const fieldClass =
  "h-11 w-full rounded-md border border-line bg-white px-3 text-sm text-ink outline-none transition focus:border-ink";

function Chip({ active, onClick, children }: { active: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        "min-h-9 rounded-full border px-3 py-1.5 text-[0.8125rem] transition-colors",
        active ? "border-ink bg-ink text-white" : "border-line text-neutral-700 hover:border-ink"
      )}
    >
      {children}
    </button>
  );
}

export function OffersExplorer({
  offers,
  categories,
  mediaRights,
}: {
  offers: Offer[];
  categories: Category[];
  mediaRights: MediaRight[];
}) {
  const [state, setState] = useState<FilterState>(DEFAULTS);
  const [hydrated, setHydrated] = useState(false);

  // Apply shareable filters from the URL (e.g. ?condicao=com-cupom or the header search ?q=).
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    setState((prev) => {
      const next = { ...prev };
      for (const [key, param] of PARAMS) {
        const value = params.get(param);
        if (value) next[key] = value;
      }
      return next;
    });
    setHydrated(true);
  }, []);

  useEffect(() => {
    if (!hydrated) return;
    const params = new URLSearchParams();
    for (const [key, param] of PARAMS) {
      const value = state[key].trim();
      if (value && value !== DEFAULTS[key]) params.set(param, value);
    }
    const qs = params.toString();
    try {
      window.history.replaceState(null, "", qs ? `${window.location.pathname}?${qs}` : window.location.pathname);
    } catch {
      // Ignore restricted origins.
    }
  }, [state, hydrated]);

  const results = useMemo(() => filterAndSortOffers(offers, state), [offers, state]);
  const set = (patch: Partial<FilterState>) => setState((prev) => ({ ...prev, ...patch }));
  const dirty = PARAMS.some(([key]) => state[key] !== DEFAULTS[key]);

  return (
    <div className="grid gap-x-16 gap-y-10 xl:grid-cols-[260px_1fr]">
      <form
        id="offers-filter-form"
        aria-label="Filtros de ofertas"
        onSubmit={(e) => e.preventDefault()}
        className="h-fit xl:sticky xl:top-24"
      >
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-xs font-medium tracking-[0.08em] text-muted uppercase">
            Filtros e ordenação
          </h2>
          {dirty && (
            <button
              type="button"
              id="offers-reset-btn"
              onClick={() => setState(DEFAULTS)}
              className="inline-flex min-h-9 items-center gap-1 rounded-lg px-2 text-xs font-semibold text-ink hover:bg-tile"
            >
              <RotateCcw className="h-3.5 w-3.5" aria-hidden="true" /> Limpar
            </button>
          )}
        </div>
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-1">
          <div className="sm:col-span-2 lg:col-span-3 xl:col-span-1">
            <label htmlFor="f-q" className="mb-1.5 block text-xs font-semibold text-neutral-700">
              Buscar por produto, marca, modelo ou código
            </label>
            <div className="relative">
              <Search className="pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-neutral-400" aria-hidden="true" />
              <input
                id="f-q"
                name="q"
                type="search"
                value={state.query}
                onChange={(e) => set({ query: e.target.value })}
                placeholder="Kingston, Redragon, 200Hz..."
                className={cn(fieldClass, "pl-9")}
              />
            </div>
          </div>
          <fieldset className="sm:col-span-2 lg:col-span-3 xl:col-span-1">
            <legend className="mb-1.5 text-xs font-semibold text-neutral-700">Categoria</legend>
            <div className="flex flex-wrap gap-1.5">
              <Chip active={state.category === "todas"} onClick={() => set({ category: "todas" })}>
                Todas
              </Chip>
              {categories.map((c) => (
                <Chip key={c.slug} active={state.category === c.slug} onClick={() => set({ category: c.slug })}>
                  {c.name}
                </Chip>
              ))}
            </div>
          </fieldset>
          <div>
            <label htmlFor="f-band" className="mb-1.5 block text-xs font-semibold text-neutral-700">
              Faixa de preço
            </label>
            <select id="f-band" name="faixa" value={state.priceBand} onChange={(e) => set({ priceBand: e.target.value })} className={fieldClass}>
              <option value="todas">Qualquer valor</option>
              <option value="ate-100">Até R$ 100</option>
              <option value="100-a-150">De R$ 100 a R$ 150</option>
              <option value="150-a-300">De R$ 150 a R$ 300</option>
              <option value="acima-de-300">Acima de R$ 300 (hardware)</option>
            </select>
          </div>
          <div>
            <label htmlFor="f-cond" className="mb-1.5 block text-xs font-semibold text-neutral-700">
              Condição
            </label>
            <select id="f-cond" name="condicao" value={state.condition} onChange={(e) => set({ condition: e.target.value })} className={fieldClass}>
              <option value="todas">Todas as condições</option>
              <option value="amostra-7-dias">Com série de 7 dias</option>
              <option value="com-cupom">Com cupom informado</option>
              <option value="pix-sem-cupom">Sem exigência de cupom</option>
            </select>
          </div>
          <div>
            <label htmlFor="f-sort" className="mb-1.5 block text-xs font-semibold text-neutral-700">
              Ordenação explícita
            </label>
            <select id="f-sort" name="ordem" value={state.sort} onChange={(e) => set({ sort: e.target.value })} className={fieldClass}>
              <option value="relevance">Relevância editorial</option>
              <option value="price_asc">Menor preço informado (sem frete)</option>
              <option value="recent">Observação mais recente</option>
            </select>
          </div>
          <div>
            <label htmlFor="f-status" className="mb-1.5 block text-xs font-semibold text-neutral-700">
              Estado do registro
            </label>
            <select id="f-status" name="status" value={state.statusFilter} onChange={(e) => set({ statusFilter: e.target.value })} className={fieldClass}>
              <option value="ativas">Somente ativas (ML + demo)</option>
              <option value="nao-ativas">Desatualizadas / expiradas / bloqueadas</option>
              <option value="todas">Mostrar todos os estados</option>
            </select>
          </div>
        </div>
      </form>

      <div className="min-w-0">
        <p id="offers-results-count" aria-live="polite" className="mb-4 text-sm text-muted">
          <strong className="text-ink">{results.length} oferta(s)</strong> exibida(s) · Ordenação:{" "}
          {SORT_LABELS[state.sort] || SORT_LABELS.relevance}.
        </p>
        <div id="offers-results" className="border-b border-line">
          <AnimatePresence initial={false} mode="popLayout">
            {results.length === 0 ? (
              <motion.div key="empty" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                <Notice variant="warning">
                  <h3 className="font-semibold">Nenhuma oferta encontrada com esses filtros</h3>
                  <p>
                    Nenhuma opção do catálogo demonstrativo corresponde a todos os critérios escolhidos (ou itens
                    desatualizados/expirados estão ocultos pelo filtro atual).
                  </p>
                  <button
                    type="button"
                    onClick={() => setState(DEFAULTS)}
                    className="mt-1 inline-flex min-h-10 items-center rounded-md border border-amber-300 bg-white px-4 text-sm font-semibold"
                  >
                    Limpar filtros
                  </button>
                </Notice>
              </motion.div>
            ) : (
              results.map((offer) => (
                <motion.div
                  key={offer.slug}
                  layout
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, scale: 0.98 }}
                  transition={{ duration: 0.2 }}
                >
                  <OfferRow offer={offer} mediaRights={mediaRights} headingLevel={2} />
                </motion.div>
              ))
            )}
          </AnimatePresence>
        </div>
      </div>
    </div>
  );
}
