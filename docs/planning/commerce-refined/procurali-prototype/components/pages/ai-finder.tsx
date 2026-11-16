"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { runDeterministicFinder } from "@/lib/contracts.mjs";
import type { Offer } from "@/lib/types";
import { recordLocalEvent } from "@/components/site/analytics";
import { Badge, Notice, buttonClass } from "@/components/site/primitives";
import { cn } from "@/lib/utils";

type Input = { need: string; maxBudgetBrl: number; requirement: string; simulatedState: string };

const PRESETS: (Input & { label: string })[] = [
  { label: "Exemplo com resultado", need: "notebook mesa", maxBudgetBrl: 180, requirement: "qualquer", simulatedState: "normal" },
  { label: "Sem correspondência", need: "cadeira reclinável", maxBudgetBrl: 40, requirement: "qualquer", simulatedState: "normal" },
  { label: "Simular teto US$ 20", need: "hub usb", maxBudgetBrl: 180, requirement: "qualquer", simulatedState: "budget-cap" },
];

const field =
  "h-11 w-full rounded-md border border-line bg-white px-3 text-sm outline-none transition focus:border-ink";

export function AiFinder({ offers }: { offers: Offer[] }) {
  const [input, setInput] = useState<Input>({
    need: "notebook postura mesa",
    maxBudgetBrl: 180,
    requirement: "qualquer",
    simulatedState: "normal",
  });
  const [result, setResult] = useState<ReturnType<typeof runDeterministicFinder> | null>(null);

  function evaluate(next: Input) {
    const res = runDeterministicFinder(next, offers);
    recordLocalEvent("finder_demo_query", `status=${res.status}`);
    setResult(res);
  }

  useEffect(() => {
    evaluate(input);
    // Initial demonstration result only.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const tone = result?.status === "match" ? "info" : result?.status === "no-match" ? "warning" : "danger";

  return (
    <div className="grid gap-x-16 gap-y-12 lg:grid-cols-[340px_1fr]">
      <form
        id="ai-finder-form"
        aria-label="Formulário guiado de seleção"
        onSubmit={(e) => {
          e.preventDefault();
          evaluate(input);
        }}
        className="h-fit space-y-5 lg:sticky lg:top-24"
      >
        <h2 className="flex items-center gap-2 text-lg font-semibold text-ink">
          Conte o que você precisa
        </h2>
        <div>
          <label htmlFor="ai-need" className="mb-1.5 block text-xs font-semibold text-neutral-700">
            O que você precisa resolver?
          </label>
          <input
            id="ai-need"
            name="need"
            type="text"
            maxLength={120}
            value={input.need}
            onChange={(e) => setInput({ ...input, need: e.target.value })}
            placeholder="Ex.: conectar monitor HDMI, melhorar postura, iluminar mesa"
            className={field}
          />
        </div>
        <div>
          <label htmlFor="ai-budget" className="mb-1.5 flex justify-between text-xs font-semibold text-neutral-700">
            <span>Quanto pretende gastar no máximo? (R$)</span>
            <span className="font-mono text-ink">R$ {input.maxBudgetBrl}</span>
          </label>
          <input
            id="ai-budget"
            name="budget"
            type="range"
            min={20}
            max={1000}
            step={10}
            value={input.maxBudgetBrl}
            onChange={(e) => setInput({ ...input, maxBudgetBrl: Number(e.target.value) })}
            className="w-full accent-neutral-900"
          />
        </div>
        <div>
          <label htmlFor="ai-req" className="mb-1.5 block text-xs font-semibold text-neutral-700">
            O que não pode faltar?
          </label>
          <select id="ai-req" name="requirement" value={input.requirement} onChange={(e) => setInput({ ...input, requirement: e.target.value })} className={field}>
            <option value="qualquer">Sem restrição específica</option>
            <option value="ergonomia">Ergonomia e suporte</option>
            <option value="energia">Energia e cabos</option>
            <option value="iluminacao">Iluminação de bancada</option>
            <option value="sem-cupom">Sem depender de cupom</option>
          </select>
        </div>
        <div>
          <label htmlFor="ai-sim" className="mb-1.5 block text-xs font-semibold text-neutral-700">
            Simular estado do sistema (validação de protótipo)
          </label>
          <select id="ai-sim" name="simulatedState" value={input.simulatedState} onChange={(e) => setInput({ ...input, simulatedState: e.target.value })} className={field}>
            <option value="normal">Normal (catálogo demonstrativo ativo)</option>
            <option value="source-stale">Catálogo desatualizado (&gt;30 min)</option>
            <option value="service-unavailable">Serviço indisponível</option>
            <option value="budget-cap">Teto mensal de US$ 20 atingido</option>
          </select>
        </div>
        <p className="text-xs leading-relaxed text-muted">
          <strong>Privacidade:</strong> não informe CPF, dados bancários, senhas, endereço completo ou credenciais de conta.
        </p>
        <button type="submit" className={buttonClass("primary", "w-full")}>
          Buscar no catálogo demo
        </button>
        <div>
          <p className="mb-2 text-xs text-muted">Atalhos de teste de estados:</p>
          <div className="flex flex-wrap gap-1.5">
            {PRESETS.map(({ label, ...preset }) => (
              <button
                key={label}
                type="button"
                onClick={() => {
                  setInput(preset);
                  evaluate(preset);
                }}
                className="min-h-9 rounded-full border border-line px-3 py-1.5 text-xs font-semibold text-ink hover:border-ink hover:bg-tile"
              >
                {label}
              </button>
            ))}
          </div>
        </div>
      </form>

      <div className="min-w-0">
        <h2 className="mb-4 text-xl font-semibold text-ink">Resultado da seleção guiada</h2>
        <div id="ai-finder-output" aria-live="polite">
          {!result ? (
            <Notice>
              <p>
                <Badge tone="petrol">Demonstração — sem consulta ao vivo</Badge>
              </p>
              <p>
                Se o JavaScript estiver desativado, consulte diretamente a{" "}
                <Link href="/ofertas/" className="font-semibold underline">
                  lista completa de ofertas
                </Link>{" "}
                ou os{" "}
                <Link href="/guias/" className="font-semibold underline">
                  guias de compra
                </Link>{" "}
                sem depender do seletor interativo.
              </p>
            </Notice>
          ) : (
            <AnimatePresence mode="wait">
              <motion.div key={`${result.status}-${result.matches.map((m) => m.slug).join(",")}`} initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }}>
                <Notice variant={tone}>
                  <p>
                    <Badge tone="petrol">{result.modeLabel}</Badge>
                  </p>
                  <p className="font-semibold">{result.explanation}</p>
                  <p className="text-xs opacity-80">Base consultada: {result.catalogTimestamp}</p>
                </Notice>
                {result.matches.length > 0 ? (
                  <div className="mt-8 grid gap-x-8 gap-y-10 md:grid-cols-2 xl:grid-cols-3">
                    {result.matches.map((m, idx) => (
                      <motion.article
                        key={m.slug}
                        initial={{ opacity: 0, y: 12 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ delay: idx * 0.08 }}
                        className="flex flex-col border-t border-line pt-5"
                      >
                        <Badge tone="lime" className="w-fit">
                          Compatível na demonstração
                        </Badge>
                        <h3 className="mt-3 font-semibold text-ink">{m.title}</h3>
                        <p className="text-xs text-muted">{m.exactVariant}</p>
                        <p className="mt-2 text-xl font-semibold text-ink tabular-nums">{m.priceFormatted}</p>
                        <p className="text-xs text-muted">Observado em {m.observedAtDisplay}</p>
                        <dl className="mt-3 space-y-2 text-sm text-neutral-700">
                          <div>
                            <dt className="font-semibold text-ink">Por que encaixa</dt>
                            <dd>{m.fitReason}</dd>
                          </div>
                          <div>
                            <dt className="font-semibold text-ink">Limitação principal</dt>
                            <dd>{m.mainTradeoff}</dd>
                          </div>
                          <div className="text-muted">
                            <dt className="font-semibold">O que não sabemos</dt>
                            <dd>{m.unknownItem}</dd>
                          </div>
                        </dl>
                        <Link href={`/ofertas/${m.slug}/`} className={buttonClass("secondary", "mt-5 w-full")}>
                          Ver ficha e condições
                        </Link>
                      </motion.article>
                    ))}
                  </div>
                ) : (
                  <div className={cn("mt-5 flex flex-wrap gap-2")}>
                    <Link href="/ofertas/" className={buttonClass("primary")}>
                      Explorar ofertas com filtros normais
                    </Link>
                    <Link href="/guias/" className={buttonClass("secondary")}>
                      Ler guias de compra
                    </Link>
                  </div>
                )}
              </motion.div>
            </AnimatePresence>
          )}
        </div>
      </div>
    </div>
  );
}
