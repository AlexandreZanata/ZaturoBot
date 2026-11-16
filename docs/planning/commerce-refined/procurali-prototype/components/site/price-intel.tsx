import { formatBrl } from "@/lib/contracts.mjs";
import { sanitizeSparklineSvg } from "@/lib/price-intel";
import type { PriceIntelligence } from "@/lib/types";
import { Badge } from "@/components/site/primitives";

export function PriceIntelPanel({ intel }: { intel: PriceIntelligence }) {
  const days = intel.windowDays || 30;
  const records =
    Number.isFinite(intel.observationCount) && (intel.observationCount as number) > 0
      ? ` · ${intel.observationCount} registros`
      : "";
  const tone = intel.priceLevel === "lowest_price" ? "emerald" : intel.priceLevel === "good_price" ? "petrol" : "neutral";
  const sparkline = sanitizeSparklineSvg(intel.svgSparkline);

  return (
    <section className="border-t border-line pt-8">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
            <Badge tone={tone}>{intel.priceLevelLabel || "Preço observado"}</Badge>
            {intel.priceTrend === "down" && <Badge tone="emerald">Em queda ({intel.priceDeltaPct || 0}%)</Badge>}
            {intel.priceTrend === "up" && (
              <Badge>Acima da captura anterior (+{intel.priceDeltaPct || 0}%)</Badge>
            )}
          </div>
          <h2 className="mt-3 text-xl font-semibold text-ink">
            {`Histórico e Inteligência de Preço (${days} dias${records})`}
          </h2>
        </div>
        {intel.verifiedAtDisplay && <p className="text-sm text-muted">{intel.verifiedAtDisplay}</p>}
      </div>
      <dl className="mt-6 grid gap-6 sm:grid-cols-3">
        <div>
          <dt className="text-xs text-muted">Menor preço (30d)</dt>
          <dd className="mt-1 text-lg font-semibold text-ink tabular-nums">{formatBrl(intel.lowestPrice30dCents as number)}</dd>
        </div>
        <div>
          <dt className="text-xs text-muted">Preço médio (30d)</dt>
          <dd className="mt-1 text-lg font-semibold text-ink tabular-nums">{formatBrl(intel.averagePrice30dCents as number)}</dd>
        </div>
        <div>
          <dt className="text-xs text-muted">Maior preço (30d)</dt>
          <dd className="mt-1 text-lg font-semibold text-ink tabular-nums">{formatBrl(intel.highestPrice30dCents as number)}</dd>
        </div>
      </dl>
      {sparkline && (
        <div
          className="mt-6 max-w-md text-emerald-700"
          role="img"
          aria-label="Tendência de preço nos últimos 30 dias"
          dangerouslySetInnerHTML={{ __html: sparkline }}
        />
      )}
    </section>
  );
}
