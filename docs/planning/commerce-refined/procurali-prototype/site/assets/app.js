"use strict";
(function () {
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
function formatBrl(cents) {
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
function contrastRatio(hexA, hexB) {
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
function isSafePublicAssetPath(value) {
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
function resolveMediaSrc(root, assetPath) {
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
function validateCatalogSnapshot(snapshot) {
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
function isOfferFresh(offer, referenceIso) {
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
function verifySevenDaySample(offer) {
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
function buildOfficialAffiliateAnchorProps(offer, mediaRightsList = []) {
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
function checkForbiddenClaims(text) {
  const normalized = String(text || "").toLowerCase();
  return FORBIDDEN_UNSUPPORTED_CLAIMS.filter((phrase) => normalized.includes(phrase));
}

/**
 * Filters and sorts catalog offers explicitly.
 * @param {Array<object>} offers
 * @param {object} options
 * @returns {Array<object>}
 */
function filterAndSortOffers(
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
    if (priceBand === "acima-de-300" && reais <= 300) return false;

    if (condition === "com-cupom" && !offer.couponCondition) return false;
    if (condition === "amostra-7-dias" && !verifySevenDaySample(offer).eligible) return false;
    if (condition === "pix-sem-cupom" && offer.couponCondition) return false;

    if (q) {
      const haystack = [
        offer.title,
        offer.exactVariant,
        offer.oneLineReason,
        offer.forWhomItMakesSense,
        offer.brand || "",
        offer.model || "",
        offer.productId || "",
        offer.sellerName || "",
        offer.merchant || "",
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
    // Editorial relevance: approved first, real affiliate offers before demo fixtures, then observedAt desc
    sorted.sort((a, b) => {
      if (a.state === "approved" && b.state !== "approved") return -1;
      if (a.state !== "approved" && b.state === "approved") return 1;
      if (!a.isDemoFixture && b.isDemoFixture) return -1;
      if (a.isDemoFixture && !b.isDemoFixture) return 1;
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
function runDeterministicFinder(
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
function formatTelegramHtmlOffer(offer, siteBaseUrl = "https://procurali.com.br") {
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
  const priceLine =
    refStr && offer.discountPercentLabel
      ? `💰 <b>${escapeTelegramHtml(priceStr)}</b> (<s>de ${escapeTelegramHtml(refStr)}</s> · <b>${escapeTelegramHtml(offer.discountPercentLabel)}</b>)`
      : refStr
        ? `💰 <b>${escapeTelegramHtml(priceStr)}</b> (<s>de ${escapeTelegramHtml(refStr)}</s>)`
        : `💰 <b>${escapeTelegramHtml(priceStr)}</b>`;

  const lines = [
    `<i>${escapeTelegramHtml("Publicidade · Link de afiliado: podemos receber comissão se você comprar.")}</i>`,
    "",
    `🔥 <b>${escapeTelegramHtml(truncateChars(offer.title, 160))}</b>`,
    `Marca: ${escapeTelegramHtml(offer.brand || "Mercado Livre")}${offer.productId ? ` · Cód. ML: <code>${escapeTelegramHtml(offer.productId)}</code>` : ""}`,
    priceLine,
    offer.shortCondition ? `💳 ${escapeTelegramHtml(truncateChars(offer.shortCondition, 120))}` : "",
    offer.sellerName ? `🏪 ${escapeTelegramHtml(truncateChars(offer.sellerName, 100))}` : "",
    offer.observedAtDisplay ? `🕐 Observado em ${escapeTelegramHtml(offer.observedAtDisplay)}` : "",
    "",
    offer.shareId ? `🔍 Buscador ML: <code>${escapeTelegramHtml(offer.shareId)}</code>` : "",
    offer.officialAffiliateUrl
      ? `🔗 <a href="${escapeTelegramHtml(offer.officialAffiliateUrl)}">Ver oferta oficial no Mercado Livre</a>`
      : "",
    siteOfferUrl ? `📋 <a href="${escapeTelegramHtml(siteOfferUrl)}">Ficha técnica e limitações no Procurali</a>` : "",
  ].filter((line, idx, arr) => !(line === "" && (idx === 0 || arr[idx - 1] === "")));

  return truncateChars(lines.join("\n"), 1024);
}

/**
 * Formats an approved offer as human-assisted WhatsApp Web copy (no Meta API or unofficial automation).
 * @param {object} offer
 * @param {string} [siteBaseUrl]
 * @returns {string}
 */
function formatWhatsappWebAssistedText(offer, siteBaseUrl = "https://procurali.com.br") {
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
function buildWhatsappWebAssistedPackage(
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

/* Procurali Progressive Enhancement DOM Controller (compiled inside IIFE by build.mjs) */

const STORAGE_SAVED_KEY = "procurali_saved_v1";
const STORAGE_CONSENT_KEY = "procurali_analytics_consent_v1";
const STORAGE_EVENTS_KEY = "procurali_demo_events_v1";

function safeReadJson(key, fallback) {
  try {
    const raw = window.localStorage.getItem(key);
    return raw ? JSON.parse(raw) : fallback;
  } catch {
    return fallback;
  }
}

function safeWriteJson(key, value) {
  try {
    window.localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage disabled or private mode; degrade silently without errors.
  }
}

function escapeHtml(value) {
  return String(value || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function getRootPrefix() {
  return document.documentElement.getAttribute("data-root") || "./";
}

function getCatalogData() {
  const node = document.getElementById("procurali-catalog-data");
  if (!node) return null;
  try {
    return JSON.parse(node.textContent || "{}");
  } catch {
    return null;
  }
}

/* --------------------------------------------------------------------------
 * Mobile navigation drawer
 * ------------------------------------------------------------------------ */
function initMobileDrawer() {
  const toggleBtn = document.querySelector("[data-menu-toggle]");
  const drawer = document.getElementById("mobile-drawer");
  if (!toggleBtn || !drawer) return;

  toggleBtn.addEventListener("click", () => {
    const isOpen = toggleBtn.getAttribute("aria-expanded") === "true";
    toggleBtn.setAttribute("aria-expanded", String(!isOpen));
    drawer.hidden = isOpen;
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !drawer.hidden) {
      drawer.hidden = true;
      toggleBtn.setAttribute("aria-expanded", "false");
      toggleBtn.focus();
    }
  });
}

/* --------------------------------------------------------------------------
 * Saved items drawer (opt-in local browser storage only)
 * ------------------------------------------------------------------------ */
function renderSavedDrawer() {
  const saved = safeReadJson(STORAGE_SAVED_KEY, []);
  const countEls = document.querySelectorAll("[data-saved-count]");
  countEls.forEach((el) => {
    el.textContent = String(saved.length);
    el.hidden = saved.length === 0;
  });

  const saveButtons = document.querySelectorAll("[data-save-offer]");
  saveButtons.forEach((btn) => {
    const slug = btn.getAttribute("data-save-offer");
    const isSaved = saved.some((item) => item.slug === slug);
    btn.setAttribute("aria-pressed", String(isSaved));
    if (btn.getAttribute("data-save-icon") === "true") {
      btn.classList.toggle("is-saved", isSaved);
      const title = btn.getAttribute("data-offer-title") || slug;
      btn.setAttribute(
        "aria-label",
        isSaved ? `Remover ${title} dos salvos` : `Salvar ${title} na lista`
      );
    } else {
      btn.textContent = isSaved ? "Salvo na sua lista" : "Salvar item";
    }
  });

  const listContainer = document.getElementById("saved-items-list");
  if (!listContainer) return;

  const root = getRootPrefix();
  if (saved.length === 0) {
    listContainer.innerHTML =
      '<p class="muted">Nenhum item salvo ainda. Use o botão "Salvar item" em qualquer oferta para comparar depois.</p>';
    return;
  }

  listContainer.innerHTML = saved
    .map(
      (item) => `
      <div class="card">
        <div>
          <span class="badge badge-neutral">Demonstração salva localmente</span>
        </div>
        <strong>${escapeHtml(item.title)}</strong>
        <span class="muted">${escapeHtml(item.priceLabel)}</span>
        <div class="btn-row">
          <a class="btn btn-secondary" href="${escapeHtml(root)}ofertas/${escapeHtml(item.slug)}/index.html">Abrir ficha</a>
          <button type="button" class="btn btn-secondary" data-remove-saved="${escapeHtml(item.slug)}">Remover</button>
        </div>
      </div>`
    )
    .join("");
}

function initSavedDrawer() {
  const drawer = document.getElementById("saved-drawer");
  const openBtns = document.querySelectorAll("[data-open-saved]");
  const closeBtn = document.querySelector("[data-close-saved]");
  const clearBtn = document.querySelector("[data-clear-saved]");

  let lastTrigger = null;

  function openDrawer(trigger) {
    if (!drawer) return;
    lastTrigger = trigger;
    renderSavedDrawer();
    drawer.hidden = false;
    if (closeBtn) closeBtn.focus();
  }

  function closeDrawer() {
    if (!drawer) return;
    drawer.hidden = true;
    if (lastTrigger && typeof lastTrigger.focus === "function") {
      lastTrigger.focus();
    }
  }

  openBtns.forEach((btn) => {
    btn.addEventListener("click", () => openDrawer(btn));
  });

  if (closeBtn) {
    closeBtn.addEventListener("click", closeDrawer);
  }

  if (drawer) {
    drawer.addEventListener("click", (event) => {
      if (event.target === drawer) closeDrawer();
    });
  }

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && drawer && !drawer.hidden) {
      closeDrawer();
    }
  });

  if (clearBtn) {
    clearBtn.addEventListener("click", () => {
      safeWriteJson(STORAGE_SAVED_KEY, []);
      renderSavedDrawer();
    });
  }

  document.addEventListener("click", (event) => {
    const saveBtn = event.target.closest("[data-save-offer]");
    if (saveBtn) {
      const slug = saveBtn.getAttribute("data-save-offer");
      const title = saveBtn.getAttribute("data-offer-title") || slug;
      const priceLabel = saveBtn.getAttribute("data-offer-price") || "";
      const current = safeReadJson(STORAGE_SAVED_KEY, []);
      const exists = current.some((i) => i.slug === slug);
      const next = exists
        ? current.filter((i) => i.slug !== slug)
        : [...current, { slug, title, priceLabel }];
      safeWriteJson(STORAGE_SAVED_KEY, next);
      renderSavedDrawer();
      return;
    }

    const removeBtn = event.target.closest("[data-remove-saved]");
    if (removeBtn) {
      const slug = removeBtn.getAttribute("data-remove-saved");
      const current = safeReadJson(STORAGE_SAVED_KEY, []);
      safeWriteJson(
        STORAGE_SAVED_KEY,
        current.filter((i) => i.slug !== slug)
      );
      renderSavedDrawer();
    }
  });

  renderSavedDrawer();
}

/* --------------------------------------------------------------------------
 * Privacy-first optional local analytics (off by default, never blocks links)
 * ------------------------------------------------------------------------ */
function recordLocalAnalyticsEvent(eventName, detail) {
  const consent = safeReadJson(STORAGE_CONSENT_KEY, { optionalAnalytics: false });
  if (!consent.optionalAnalytics) return;
  const events = safeReadJson(STORAGE_EVENTS_KEY, []);
  const entry = {
    event: eventName,
    detail: detail || "",
    at: new Date().toISOString(),
  };
  safeWriteJson(STORAGE_EVENTS_KEY, [entry, ...events].slice(0, 20));
  renderAnalyticsLog();
}

function renderAnalyticsLog() {
  const logEl = document.getElementById("analytics-demo-log");
  const statusEl = document.getElementById("analytics-consent-status");
  const consent = safeReadJson(STORAGE_CONSENT_KEY, { optionalAnalytics: false });
  if (statusEl) {
    statusEl.textContent = consent.optionalAnalytics
      ? "Medição local demonstrativa ATIVADA (nenhum dado sai do seu navegador)."
      : "Medição opcional DESATIVADA por padrão.";
  }
  if (!logEl) return;
  const events = safeReadJson(STORAGE_EVENTS_KEY, []);
  if (events.length === 0) {
    logEl.innerHTML =
      '<p class="muted">Nenhum evento registrado localmente ainda.</p>';
    return;
  }
  logEl.innerHTML = `<ul class="mono">${events
    .map(
      (ev) =>
        `<li>${escapeHtml(ev.at)} · <strong>${escapeHtml(ev.event)}</strong> (${escapeHtml(ev.detail)})</li>`
    )
    .join("")}</ul>`;
}

function initAnalyticsControls() {
  const toggle = document.getElementById("analytics-consent-toggle");
  const clearBtn = document.getElementById("analytics-clear-log");
  if (toggle) {
    const consent = safeReadJson(STORAGE_CONSENT_KEY, { optionalAnalytics: false });
    toggle.checked = Boolean(consent.optionalAnalytics);
    toggle.addEventListener("change", () => {
      safeWriteJson(STORAGE_CONSENT_KEY, { optionalAnalytics: toggle.checked });
      renderAnalyticsLog();
    });
  }
  if (clearBtn) {
    clearBtn.addEventListener("click", () => {
      safeWriteJson(STORAGE_EVENTS_KEY, []);
      renderAnalyticsLog();
    });
  }

  // Track voluntary outbound clicks asynchronously without ever preventing navigation
  document.addEventListener("click", (event) => {
    const tracked = event.target.closest("[data-analytics-event]");
    if (!tracked) return;
    const evName = tracked.getAttribute("data-analytics-event");
    const evDetail = tracked.getAttribute("data-analytics-detail") || "";
    recordLocalAnalyticsEvent(evName, evDetail);
  });

  renderAnalyticsLog();
}

/* --------------------------------------------------------------------------
 * Offers catalog interactive filter & sort (/ofertas/)
 * ------------------------------------------------------------------------ */
function offerStatusBadge(offer) {
  if (offer.state === "approved") {
    const sevenDay = verifySevenDaySample(offer);
    if (!offer.isDemoFixture) {
      return sevenDay.eligible
        ? `<span class="badge badge-emerald">✓ Ativa · ${escapeHtml(offer.marketplaceLabel)} · Série 7 dias</span>`
        : `<span class="badge badge-emerald">✓ Ativa · ${escapeHtml(offer.marketplaceLabel)}</span>`;
    }
    if (sevenDay.eligible) {
      return '<span class="badge badge-emerald">✓ Ativa (demo) · Série 7 dias</span>';
    }
    return '<span class="badge badge-petrol">✓ Ativa (demo)</span>';
  }
  if (offer.state === "stale") {
    return '<span class="badge badge-warning">⏳ Preço em atualização (&gt;30 min)</span>';
  }
  if (offer.state === "expired") {
    return '<span class="badge badge-neutral">⏱ Prazo encerrado (histórico)</span>';
  }
  if (offer.state === "missing_rights") {
    return '<span class="badge badge-danger">⛔ Bloqueada: direitos de mídia</span>';
  }
  return '<span class="badge badge-neutral">Indisponível</span>';
}

function initOffersFilter() {
  const form = document.getElementById("offers-filter-form");
  const resultsEl = document.getElementById("offers-results");
  const countEl = document.getElementById("offers-results-count");
  const resetBtn = document.getElementById("offers-reset-btn");
  const filterDetails = form ? form.querySelector("[data-filter-details]") : null;
  const catalog = getCatalogData();
  if (!form || !resultsEl || !catalog || !Array.isArray(catalog.offers)) return;

  if (
    filterDetails &&
    typeof window.matchMedia === "function" &&
    window.matchMedia("(max-width: 767px)").matches &&
    !window.location.search
  ) {
    filterDetails.open = false;
  }

  const root = getRootPrefix();
  const params = new URLSearchParams(window.location.search);

  const fields = {
    query: form.querySelector('[name="q"]'),
    category: form.querySelector('[name="categoria"]'),
    priceBand: form.querySelector('[name="faixa"]'),
    condition: form.querySelector('[name="condicao"]'),
    sort: form.querySelector('[name="ordem"]'),
    statusFilter: form.querySelector('[name="status"]'),
  };

  if (params.get("q") && fields.query) fields.query.value = params.get("q");
  if (params.get("categoria") && fields.category) fields.category.value = params.get("categoria");
  if (params.get("faixa") && fields.priceBand) fields.priceBand.value = params.get("faixa");
  if (params.get("condicao") && fields.condition) fields.condition.value = params.get("condicao");
  if (params.get("ordem") && fields.sort) fields.sort.value = params.get("ordem");
  if (params.get("status") && fields.statusFilter) fields.statusFilter.value = params.get("status");

  function renderOffers() {
    const state = {
      query: fields.query ? fields.query.value : "",
      category: fields.category ? fields.category.value : "todas",
      priceBand: fields.priceBand ? fields.priceBand.value : "todas",
      condition: fields.condition ? fields.condition.value : "todas",
      sort: fields.sort ? fields.sort.value : "relevance",
      statusFilter: fields.statusFilter ? fields.statusFilter.value : "ativas",
    };

    const nextParams = new URLSearchParams();
    if (state.query.trim()) nextParams.set("q", state.query.trim());
    if (state.category !== "todas") nextParams.set("categoria", state.category);
    if (state.priceBand !== "todas") nextParams.set("faixa", state.priceBand);
    if (state.condition !== "todas") nextParams.set("condicao", state.condition);
    if (state.sort !== "relevance") nextParams.set("ordem", state.sort);
    if (state.statusFilter !== "ativas") nextParams.set("status", state.statusFilter);

    try {
      const qs = nextParams.toString();
      const targetUrl = qs ? `${window.location.pathname}?${qs}` : window.location.pathname;
      window.history.replaceState(null, "", targetUrl);
    } catch {
      // Ignore under restricted origins.
    }

    const filtered = filterAndSortOffers(catalog.offers, state);
    const sortLabels = {
      relevance: "Relevância editorial",
      price_asc: "Menor preço informado (frete não incluído)",
      recent: "Observação mais recente",
    };

    if (countEl) {
      countEl.textContent = `${filtered.length} oferta(s) exibida(s) · Ordenação: ${sortLabels[state.sort] || "Relevância editorial"}.`;
    }

    if (filtered.length === 0) {
      resultsEl.innerHTML = `
        <div class="notice-box notice-box--warning">
          <h3>Nenhuma oferta encontrada com esses filtros</h3>
          <p>Nenhuma opção do catálogo demonstrativo corresponde a todos os critérios escolhidos (ou itens desatualizados/expirados estão ocultos pelo filtro atual).</p>
          <button type="button" class="btn btn-secondary" id="inline-reset-filters">Limpar filtros</button>
        </div>`;
      const inlineBtn = document.getElementById("inline-reset-filters");
      if (inlineBtn) {
        inlineBtn.addEventListener("click", resetFilters);
      }
      return;
    }

    resultsEl.innerHTML = filtered
      .map((offer) => {
        const price = formatBrl(offer.priceBrlCents);
        const refPrice = offer.referencePriceCents ? formatBrl(offer.referencePriceCents) : "";
        const savingsCents =
          offer.referencePriceCents && offer.referencePriceCents > offer.priceBrlCents
            ? offer.referencePriceCents - offer.priceBrlCents
            : 0;
        const sevenDay = verifySevenDaySample(offer);
        const imageSrc = resolveMediaSrc(root, offer.imagePath);
        const detailHref = `${root}ofertas/${offer.slug}/index.html`;
        const conditionText =
          offer.shortCondition ||
          (offer.couponCondition
            ? `Com condição de cupom · ${offer.paymentConditions}`
            : offer.paymentConditions);
        const ctaProps = buildOfficialAffiliateAnchorProps(offer, catalog.mediaRights || []);
        const isActionable = offer.state === "approved";
        const destLabel =
          ctaProps.enabled
            ? ctaProps.label
            : offer.ctaDestLabel ||
              (isActionable
                ? offer.marketplaceLabel === "Mercado Livre"
                  ? "Ver oferta no Mercado Livre"
                  : "Ver oferta na Shopee"
                : "Conferir status do item");
        const ctaClass = isActionable ? "btn btn-action" : "btn btn-secondary";

        return `
        <article class="offer-row">
          <a class="offer-thumb" href="${escapeHtml(detailHref)}" tabindex="-1" aria-hidden="true">
            <img src="${escapeHtml(imageSrc)}" alt="${escapeHtml(offer.imageAlt)}" width="640" height="480" loading="lazy">
          </a>
          <div class="offer-body">
            <div class="offer-meta-row">
              ${offerStatusBadge(offer)}
              <span class="offer-merchant">${escapeHtml(offer.merchant)}</span>
              ${offer.sponsored ? '<span class="badge badge-warning">Patrocinado no ML</span>' : ""}
            </div>
            <h2 class="offer-title"><a href="${escapeHtml(detailHref)}">${escapeHtml(offer.title)}</a></h2>
            <p class="offer-variant">${offer.brand && offer.model ? `<strong>${escapeHtml(offer.brand)} · ${escapeHtml(offer.model)}</strong> · ` : ""}Variante: ${escapeHtml(offer.exactVariant)}</p>
            <p class="offer-reason">${escapeHtml(offer.oneLineReason)}</p>
            ${
              offer.marketplaceRating
                ? `<p class="offer-rating-line"><strong>Anúncio ML:</strong> ${escapeHtml(offer.marketplaceRating)} ★${offer.marketplaceSalesLabel ? ` (${escapeHtml(offer.marketplaceSalesLabel)})` : ""}${offer.returnDays ? ` · Devolução grátis ${escapeHtml(offer.returnDays)}d` : ""}</p>`
                : ""
            }
            ${
              sevenDay.eligible
                ? `<p class="offer-flag"><svg class="icon" viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><path d="m5 12.5 4.5 4.5L19 7.5"/></svg><span>${escapeHtml(sevenDay.label)}</span></p>`
                : ""
            }
            ${
              offer.couponCode
                ? `<div class="offer-inline-coupon"><button type="button" class="coupon-pill" data-copy-code="${escapeHtml(offer.couponCode)}">Copiar cupom: <strong>${escapeHtml(offer.couponCode)}</strong></button></div>`
                : ""
            }
          </div>
          <div class="offer-aside">
            <div class="offer-price-block">
              ${
                refPrice
                  ? `<div class="offer-ref-row"><span class="price-old">De ${escapeHtml(refPrice)}</span>${offer.discountPercentLabel ? `<span class="discount-chip">${escapeHtml(offer.discountPercentLabel)}</span>` : ""}</div>`
                  : ""
              }
              <strong class="offer-price">${escapeHtml(price)}</strong>
              ${savingsCents > 0 && !offer.discountPercentLabel ? `<span class="savings-pill">Economia de ${escapeHtml(formatBrl(savingsCents))}</span>` : ""}
            </div>
            <span class="offer-cond">${escapeHtml(offer.installmentSummary || conditionText)}</span>
            ${offer.validUntilDisplay ? `<span class="deadline-pill">${escapeHtml(offer.validUntilDisplay)}</span>` : ""}
            <span class="offer-obs"><svg class="icon" viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false"><circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/></svg><span>Obs. ${escapeHtml(offer.observedAtDisplay)} · frete não incluído</span></span>
            <div class="offer-actions">
              ${
                ctaProps.enabled
                  ? `<a class="${ctaClass}" href="${escapeHtml(ctaProps.href)}" rel="${escapeHtml(ctaProps.rel)}" target="${escapeHtml(ctaProps.target)}" data-analytics-event="outbound_offer_click" data-analytics-detail="${escapeHtml(offer.slug)}">${escapeHtml(destLabel)}</a>
                     <a class="btn btn-secondary" href="${escapeHtml(detailHref)}">Ficha técnica</a>`
                  : `<a class="${ctaClass}" href="${escapeHtml(detailHref)}">${escapeHtml(destLabel)}</a>`
              }
              <button type="button" class="btn btn-secondary" data-save-offer="${escapeHtml(offer.slug)}" data-offer-title="${escapeHtml(offer.title)}" data-offer-price="${escapeHtml(price)}">Salvar</button>
            </div>
            ${ctaProps.enabled ? '<span class="offer-affiliate-note">Link de afiliado · podemos receber comissão.</span>' : ""}
          </div>
        </article>`;
      })
      .join("");

    renderSavedDrawer();
  }

  function resetFilters() {
    if (fields.query) fields.query.value = "";
    if (fields.category) fields.category.value = "todas";
    if (fields.priceBand) fields.priceBand.value = "todas";
    if (fields.condition) fields.condition.value = "todas";
    if (fields.sort) fields.sort.value = "relevance";
    if (fields.statusFilter) fields.statusFilter.value = "ativas";
    renderOffers();
  }

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    renderOffers();
  });
  form.addEventListener("change", renderOffers);
  if (fields.query) fields.query.addEventListener("input", renderOffers);
  if (resetBtn) resetBtn.addEventListener("click", resetFilters);

  renderOffers();
}

/* --------------------------------------------------------------------------
 * Guided AI Finder local demonstration (/encontre-com-ia/)
 * ------------------------------------------------------------------------ */
function initAiFinder() {
  const form = document.getElementById("ai-finder-form");
  const outEl = document.getElementById("ai-finder-output");
  const catalog = getCatalogData();
  if (!form || !outEl || !catalog || !Array.isArray(catalog.offers)) return;

  const root = getRootPrefix();

  function evaluateFinder() {
    const need = (form.querySelector('[name="need"]')?.value || "").trim();
    const maxBudgetBrl = Number(form.querySelector('[name="budget"]')?.value || 180);
    const requirement = form.querySelector('[name="requirement"]')?.value || "qualquer";
    const simulatedState = form.querySelector('[name="simulatedState"]')?.value || "normal";

    const result = runDeterministicFinder(
      { need, maxBudgetBrl, requirement, simulatedState },
      catalog.offers
    );

    recordLocalAnalyticsEvent("finder_demo_query", `status=${result.status}`);

    const bannerClass =
      result.status === "match"
        ? "notice-box"
        : result.status === "no-match"
        ? "notice-box notice-box--warning"
        : "notice-box notice-box--danger";

    let bodyHtml = `
      <div class="${bannerClass}">
        <p><span class="badge badge-petrol">${escapeHtml(result.modeLabel)}</span></p>
        <p><strong>${escapeHtml(result.explanation)}</strong></p>
        <p class="muted" style="margin-bottom:0;">Base consultada: ${escapeHtml(result.catalogTimestamp)}</p>
      </div>`;

    if (result.matches.length > 0) {
      bodyHtml += `<div class="finder-results-grid">` + result.matches
        .map(
          (m) => `
          <article class="card finder-card">
            <div><span class="badge badge-lime">✓ Compatível na demonstração</span></div>
            <h3 class="finder-card-title">${escapeHtml(m.title)}</h3>
            <p class="muted finder-card-variant">${escapeHtml(m.exactVariant)}</p>
            <p class="offer-price">${escapeHtml(m.priceFormatted)}</p>
            <p class="muted finder-card-obs">Observado em ${escapeHtml(m.observedAtDisplay)}</p>
            <p><strong>Por que encaixa:</strong> ${escapeHtml(m.fitReason)}</p>
            <p><strong>Limitação principal:</strong> ${escapeHtml(m.mainTradeoff)}</p>
            <p class="muted"><strong>O que não sabemos:</strong> ${escapeHtml(m.unknownItem)}</p>
            <div class="card-foot">
              <a class="btn btn-petrol btn-block" href="${escapeHtml(root)}ofertas/${escapeHtml(m.slug)}/index.html">Ver ficha e condições</a>
            </div>
          </article>`
        )
        .join("") + `</div>`;
    } else {
      bodyHtml += `
        <div class="btn-row">
          <a class="btn btn-petrol" href="${escapeHtml(root)}ofertas/index.html">Explorar ofertas com filtros normais</a>
          <a class="btn btn-secondary" href="${escapeHtml(root)}guias/index.html">Ler guias de compra</a>
        </div>`;
    }

    outEl.innerHTML = bodyHtml;
  }

  form.addEventListener("submit", (event) => {
    event.preventDefault();
    evaluateFinder();
  });

  document.querySelectorAll("[data-finder-preset]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const needInput = form.querySelector('[name="need"]');
      const budgetInput = form.querySelector('[name="budget"]');
      const reqSelect = form.querySelector('[name="requirement"]');
      const stateSelect = form.querySelector('[name="simulatedState"]');
      if (needInput) needInput.value = btn.getAttribute("data-need") || "";
      if (budgetInput) budgetInput.value = btn.getAttribute("data-budget") || "180";
      if (reqSelect) reqSelect.value = btn.getAttribute("data-req") || "qualquer";
      if (stateSelect) stateSelect.value = btn.getAttribute("data-state") || "normal";
      evaluateFinder();
    });
  });

  evaluateFinder();
}

/* --------------------------------------------------------------------------
 * Copy buttons on Creative Kit (/criativos/)
 * ------------------------------------------------------------------------ */
function initCopyButtons() {
  const statusEl = document.getElementById("copy-feedback");
  document.addEventListener("click", async (event) => {
    const codeBtn = event.target.closest("[data-copy-code]");
    if (codeBtn) {
      const code = codeBtn.getAttribute("data-copy-code") || "";
      if (!code) return;
      const origHtml = codeBtn.innerHTML;
      try {
        await navigator.clipboard.writeText(code);
        codeBtn.textContent = `Copiado: ${code}`;
      } catch {
        codeBtn.textContent = `Código: ${code}`;
      }
      setTimeout(() => {
        codeBtn.innerHTML = origHtml;
      }, 1800);
      return;
    }

    const btn = event.target.closest("[data-copy-target]");
    if (!btn) return;
    const id = btn.getAttribute("data-copy-target");
    const target = document.getElementById(id);
    if (!target) return;
    const text = target.textContent || "";
    try {
      await navigator.clipboard.writeText(text);
      if (statusEl) statusEl.textContent = "Texto copiado para a área de transferência.";
      const original = btn.textContent;
      btn.textContent = "Copiado";
      setTimeout(() => {
        btn.textContent = original;
      }, 1800);
    } catch {
      if (statusEl) {
        statusEl.textContent =
          "Cópia automática indisponível neste navegador; selecione e copie o bloco manualmente.";
      }
    }
  });
}

function initSearchFocusShortcut() {
  const headerSearchTrigger = document.querySelector("[data-focus-search]");
  const heroInput =
    document.getElementById("hero-search-input") ||
    document.getElementById("header-search-input") ||
    document.getElementById("opening-search-input");
  if (!headerSearchTrigger || !heroInput) return;

  headerSearchTrigger.addEventListener("click", (event) => {
    event.preventDefault();
    heroInput.focus();
    heroInput.scrollIntoView({ behavior: "smooth", block: "center" });
  });
}

function initOfferGallery() {
  const mainPhoto = document.querySelector("[data-gallery-main]");
  const buttons = Array.from(document.querySelectorAll("[data-gallery-btn]"));
  if (!mainPhoto || buttons.length === 0) return;

  buttons.forEach((btn) => {
    btn.addEventListener("click", () => {
      const nextSrc = btn.getAttribute("data-gallery-src");
      const nextAlt = btn.getAttribute("data-gallery-alt");
      if (nextSrc) mainPhoto.setAttribute("src", nextSrc);
      if (nextAlt) mainPhoto.setAttribute("alt", nextAlt);
      buttons.forEach((item) => {
        const active = item === btn;
        item.classList.toggle("is-active", active);
        item.setAttribute("aria-pressed", String(active));
      });
    });
  });
}

document.addEventListener("DOMContentLoaded", () => {
  initMobileDrawer();
  initSavedDrawer();
  initAnalyticsControls();
  initOffersFilter();
  initOfferGallery();
  initAiFinder();
  initCopyButtons();
  initSearchFocusShortcut();
});

})();
