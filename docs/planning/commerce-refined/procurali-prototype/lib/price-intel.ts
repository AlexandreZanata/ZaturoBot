import type { PriceIntelligence } from "@/lib/types";

/** Keep a hub sparkline only when it is a single SVG without active content. */
export function sanitizeSparklineSvg(raw: unknown): string {
  if (typeof raw !== "string") return "";
  const trimmed = raw.trim();
  if (!trimmed.startsWith("<svg") || !trimmed.endsWith("</svg>")) return "";
  const lower = trimmed.toLowerCase();
  if (
    lower.includes("<script") ||
    lower.includes("<foreignobject") ||
    lower.includes("javascript:") ||
    /\son[a-z]+\s*=/.test(lower)
  ) {
    return "";
  }
  return trimmed;
}

export function usablePriceIntelligence(value: PriceIntelligence | null | undefined): PriceIntelligence | null {
  if (!value || !Number.isFinite(value.lowestPrice30dCents)) return null;
  return value;
}
