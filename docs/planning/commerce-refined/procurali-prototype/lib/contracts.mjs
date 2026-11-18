/**
 * Procurali — Pure Contract Validators, Filtering & Deterministic Demo Finder
 * Zero external dependencies; runnable in both Node.js (SSG/tests) and modern browsers.
 */

const FORBIDDEN_UNSUPPORTED_CLAIMS = [
  "menor preço da internet",
  "mais barato da internet",
  "preço imbatível",
  "nunca esteve tão barato",
  "últimas unidades",
  "só hoje",
  "acaba em",
];

/**
 * Formats integer BRL cents as Brazilian currency string (e.g. 8990 -> "R$ 89,90").
 * @param {number} cents
 * @returns {string}
 */
export function formatBrl(cents) {
  const safeCents = Number.isFinite(cents) ? Math.round(cents) : 0;
  const reais = (safeCents / 100).toFixed(2).replace(".", ",");
  const [whole, dec] = reais.split(",");
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ".");
  return `R$ ${grouped},${dec}`;
}

/**
 * Computes relative WCAG contrast ratio between two hex colors (#RRGGBB).
 * @param {string} hexA
 * @param {string} hexB
 * @returns {number}
 */
export function contrastRatio(hexA, hexB) {
  const lum = (hex) => {
    const clean = hex.replace("#", "");
    const nums = [0, 2, 4].map((i) => parseInt(clean.slice(i, i + 2), 16) / 255);
    const linear = nums.map((c) =>
      c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4)
    );
    return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
  };
  const l1 = lum(hexA);
  const l2 = lum(hexB);
  const hi = Math.max(l1, l2);
  const lo = Math.min(l1, l2);
  return (hi + 0.05) / (lo + 0.05);
}

/**
 * Validates that an asset URL/path is either a safe local /brand/ path, an HTTPS MinIO/CDN URL,
 * or a local loopback test endpoint — rejecting filesystem leaks, file://, and javascript:.
 * @param {string} value
 * @returns {boolean}
 */
export function isSafePublicAssetPath(value) {
  if (typeof value !== "string") return false;
  const trimmed = value.trim();
  if (!trimmed || trimmed.length > 1024 || /[\s\0\\@]/.test(trimmed)) {
    return false;
  }
  const lower = trimmed.toLowerCase();
  if (
    lower.includes(".local/") ||
    lower.includes("/home/") ||
    lower.includes("file://") ||
    lower.includes("javascript:") ||
    lower.includes("matt_tool=") ||
    lower.includes("..")
  ) {
    return false;
  }
  if (trimmed.startsWith("https://")) {
    const rest = trimmed.slice("https://".length);
    const host = rest.split(/[/:]/)[0];
    return Boolean(host) && rest.includes("/");
  }
  if (trimmed.startsWith("http://")) {
    const rest = trimmed.slice("http://".length);
    return (rest.startsWith("127.0.0.1") || rest.startsWith("[::1]")) && rest.includes("/");
  }
  return trimmed.startsWith("/brand/") || trimmed.startsWith("brand/");
}

/**
 * Resolves a catalog image path for static HTML or browser DOM: preserves MinIO HTTP(S) URLs
 * verbatim and prefixes relative asset paths with route depth `root`.
 * @param {string} root
 * @param {string} assetPath
 * @returns {string}
 */
export function resolveMediaSrc(root, assetPath) {
  const value = String(assetPath || "").trim();
  if (!value) return "";
  if (value.startsWith("https://") || value.startsWith("http://")) {
    return value;
  }
  return `${root}${value.replace(/^\//, "")}`;
}

/**
 * Validates a Hub-emitted SiteCatalogSnapshot or merged runtime catalog before static generation.
 * @param {object} snapshot
 * @returns {{ valid: boolean, errors: Array<string> }}
 */
export function validateCatalogSnapshot(snapshot) {
  const errors = [];
  if (!snapshot || typeof snapshot !== "object") {
    return { valid: false, errors: ["snapshot must be a JSON object"] };
  }
  if (!Array.isArray(snapshot.offers) || snapshot.offers.length === 0) {
    errors.push("snapshot.offers must contain at least one offer");
  }
  if (!Array.isArray(snapshot.categories) || snapshot.categories.length === 0) {
    errors.push("snapshot.categories must contain at least one category");
  }
  if (!Array.isArray(snapshot.mediaRights)) {
    errors.push("snapshot.mediaRights must be an array");
  }
  if (!Array.isArray(snapshot.evidenceSources)) {
    errors.push("snapshot.evidenceSources must be an array");
  }

  for (const offer of snapshot.offers || []) {
    if (!offer || typeof offer !== "object") {
      errors.push("invalid offer entry");
      continue;
    }
    if (!offer.slug || typeof offer.slug !== "string") {
      errors.push(`offer ${offer.id || "?"} missing slug`);
    }
    if (!offer.title || typeof offer.title !== "string") {
      errors.push(`offer ${offer.slug || offer.id || "?"} missing title`);
    }
    if (!Number.isInteger(offer.priceBrlCents) || offer.priceBrlCents <= 0) {
      errors.push(`offer ${offer.slug} missing positive integer priceBrlCents`);
    }
    if (!isSafePublicAssetPath(offer.imagePath)) {
      errors.push(`offer ${offer.slug} has unsafe or invalid imagePath`);
    }
    for (const img of offer.galleryPaths || []) {
      if (!img || !isSafePublicAssetPath(img.path)) {
        errors.push(`offer ${offer.slug} has unsafe or invalid galleryPaths item`);
      }
    }
    if (!offer.isDemoFixture && offer.state === "approved") {
      if (
        typeof offer.officialAffiliateUrl !== "string" ||
        !offer.officialAffiliateUrl.startsWith("https://meli.la/")
      ) {
        errors.push(`live ML offer ${offer.slug} must preserve official https://meli.la/ link`);
      }
    }
    const serialized = JSON.stringify(offer).toLowerCase();
    if (serialized.includes(".local/") || serialized.includes("/home/") || serialized.includes("file://")) {
      errors.push(`offer ${offer.slug} leaked restricted filesystem path`);
    }
  }

  return { valid: errors.length === 0, errors };
}

/**
 * Checks whether an offer observation is still within its freshness window (default 30 min).
 * @param {object} offer
 * @param {string} referenceIso
 * @returns {{ fresh: boolean, ageMinutes: number }}
 */
export function isOfferFresh(offer, referenceIso) {
  const obsMs = Date.parse(offer.observedAt);
  const refMs = Date.parse(referenceIso);
  if (!Number.isFinite(obsMs) || !Number.isFinite(refMs)) {
    return { fresh: false, ageMinutes: Infinity };
  }
  const ageMinutes = Math.max(0, Math.round((refMs - obsMs) / 60000));
  const ttl = Number.isFinite(offer.freshnessTtlMinutes) ? offer.freshnessTtlMinutes : 30;
  const fresh = offer.state === "approved" && ageMinutes <= ttl;
  return { fresh, ageMinutes };
}

/**
 * Validates whether a scoped 7-day rolling minimum claim is permitted.
 * @param {object} offer
 * @returns {{ eligible: boolean, minPriceCents: number | null, label: string | null }}
 */
export function verifySevenDaySample(offer) {
  if (!offer || !offer.hasVerifiedSevenDaySample || !Array.isArray(offer.priceObservations)) {
    return { eligible: false, minPriceCents: null, label: null };
  }
  const obs = offer.priceObservations;
  if (obs.length < 7) {
    return { eligible: false, minPriceCents: null, label: null };
  }
  const ref = obs[0];
  const sameScope = obs.every(
    (entry) =>
      entry.variantId === ref.variantId &&
      entry.sellerName === ref.sellerName &&
      entry.paymentBasis === ref.paymentBasis
  );
  if (!sameScope) {
    return { eligible: false, minPriceCents: null, label: null };
  }
  const uniqueDays = new Set(obs.map((entry) => String(entry.observedAt).slice(0, 10)));
  if (uniqueDays.size < 7) {
    return { eligible: false, minPriceCents: null, label: null };
  }
  const minPriceCents = Math.min(...obs.map((entry) => entry.priceBrlCents));
  if (offer.priceBrlCents > minPriceCents) {
    return { eligible: false, minPriceCents, label: null };
  }
  return {
    eligible: true,
    minPriceCents,
    label: "Menor preço observado pelo Procurali nesta semana (amostra de 7 dias)",
  };
}

/**
 * Preserves exact approved official affiliate URL bytes and blocks demo/invalid purchases.
 * @param {object} offer
 * @param {Array<object>} mediaRightsList
 * @returns {{ enabled: boolean, href: string | null, rel: string | null, target: string | null, label: string, reason: string }}
 */
export function buildOfficialAffiliateAnchorProps(offer, mediaRightsList = []) {
  const rights = mediaRightsList.find((m) => m.id === offer.mediaRightsRef);
  const storeAction =
    offer.marketplaceLabel === "Mercado Livre"
      ? "Ver oferta no Mercado Livre"
      : "Ver oferta na Shopee";
  if (offer.isDemoFixture) {
    return {
      enabled: false,
      href: null,
      rel: null,
      target: null,
      label: `${storeAction} (desabilitado — demonstração)`,
      reason:
        "Item fictício de demonstração: não gera compra real nem utiliza link de afiliado inventado.",
    };
  }
  if (!rights || !rights.rightsVerified || offer.state === "missing_rights") {
    return {
      enabled: false,
      href: null,
      rel: null,
      target: null,
      label: "Oferta indisponível (direitos de mídia pendentes)",
      reason: "Direitos de imagem não confirmados bloqueiam o link comercial.",
    };
  }
  if (offer.state !== "approved") {
    return {
      enabled: false,
      href: null,
      rel: null,
      target: null,
      label: `Oferta indisponível (${offer.state})`,
      reason: "Oferta expirada, desatualizada ou aguardando aprovação editorial.",
    };
  }
  if (typeof offer.officialAffiliateUrl !== "string" || !offer.officialAffiliateUrl.startsWith("https://")) {
    return {
      enabled: false,
      href: null,
      rel: null,
      target: null,
      label: "Aguardando URL oficial aprovada",
      reason: "URL oficial ausente ou inválida.",
    };
  }
  // Preserve exact official URL bytes untouched (no shortener, no appended tracking params).
  return {
    enabled: true,
    href: offer.officialAffiliateUrl,
    rel: "sponsored noopener",
    target: "_blank",
    label: storeAction,
    reason: "URL oficial preservada integralmente com rel=\"sponsored noopener\".",
  };
}

/**
 * Detects prohibited unsupported marketing claims in public text.
 * @param {string} text
 * @returns {Array<string>}
 */
export function checkForbiddenClaims(text) {
  const normalized = String(text || "").toLowerCase();
  return FORBIDDEN_UNSUPPORTED_CLAIMS.filter((phrase) => normalized.includes(phrase));
}

const SEARCH_STOPWORDS = new Set([
  "a", "o", "as", "os", "um", "uma", "de", "da", "do", "das", "dos", "e", "em", "no", "na", "nos", "nas",
  "com", "sem", "para", "pra", "por", "ate", "que", "ou", "pol", "polegada", "polegadas",
]);

/** Lowercase, accent-free text with punctuation collapsed to spaces ("Mecânico" → "mecanico"). */
export function normalizeSearchText(text) {
  return String(text || "")
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

/** Meaningful query tokens: accent-free, without Portuguese stopwords or single letters. */
export function searchTokens(query) {
  return normalizeSearchText(query)
    .split(" ")
    .filter((t) => t.length > 1 && !SEARCH_STOPWORDS.has(t));
}

/**
 * Everyday Portuguese words mapped to the catalog vocabulary, so "fone", "tela" or
 * "armazenamento" still find headsets, monitors and SSDs.
 */
const SEARCH_SYNONYMS = {
  fone: ["headset", "fone"],
  fones: ["headset", "fone"],
  ouvido: ["headset"],
  headphone: ["headset"],
  tela: ["monitor"],
  display: ["monitor"],
  gamer: ["gamer", "gaming"],
  armazenamento: ["ssd", "armazenamento"],
  hd: ["ssd", "hd"],
  disco: ["ssd"],
  m2: ["m 2", "nvme"],
  periferico: ["periferico", "teclado", "headset", "mouse"],
  perifericos: ["periferico", "teclado", "headset", "mouse"],
  carregador: ["carregador", "gan"],
  luminaria: ["luminaria", "iluminacao"],
  luz: ["luminaria", "iluminacao"],
  cabo: ["cabo", "cabos"],
  tomada: ["tomada", "tomadas", "filtro"],
  adaptador: ["adaptador", "hub"],
  suporte: ["suporte"],
  escritorio: ["home office", "mesa"],
  ergonomico: ["ergonomia"],
  ergonomica: ["ergonomia"],
};

function tokenVariants(token) {
  const variants = [token, ...(SEARCH_SYNONYMS[token] || []).map((v) => v.replace(/ /g, ""))];
  // Simple plural folding: "teclados" → "teclado", "monitores" → "monitor".
  if (token.length > 4 && token.endsWith("es")) variants.push(token.slice(0, -2));
  if (token.length > 3 && token.endsWith("s")) variants.push(token.slice(0, -1));
  return variants;
}

function fieldMatches(field, token) {
  // `compact` lets "200 hz" match "200Hz" and "pcie 4" match "pcie-4".
  return tokenVariants(token).some((v) => field.words.some((w) => w.startsWith(v)) || field.compact.includes(v));
}

function searchFields(offer) {
  const weighted = [
    [3, [offer.title, offer.brand, offer.model]],
    [2, [offer.exactVariant, offer.productId, ...(offer.tags || [])]],
    [1, [offer.oneLineReason, offer.forWhomItMakesSense, offer.sellerName, offer.merchant, offer.categorySlug, offer.categoryName]],
  ];
  return weighted.map(([weight, parts]) => {
    const text = normalizeSearchText(parts.filter(Boolean).join(" "));
    return { weight, words: text.split(" "), compact: text.replace(/ /g, "") };
  });
}

/**
 * Scores an offer for the query tokens: matched token count and a weighted relevance score
 * (title/brand/model hits weigh more than tags, which weigh more than editorial text).
 */
export function scoreOfferForQuery(offer, tokens) {
  const fields = searchFields(offer);
  let matched = 0;
  let score = 0;
  for (const token of tokens) {
    const best = fields.reduce((acc, f) => (fieldMatches(f, token) ? Math.max(acc, f.weight) : acc), 0);
    if (best > 0) {
      matched += 1;
      score += best;
    }
  }
  return { matched, score };
}

/**
 * Filters and sorts catalog offers explicitly, and reports how the text query matched.
 * Every meaningful query word must match (accent- and plural-insensitive); when none of the
 * offers match every word, offers matching some of the words are returned as `approximate`.
 * @param {Array<object>} offers
 * @param {object} options
 * @returns {{results: Array<object>, approximate: boolean, tokens: Array<string>}}
 */
export function searchOffers(
  offers,
  {
    category = "todas",
    priceBand = "todas",
    condition = "todas",
    sort = "relevance",
    query = "",
    statusFilter = "ativas",
  } = {}
) {
  const tokens = searchTokens(query);

  const filtered = offers.filter((offer) => {
    if (statusFilter === "ativas" && offer.state !== "approved") {
      return false;
    }
    if (statusFilter === "nao-ativas" && offer.state === "approved") {
      return false;
    }
    if (category && category !== "todas" && offer.categorySlug !== category) {
      return false;
    }
    const reais = offer.priceBrlCents / 100;
    if (priceBand === "ate-100" && reais > 100) return false;
    if (priceBand === "100-a-150" && (reais < 100 || reais > 150)) return false;
    if (priceBand === "150-a-300" && (reais < 150 || reais > 300)) return false;
    if (priceBand === "acima-de-300" && reais <= 300) return false;

    if (condition === "com-cupom" && !offer.couponCondition) return false;
    if (condition === "amostra-7-dias" && !verifySevenDaySample(offer).eligible) return false;
    if (condition === "pix-sem-cupom" && offer.couponCondition) return false;
    return true;
  });

  let pool = filtered.map((offer) => ({ offer, matched: 0, score: 0 }));
  let approximate = false;
  if (tokens.length > 0) {
    const scored = filtered.map((offer) => ({ offer, ...scoreOfferForQuery(offer, tokens) }));
    const exact = scored.filter((s) => s.matched === tokens.length);
    if (exact.length > 0 || tokens.length === 1) {
      pool = exact;
    } else {
      pool = scored.filter((s) => s.matched > 0);
      approximate = pool.length > 0;
    }
  }

  const editorial = (a, b) => {
    if (a.state === "approved" && b.state !== "approved") return -1;
    if (a.state !== "approved" && b.state === "approved") return 1;
    if (!a.isDemoFixture && b.isDemoFixture) return -1;
    if (a.isDemoFixture && !b.isDemoFixture) return 1;
    return Date.parse(b.observedAt) - Date.parse(a.observedAt);
  };

  if (sort === "price_asc") {
    pool.sort((a, b) => a.offer.priceBrlCents - b.offer.priceBrlCents);
  } else if (sort === "recent") {
    pool.sort((a, b) => Date.parse(b.offer.observedAt) - Date.parse(a.offer.observedAt));
  } else {
    // Editorial relevance: best text match first, then approved, real before demo, newest first.
    pool.sort((a, b) => b.matched - a.matched || b.score - a.score || editorial(a.offer, b.offer));
  }
  return { results: pool.map((s) => s.offer), approximate, tokens };
}

/**
 * Filters and sorts catalog offers explicitly.
 * @param {Array<object>} offers
 * @param {object} options
 * @returns {Array<object>}
 */
export function filterAndSortOffers(offers, options = {}) {
  return searchOffers(offers, options).results;
}

/** Product kind used for like-for-like price comparison (first tag, e.g. "ssd", "monitor"). */
export function offerKind(offer) {
  return normalizeSearchText((offer.tags || [])[0] || offer.categorySlug || "");
}

/**
 * Compares the prices of approved offers within the same product kind.
 * Offers of different kinds (an SSD and a monitor) are never compared with each other.
 * @param {Array<object>} offers
 * @returns {Record<string, {peers: number, cheapestCents: number, deltaCents: number, isCheapest: boolean}>}
 */
export function comparePricesByKind(offers) {
  const groups = new Map();
  for (const offer of offers) {
    if (offer.state !== "approved" || !Number.isFinite(offer.priceBrlCents)) continue;
    const kind = offerKind(offer);
    if (!groups.has(kind)) groups.set(kind, []);
    groups.get(kind).push(offer);
  }
  const out = {};
  for (const group of groups.values()) {
    if (group.length < 2) continue;
    const cheapestCents = Math.min(...group.map((o) => o.priceBrlCents));
    for (const offer of group) {
      out[offer.slug] = {
        peers: group.length,
        cheapestCents,
        deltaCents: offer.priceBrlCents - cheapestCents,
        isCheapest: offer.priceBrlCents === cheapestCents,
      };
    }
  }
  return out;
}

/** Min/max/average of a list of prices in cents (ignores non-finite values). */
export function priceStats(cents) {
  const values = cents.filter((c) => Number.isFinite(c));
  if (values.length === 0) return null;
  const min = Math.min(...values);
  const max = Math.max(...values);
  const avg = Math.round(values.reduce((a, b) => a + b, 0) / values.length);
  return { min, max, avg, count: values.length };
}

/**
 * Runs local deterministic demo finder without calling any external AI model.
 * Explicitly labels output as "Demonstração — sem consulta ao vivo".
 * @param {object} input
 * @param {Array<object>} offers
 * @returns {object}
 */
export function runDeterministicFinder(
  { need = "", maxBudgetBrl = 180, requirement = "qualquer", simulatedState = "normal" },
  offers
) {
  const base = {
    modeLabel: "Demonstração — sem consulta ao vivo",
    catalogTimestamp: "15/11/2026 às 14:25 (horário de Cuiabá · catálogo fictício)",
    status: simulatedState,
    matches: [],
    explanation: "",
  };

  if (simulatedState === "source-stale") {
    return {
      ...base,
      explanation:
        "Catálogo temporariamente desatualizado (janela > 30 minutos). A recomendação com preço foi pausada para evitar indicar valores vencidos. Você pode consultar os guias de compra enquanto a fonte é revalidada.",
    };
  }
  if (simulatedState === "service-unavailable") {
    return {
      ...base,
      explanation:
        "O serviço de seleção assistida por IA não está conectado neste protótipo estático (e exige adaptador no servidor em produção). Utilize os filtros normais de ofertas ou os guias editoriais.",
    };
  }
  if (simulatedState === "budget-cap") {
    return {
      ...base,
      explanation:
        "Teto mensal de IA atingido (US$ 20/mês combinados entre DeepSeek Flash 4.1 e Muse Spark 1.3 contributor, com Extra Usage desligado). Novas consultas ao modelo permanecem pausadas até o próximo ciclo ou reconciliação; o catálogo estático continua disponível.",
    };
  }

  const budgetCents = Math.max(0, Math.round(Number(maxBudgetBrl || 0) * 100));
  const queryTerms = String(need || "")
    .toLowerCase()
    .split(/\s+/)
    .filter((w) => w.length >= 3);

  const eligible = offers.filter((offer) => {
    if (offer.state !== "approved") return false;
    if (budgetCents > 0 && offer.priceBrlCents > budgetCents) return false;
    if (requirement === "ergonomia" && offer.categorySlug !== "ergonomia-e-suportes") return false;
    if (requirement === "energia" && offer.categorySlug !== "energia-e-cabos") return false;
    if (requirement === "iluminacao" && offer.categorySlug !== "iluminacao-de-bancada") return false;
    if (requirement === "sem-cupom" && Boolean(offer.couponCondition)) return false;
    return true;
  });

  const scored = eligible
    .map((offer) => {
      const text = [
        offer.title,
        offer.exactVariant,
        offer.oneLineReason,
        offer.forWhomItMakesSense,
        ...(offer.tags || []),
      ]
        .join(" ")
        .toLowerCase();
      let score = queryTerms.length === 0 ? 1 : 0;
      for (const term of queryTerms) {
        if (text.includes(term)) score += 2;
      }
      if (verifySevenDaySample(offer).eligible) score += 0.5;
      return { offer, score };
    })
    .filter((entry) => entry.score > 0)
    .sort((a, b) => b.score - a.score || a.offer.priceBrlCents - b.offer.priceBrlCents)
    .slice(0, 3);

  if (scored.length === 0) {
    return {
      ...base,
      status: "no-match",
      explanation:
        "Nenhuma opção ativa do catálogo demonstrativo atende simultaneamente ao orçamento e aos critérios informados. Tente ampliar o teto ou consultar o catálogo completo sem filtros restritivos.",
    };
  }

  return {
    ...base,
    status: "match",
    explanation: `Exibindo até ${scored.length} opção(ões) do catálogo fictício de demonstração compatíveis com teto de ${formatBrl(budgetCents)}.`,
    matches: scored.map(({ offer }) => ({
      slug: offer.slug,
      title: offer.title,
      exactVariant: offer.exactVariant,
      priceFormatted: formatBrl(offer.priceBrlCents),
      observedAtDisplay: offer.observedAtDisplay,
      fitReason: offer.forWhomItMakesSense,
      mainTradeoff: offer.limitations[0],
      unknownItem: offer.unknowns[0],
    })),
  };
}

function escapeTelegramHtml(text) {
  return String(text || "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function truncateChars(text, maxChars) {
  const s = String(text || "").trim();
  if (s.length <= maxChars) return s;
  return `${s.slice(0, Math.max(0, maxChars - 3))}...`;
}

function sanitizeWhatsappInline(text) {
  return String(text || "")
    .replace(/\s+/g, " ")
    .replace(/[*_~`]/g, "")
    .trim();
}

/**
 * Formats an approved live offer for Telegram Bot API (`parse_mode: "HTML"`, <= 1024 chars caption).
 * @param {object} offer
 * @param {string} [siteBaseUrl]
 * @returns {string}
 */
export function formatTelegramHtmlOffer(offer, siteBaseUrl = "https://procurali.com.br") {
  const cleanSite = String(siteBaseUrl || "").replace(/\/+$/, "");
  const siteOfferUrl =
    cleanSite.startsWith("https://") || cleanSite.startsWith("http://")
      ? `${cleanSite}/ofertas/${offer.slug}/index.html`
      : null;
  const priceStr = formatBrl(offer.priceBrlCents);
  const hasLowerPrice =
    Number.isFinite(offer.referencePriceCents) && offer.referencePriceCents > offer.priceBrlCents;
  const priceLines = [];
  if (hasLowerPrice) {
    const refStr = formatBrl(offer.referencePriceCents);
    const savingsCents = offer.referencePriceCents - offer.priceBrlCents;
    const savingsStr = formatBrl(savingsCents);
    const computedPct = Math.round((savingsCents / offer.referencePriceCents) * 100);
    const badge = String(offer.discountPercentLabel || `${computedPct}% OFF`).trim();
    priceLines.push(`❌ De: <s>${escapeTelegramHtml(refStr)}</s>`);
    priceLines.push(
      `✅ <b>POR APENAS: ${escapeTelegramHtml(priceStr)}</b> 🔥 <b>(${escapeTelegramHtml(badge)} · Economia de ${escapeTelegramHtml(savingsStr)}!)</b>`
    );
  } else if (offer.discountPercentLabel) {
    priceLines.push(
      `✅ <b>PREÇO EM OFERTA: ${escapeTelegramHtml(priceStr)}</b> 🔥 <b>(${escapeTelegramHtml(offer.discountPercentLabel)})</b>`
    );
  } else {
    priceLines.push(`✅ <b>PREÇO EM OFERTA: ${escapeTelegramHtml(priceStr)}</b> 🔥`);
  }

  const lines = [
    `🚨 <b>OFERTA RELÂMPAGO · ACHADO NO MERCADO LIVRE!</b> 🔥`,
    "",
    `📦 <b>${escapeTelegramHtml(truncateChars(offer.title, 160))}</b>`,
    `🏷️ <b>Marca:</b> ${escapeTelegramHtml(offer.brand || "Mercado Livre")}${offer.productId ? ` · <b>Cód. ML:</b> <code>${escapeTelegramHtml(offer.productId)}</code>` : ""}`,
    "",
    ...priceLines,
    offer.shortCondition ? `💳 ${escapeTelegramHtml(truncateChars(offer.shortCondition, 120))}` : "",
    offer.sellerName ? `🏪 ${escapeTelegramHtml(truncateChars(offer.sellerName, 100))}` : "",
    offer.observedAtDisplay ? `🕐 Atualizado em ${escapeTelegramHtml(offer.observedAtDisplay)}` : "",
    "",
    `⏳ <b>CORRA! Preço e estoque promocional sujeitos a alteração a qualquer momento!</b>`,
    offer.officialAffiliateUrl
      ? `👉 <b>GARANTA O SEU AQUI:</b> <a href="${escapeTelegramHtml(offer.officialAffiliateUrl)}">Clique para abrir a oferta oficial no Mercado Livre</a>`
      : "",
    offer.shareId ? `🔍 <b>Código rápido no app ML:</b> <code>${escapeTelegramHtml(offer.shareId)}</code>` : "",
    siteOfferUrl ? `📋 <a href="${escapeTelegramHtml(siteOfferUrl)}">Ver ficha técnica e histórico no Procurali</a>` : "",
    `<i>${escapeTelegramHtml("Publicidade · Link de afiliado: podemos receber comissão se você comprar.")}</i>`,
  ].filter((line, idx, arr) => !(line === "" && (idx === 0 || arr[idx - 1] === "")));

  return truncateChars(lines.join("\n"), 1024);
}

/**
 * Formats an approved offer as human-assisted WhatsApp Web copy (no Meta API or unofficial automation).
 * @param {object} offer
 * @param {string} [siteBaseUrl]
 * @returns {string}
 */
export function formatWhatsappWebAssistedText(offer, siteBaseUrl = "https://procurali.com.br") {
  const cleanSite = String(siteBaseUrl || "").replace(/\/+$/, "");
  const siteOfferUrl =
    cleanSite.startsWith("https://") || cleanSite.startsWith("http://")
      ? `${cleanSite}/ofertas/${offer.slug}/index.html`
      : null;
  const priceStr = formatBrl(offer.priceBrlCents);
  const refStr =
    offer.referencePriceCents && offer.referencePriceCents > offer.priceBrlCents
      ? formatBrl(offer.referencePriceCents)
      : null;
  const badge = offer.discountPercentLabel ? ` · *${sanitizeWhatsappInline(offer.discountPercentLabel)}*` : "";
  const priceLine = refStr
    ? `💰 *${priceStr}* (~De ${refStr}~${badge})`
    : `💰 *${priceStr}*${badge ? ` (${badge.replace(/^ · /, "")})` : ""}`;

  const lines = [
    "_Publicidade · Link de afiliado: podemos receber comissão se você comprar._",
    "",
    `🔥 *${sanitizeWhatsappInline(offer.title)}*`,
    `Marca: *${sanitizeWhatsappInline(offer.brand || "Mercado Livre")}*`,
    priceLine,
    offer.shortCondition ? `💳 ${sanitizeWhatsappInline(offer.shortCondition)}` : null,
    offer.sellerName ? `🏪 ${sanitizeWhatsappInline(offer.sellerName)}` : null,
    Array.isArray(offer.limitations) && offer.limitations[0]
      ? `⚠️ Atenção: ${sanitizeWhatsappInline(offer.limitations[0])}`
      : null,
    offer.observedAtDisplay ? `_Observado em ${sanitizeWhatsappInline(offer.observedAtDisplay)}_` : null,
    "",
    offer.shareId ? `🔎 Código ML: \`\`\`${sanitizeWhatsappInline(offer.shareId)}\`\`\`` : null,
    offer.officialAffiliateUrl ? `🔗 Oferta oficial: ${offer.officialAffiliateUrl}` : null,
    siteOfferUrl ? `📋 Ficha completa no Procurali: ${siteOfferUrl}` : null,
  ].filter((item) => item !== null);

  return lines.join("\n");
}

/**
 * Builds the site-emitted WhatsApp Web human-assisted draft package for active affiliate offers.
 * @param {Array<object>} offers
 * @param {string} generatedAtUtc
 * @param {string} [siteBaseUrl]
 * @returns {object}
 */
export function buildWhatsappWebAssistedPackage(
  offers,
  generatedAtUtc,
  siteBaseUrl = "https://procurali.com.br"
) {
  const cleanSite = String(siteBaseUrl || "").replace(/\/+$/, "");
  const liveOffers = (offers || []).filter((o) => !o.isDemoFixture && o.state === "approved");
  const drafts = liveOffers.map((offer) => ({
    id: offer.id,
    productId: offer.productId || offer.slug,
    slug: offer.slug,
    title: offer.title,
    priceBrlCents: offer.priceBrlCents,
    referencePriceCents: offer.referencePriceCents ?? null,
    discountLabel: offer.discountPercentLabel ?? null,
    shareLink: offer.officialAffiliateUrl,
    shareId: offer.shareId ?? null,
    imagePublicUrl: offer.imagePath,
    siteOfferUrl: `${cleanSite}/ofertas/${offer.slug}/index.html`,
    whatsappText: formatWhatsappWebAssistedText(offer, cleanSite),
    operatorChecklist: [
      "Envio humano assistido no WhatsApp Web (proibido usar API da Meta ou automação no WhatsApp Web).",
      `Conferir se preço (${formatBrl(offer.priceBrlCents)}) e estoque no link oficial ainda estão ativos (< 30 min).`,
      "Colar manualmente no grupo autorizado após validar a pré-visualização Open Graph do link.",
    ],
  }));

  const combinedDigestText =
    drafts.length === 0
      ? "_Procurali · Resumo manual WhatsApp Web_\nNenhuma oferta aprovada disponível no momento."
      : [
          "*Seleção Procurali · Achados verificados*",
          "_Publicidade · Link de afiliado: podemos receber comissão se você comprar._",
          "",
          ...drafts.flatMap((d, idx) => [
            `${idx + 1}. *${sanitizeWhatsappInline(truncateChars(d.title, 90))}*`,
            `   💰 *${formatBrl(d.priceBrlCents)}* · 🔗 ${d.shareLink}`,
            `   📋 Ficha: ${d.siteOfferUrl}`,
          ]),
          "",
          "_Revisão humana obrigatória no WhatsApp Web antes de enviar (sem API da Meta)._",
        ].join("\n");

  return {
    generatedAt: generatedAtUtc,
    mode: "whatsapp_web_human_assisted_no_meta_api",
    channelId: "@procurali",
    draftCount: drafts.length,
    combinedDigestText,
    drafts,
  };
}
