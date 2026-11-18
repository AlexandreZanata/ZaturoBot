import type { Offer, MediaRight } from "./types";
export function formatBrl(cents: number): string;
export function contrastRatio(a: string, b: string): number;
export function isSafePublicAssetPath(value: string): boolean;
export function resolveMediaSrc(root: string, assetPath: string): string;
export function validateCatalogSnapshot(snapshot: unknown): { valid: boolean; errors: string[] };
export function isOfferFresh(offer: Offer, referenceIso: string): { fresh: boolean; ageMinutes: number };
export function verifySevenDaySample(offer: Offer): { eligible: boolean; minPriceCents: number | null; label: string | null };
export function buildOfficialAffiliateAnchorProps(
  offer: Offer,
  mediaRightsList?: MediaRight[]
): { enabled: boolean; href: string | null; rel: string | null; target: string | null; label: string; reason: string };
export function checkForbiddenClaims(text: string): string[];
export function filterAndSortOffers(offers: Offer[], state: Record<string, unknown>): Offer[];
export function searchOffers(
  offers: Offer[],
  state: Record<string, unknown>
): { results: Offer[]; approximate: boolean; tokens: string[] };
export function normalizeSearchText(text: string): string;
export function searchTokens(query: string): string[];
export function scoreOfferForQuery(offer: Offer, tokens: string[]): { matched: number; score: number };
export function offerKind(offer: Offer): string;
export type PriceComparison = { peers: number; cheapestCents: number; deltaCents: number; isCheapest: boolean };
export function comparePricesByKind(offers: Offer[]): Record<string, PriceComparison>;
export function priceStats(cents: number[]): { min: number; max: number; avg: number; count: number } | null;
export type FinderMatch = {
  slug: string;
  title: string;
  exactVariant: string;
  priceFormatted: string;
  observedAtDisplay: string;
  fitReason: string;
  mainTradeoff: string;
  unknownItem: string;
};
export function runDeterministicFinder(
  input: { need: string; maxBudgetBrl: number; requirement?: string; simulatedState?: string },
  offers: Offer[]
): {
  status: string;
  modeLabel: string;
  explanation: string;
  catalogTimestamp: string;
  matches: FinderMatch[];
};
export function formatTelegramHtmlOffer(offer: Offer, siteBaseUrl?: string): string;
export function formatWhatsappWebAssistedText(offer: Offer, siteBaseUrl?: string): string;
export function buildWhatsappWebAssistedPackage(offers: Offer[], generatedAt: string, siteBaseUrl?: string): unknown;
