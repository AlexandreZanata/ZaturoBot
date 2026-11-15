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
  if (offer.isDemoFixture) {
    return {
      enabled: false,
      href: null,
      rel: null,
      target: null,
      label: "Ver oferta na Shopee (desabilitado — demonstração)",
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
    label: "Ver oferta na Shopee",
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

/**
 * Filters and sorts catalog offers explicitly.
 * @param {Array<object>} offers
 * @param {object} options
 * @returns {Array<object>}
 */
export function filterAndSortOffers(
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
  const q = String(query || "").trim().toLowerCase();

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

    if (condition === "com-cupom" && !offer.couponCondition) return false;
    if (condition === "amostra-7-dias" && !verifySevenDaySample(offer).eligible) return false;
    if (condition === "pix-sem-cupom" && offer.couponCondition) return false;

    if (q) {
      const haystack = [
        offer.title,
        offer.exactVariant,
        offer.oneLineReason,
        offer.forWhomItMakesSense,
        ...(offer.tags || []),
      ]
        .join(" ")
        .toLowerCase();
      if (!haystack.includes(q)) return false;
    }
    return true;
  });

  const sorted = [...filtered];
  if (sort === "price_asc") {
    sorted.sort((a, b) => a.priceBrlCents - b.priceBrlCents);
  } else if (sort === "recent") {
    sorted.sort((a, b) => Date.parse(b.observedAt) - Date.parse(a.observedAt));
  } else {
    // Editorial relevance: approved first, then 7-day sample, then observedAt desc
    sorted.sort((a, b) => {
      if (a.state === "approved" && b.state !== "approved") return -1;
      if (a.state !== "approved" && b.state === "approved") return 1;
      return Date.parse(b.observedAt) - Date.parse(a.observedAt);
    });
  }
  return sorted;
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
