/**
 * Procurali — Page Content Renderers (pt-BR)
 * Every public page works with semantic HTML without requiring JavaScript.
 */

import {
  AI_BUDGET_POLICY,
  CATEGORIES,
  COMMUNITIES,
  CREATIVE_ASSETS,
  EDITORIAL_APPROVALS,
  EVIDENCE_SOURCES,
  GUIDE_VISUALS,
  GUIDES,
  HOME_SHOWCASE_META,
  MEDIA_RIGHTS,
  OFFERS,
  SHORT_VIDEO_STORYBOARD,
  SITE_META,
  TELEGRAM_POST_TEMPLATE,
  WEEKLY_SELECTION,
  WHATSAPP_MANUAL_DIGEST_TEMPLATE,
} from "../data/demo-catalog.mjs";
import {
  buildOfficialAffiliateAnchorProps,
  formatBrl,
  isOfferFresh,
  verifySevenDaySample,
} from "./validate-contracts.mjs";
import { CATEGORY_ICONS, icon } from "./icons.mjs";

export function esc(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function statusBadgeHtml(offer) {
  if (offer.state === "approved") {
    const sevenDay = verifySevenDaySample(offer);
    if (!offer.isDemoFixture) {
      return sevenDay.eligible
        ? `<span class="badge badge-emerald">✓ Ativa · ${esc(offer.marketplaceLabel)} · Série 7 dias</span>`
        : `<span class="badge badge-emerald">✓ Ativa · ${esc(offer.marketplaceLabel)}</span>`;
    }
    return sevenDay.eligible
      ? '<span class="badge badge-emerald">✓ Ativa (demo) · Série 7 dias</span>'
      : '<span class="badge badge-petrol">✓ Ativa (demo)</span>';
  }
  if (offer.state === "stale") {
    return '<span class="badge badge-warning">⏳ Preço em atualização (&gt;30 min)</span>';
  }
  if (offer.state === "expired") {
    return '<span class="badge badge-neutral">⏱ Prazo encerrado (histórico)</span>';
  }
  if (offer.state === "missing_rights") {
    return '<span class="badge badge-danger">⛔ Direitos de mídia pendentes</span>';
  }
  return '<span class="badge badge-neutral">Indisponível</span>';
}

function renderMultilineParagraphs(text) {
  return String(text || "")
    .split(/\n{2,}/)
    .map((block) => block.trim())
    .filter(Boolean)
    .map((block) => `<p>${esc(block).replace(/\n/g, "<br>")}</p>`)
    .join("");
}

function breadcrumbsHtml(root, items) {
  const parts = items.map((item, idx) => {
    const isLast = idx === items.length - 1;
    if (isLast || !item.href) {
      return `<span aria-current="page">${esc(item.label)}</span>`;
    }
    return `<a href="${esc(root + item.href)}">${esc(item.label)}</a>`;
  });
  return `<nav class="breadcrumbs" aria-label="Caminho de navegação">${parts.join(
    ' <span aria-hidden="true">/</span> '
  )}</nav>`;
}

/**
 * Shared offer row used by Home, Offers and Category pages. The browser bundle
 * (assets/app.js) mirrors this exact structure when re-rendering filtered results.
 */
function offerRowHtml(root, offer, { compact = false, headingTag = "h3" } = {}) {
  const sevenDay = verifySevenDaySample(offer);
  const price = formatBrl(offer.priceBrlCents);
  const refPrice = offer.referencePriceCents ? formatBrl(offer.referencePriceCents) : "";
  const savingsCents =
    offer.referencePriceCents && offer.referencePriceCents > offer.priceBrlCents
      ? offer.referencePriceCents - offer.priceBrlCents
      : 0;
  const detailHref = `${root}ofertas/${offer.slug}/index.html`;
  const conditionText =
    offer.shortCondition ||
    (offer.couponCondition
      ? `Com condição de cupom · ${offer.paymentConditions}`
      : offer.paymentConditions);
  const ctaProps = buildOfficialAffiliateAnchorProps(offer, MEDIA_RIGHTS);
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
  <article class="offer-row${compact ? " offer-row--compact" : ""}">
    <a class="offer-thumb" href="${esc(detailHref)}" tabindex="-1" aria-hidden="true">
      <img src="${esc(root + offer.imagePath.replace(/^\//, ""))}" alt="${esc(offer.imageAlt)}" width="640" height="480" loading="lazy">
    </a>
    <div class="offer-body">
      <div class="offer-meta-row">
        ${statusBadgeHtml(offer)}
        <span class="offer-merchant">${esc(offer.merchant)}</span>
        ${offer.sponsored ? '<span class="badge badge-warning">Patrocinado no ML</span>' : ""}
      </div>
      <${headingTag} class="offer-title"><a href="${esc(detailHref)}">${esc(offer.title)}</a></${headingTag}>
      <p class="offer-variant">
        ${offer.brand && offer.model ? `<strong>${esc(offer.brand)} · ${esc(offer.model)}</strong> · ` : ""}Variante: ${esc(offer.exactVariant)}
      </p>
      <p class="offer-reason">${esc(offer.oneLineReason)}</p>
      ${
        offer.marketplaceRating
          ? `<p class="offer-rating-line"><strong>Anúncio ML:</strong> ${esc(offer.marketplaceRating)} ★${offer.marketplaceSalesLabel ? ` (${esc(offer.marketplaceSalesLabel)})` : ""}${offer.returnDays ? ` · Devolução grátis ${esc(offer.returnDays)}d` : ""}</p>`
          : ""
      }
      ${sevenDay.eligible ? `<p class="offer-flag">${icon("check")}<span>${esc(sevenDay.label)}</span></p>` : ""}
      ${
        offer.couponCode
          ? `<div class="offer-inline-coupon"><button type="button" class="coupon-pill" data-copy-code="${esc(offer.couponCode)}">Copiar cupom: <strong>${esc(offer.couponCode)}</strong></button></div>`
          : ""
      }
    </div>
    <div class="offer-aside">
      <div class="offer-price-block">
        ${
          refPrice
            ? `<div class="offer-ref-row"><span class="price-old">De ${esc(refPrice)}</span>${offer.discountPercentLabel ? `<span class="discount-chip">${esc(offer.discountPercentLabel)}</span>` : ""}</div>`
            : ""
        }
        <strong class="offer-price">${esc(price)}</strong>
        ${savingsCents > 0 && !offer.discountPercentLabel ? `<span class="savings-pill">Economia de ${esc(formatBrl(savingsCents))}</span>` : ""}
      </div>
      <span class="offer-cond">${esc(offer.installmentSummary || conditionText)}</span>
      ${offer.validUntilDisplay ? `<span class="deadline-pill">${esc(offer.validUntilDisplay)}</span>` : ""}
      <span class="offer-obs">${icon("clock")}<span>Obs. ${esc(offer.observedAtDisplay)} · frete não incluído</span></span>
      ${
        compact
          ? ""
          : `<div class="offer-actions">
        ${
          ctaProps.enabled
            ? `<a class="${ctaClass}" href="${esc(ctaProps.href)}" rel="${esc(ctaProps.rel)}" target="${esc(ctaProps.target)}" data-analytics-event="outbound_offer_click" data-analytics-detail="${esc(offer.slug)}">${esc(destLabel)}</a>
               <a class="btn btn-secondary" href="${esc(detailHref)}">Ficha técnica</a>`
            : `<a class="${ctaClass}" href="${esc(detailHref)}">${esc(destLabel)}</a>`
        }
        <button type="button" class="btn btn-secondary" data-save-offer="${esc(offer.slug)}" data-offer-title="${esc(offer.title)}" data-offer-price="${esc(price)}">Salvar</button>
      </div>
      ${ctaProps.enabled ? '<span class="offer-affiliate-note">Link de afiliado · podemos receber comissão.</span>' : ""}`
      }
    </div>
  </article>`;
}

function renderVerticalOfferCard(root, card) {
  const detailHref = `${root}ofertas/${card.slug}/index.html`;
  const price = formatBrl(card.priceCents);
  const oldPrice = card.oldPriceCents ? formatBrl(card.oldPriceCents) : "";
  const isMercadoLivre = card.marketplaceLabel === "Mercado Livre";
  const storeToneClass = isMercadoLivre ? "store-chip--ml" : "store-chip--shopee";
  const toneClass =
    card.badgeTone === "warning"
      ? "v-badge--warning"
      : card.badgeTone === "neutral"
      ? "v-badge--neutral"
      : card.badgeTone === "danger"
      ? "v-badge--danger"
      : "v-badge--emerald";
  const cardStateClass = card.isStale
    ? " v-offer-card--stale"
    : card.isFeaturedCard
    ? " v-offer-card--featured"
    : "";
  const ctaClass = card.isStale ? "btn-offer-card btn-offer-card--neutral" : "btn-offer-card";
  const hasLiveAffiliateUrl =
    !card.isDemoFixture &&
    !card.isStale &&
    typeof card.officialAffiliateUrl === "string" &&
    card.officialAffiliateUrl.startsWith("https://");

  return `
  <article class="v-offer-card${cardStateClass}">
    <div class="v-offer-top">
      <div class="v-offer-top-badges">
        <span class="store-chip ${storeToneClass}">${icon("store", "store-icon")}<span>${esc(card.marketplaceLabel || "Shopee")}</span></span>
        <span class="v-badge ${toneClass}">${esc(card.topBadge)}</span>
        ${card.sponsored ? '<span class="v-badge v-badge--warning">Patrocinado</span>' : ""}
      </div>
      <button
        type="button"
        class="card-heart-btn"
        data-save-offer="${esc(card.slug)}"
        data-save-icon="true"
        data-offer-title="${esc(card.shortTitle)}"
        data-offer-price="${esc(price)}"
        aria-label="Salvar ${esc(card.shortTitle)} na lista">
        ${icon("heart", "heart-icon")}
      </button>
    </div>

    <a class="v-offer-media" href="${esc(detailHref)}" tabindex="-1" aria-hidden="true">
      <img src="${esc(root + card.imagePath.replace(/^\//, ""))}" alt="${esc(card.imageAlt)}" width="560" height="340" loading="lazy">
    </a>

    <div class="v-offer-body">
      ${
        card.brandModelLine || card.ratingText
          ? `<div class="v-brand-row">
              ${card.brandModelLine ? `<span class="v-brand-model">${esc(card.brandModelLine)}</span>` : ""}
              ${card.ratingText ? `<span class="v-rating-pill" title="Avaliação no anúncio do Mercado Livre">${icon("star", "v-star-icon")}<span>${esc(card.ratingText)}${card.salesLabel ? ` · ${esc(card.salesLabel)}` : ""}</span></span>` : ""}
            </div>`
          : ""
      }
      <h3 class="v-offer-title"><a href="${esc(detailHref)}">${esc(card.shortTitle)}</a></h3>
      <p class="v-offer-use">${esc(card.shortUse || "")}</p>

      <div class="v-offer-price-block">
        <div class="v-offer-price-inline">
          <strong class="v-offer-price">${esc(price)}</strong>
          ${oldPrice ? `<span class="v-offer-old-price">De ${esc(oldPrice)}</span>` : ""}
        </div>
        <p class="v-offer-condition">${esc(card.installmentSummary || card.shortCondition || card.storeLabel)}</p>
        <div class="v-offer-signals">
          ${
            card.mintText || card.couponCode
              ? `<div class="v-signals-inline">
                  ${card.mintText ? `<span class="v-mint-pill">${esc(card.mintText)}</span>` : ""}
                  ${
                    card.couponCode
                      ? `<button type="button" class="v-coupon-btn" data-copy-code="${esc(card.couponCode)}">Copiar: <strong>${esc(card.couponCode)}</strong></button>`
                      : ""
                  }
                </div>`
              : ""
          }
          ${
            card.deadlineText
              ? `<span class="v-deadline-pill${card.isStale ? " v-deadline-pill--stale" : ""}">${esc(card.deadlineText)}</span>`
              : ""
          }
        </div>
      </div>

      <div class="v-offer-foot">
        <span class="v-offer-checked">${icon("clock")}<span>${esc(card.checkedLabel || "Conferido hoje")} · ${esc(card.storeLabel)}</span></span>
        ${
          hasLiveAffiliateUrl
            ? `<a class="${ctaClass}" href="${esc(card.officialAffiliateUrl)}" rel="sponsored noopener" target="_blank" data-analytics-event="outbound_offer_click" data-analytics-detail="${esc(card.slug)}">
                <span>${esc(card.ctaLabel || "Ver oferta no Mercado Livre")}</span>
                ${icon("external")}
              </a>
              <div class="v-card-subfoot">
                <span class="v-affiliate-note">Link de afiliado</span>
                <a class="v-card-secondary-link" href="${esc(detailHref)}">Ficha técnica e fotos →</a>
              </div>`
            : `<a class="${ctaClass}" href="${esc(detailHref)}">
                <span>${esc(card.ctaLabel || "Ver oferta")}</span>
                ${card.isStale ? icon("arrowRight") : icon("external")}
              </a>`
        }
      </div>
    </div>
  </article>`;
}

export function renderHomePage(root) {
  const dailyCards = HOME_SHOWCASE_META.dailyCards || [];

  return `
  <section class="catalog-opening">
    <div class="container">
      <h1 class="sr-only">Encontre o que vale a pena para você. Ofertas e guias para escolher com mais contexto.</h1>

      <form class="opening-search-form" action="${esc(root)}ofertas/index.html" method="get" role="search" aria-label="Buscar ofertas no catálogo">
        <label class="sr-only" for="opening-search-input">Busque por produto, categoria ou oferta</label>
        <span class="opening-search-icon" aria-hidden="true">${icon("search")}</span>
        <input id="opening-search-input" class="opening-search-input" type="search" name="q" placeholder="Buscar SSD, teclado, headset, monitor, suporte...">
        <button type="submit" class="btn btn-lime btn-sm">Buscar</button>
      </form>

      <div class="opening-tags-row">
        <nav class="quick-filter-bar" aria-label="Filtros essenciais da vitrine">
          <a class="filter-chip filter-chip--active" href="${esc(root)}ofertas/index.html">
            <span>Todos em destaque</span>
          </a>
          <a class="filter-chip" href="${esc(root)}categorias/hardware-e-perifericos/index.html">
            ${icon("cpu")}
            <span>Hardware &amp; PC</span>
          </a>
          <a class="filter-chip" href="${esc(root)}ofertas/index.html?condicao=com-cupom">
            ${icon("ticket")}
            <span>Com cupom ativo</span>
          </a>
          <a class="filter-chip" href="${esc(root)}categorias/ergonomia-e-suportes/index.html">
            ${icon("laptop")}
            <span>Ergonomia</span>
          </a>
          <a class="filter-chip" href="${esc(root)}categorias/energia-e-cabos/index.html">
            ${icon("zap")}
            <span>Energia e cabos</span>
          </a>
          <a class="filter-chip" href="${esc(root)}categorias/iluminacao-de-bancada/index.html">
            ${icon("lamp")}
            <span>Iluminação</span>
          </a>
          <a class="filter-chip" href="${esc(root)}categorias/organizacao-de-mesa/index.html">
            ${icon("grid")}
            <span>Organização</span>
          </a>
        </nav>

        <a class="opening-explore-link" href="${esc(root)}ofertas/index.html">
          <span>Explorar ofertas</span>
          ${icon("arrowRight")}
        </a>
      </div>
    </div>
  </section>

  <section class="daily-finds-section" aria-labelledby="vitrine-heading">
    <div class="container">
      <div class="section-row-head">
        <div class="section-row-title-wrap">
          <span class="section-icon-accent" aria-hidden="true">${icon("zap")}</span>
          <h2 id="vitrine-heading">Vitrine de achados do dia</h2>
          <span class="sage-pill-badge">
            <span>4 promoções reais ML (teste) · 15/11</span>
          </span>
        </div>
        <a class="section-more-link" href="${esc(root)}ofertas/index.html">
          <span>Ver catálogo completo</span>
          ${icon("arrowRight")}
        </a>
      </div>

      <div class="daily-finds-grid">
        ${dailyCards.map((card) => renderVerticalOfferCard(root, card)).join("")}
      </div>
    </div>
  </section>

  <section class="trust-strip-bar" aria-label="Pilares de transparência do Procurali">
    <div class="container">
      <div class="trust-strip-grid">
        <div class="trust-item">
          <span class="trust-icon-box" aria-hidden="true">${icon("shield")}</span>
          <div>
            <strong class="trust-title">Pesquisa e critério técnico</strong>
            <p class="trust-sub">Especificações conferidas com foco em uso real</p>
          </div>
        </div>
        <div class="trust-item">
          <span class="trust-icon-box" aria-hidden="true">${icon("clock")}</span>
          <div>
            <strong class="trust-title">Horário visível no card</strong>
            <p class="trust-sub">Itens &gt;30 min entram em atualização e pausam o clique</p>
          </div>
        </div>
        <div class="trust-item">
          <span class="trust-icon-box" aria-hidden="true">${icon("link")}</span>
          <div>
            <strong class="trust-title">Afiliado transparente</strong>
            <p class="trust-sub">Você sabe antes do clique quando há comissão possível</p>
          </div>
        </div>
        <div class="trust-item">
          <span class="trust-icon-box" aria-hidden="true">${icon("coins")}</span>
          <div>
            <strong class="trust-title">Sem pressão fabricada</strong>
            <p class="trust-sub">Prazos e cupons só aparecem quando realmente confirmados</p>
          </div>
        </div>
      </div>
    </div>
  </section>

  <section class="guides-showcase-section">
    <div class="container">
      <div class="section-row-head">
        <div class="section-row-title-wrap section-row-title-wrap--guides">
          <span class="section-icon-dark" aria-hidden="true">${icon("book")}</span>
          <h2>Guias e artigos</h2>
          <p class="section-inline-sub">Comparações práticas antes de decidir a compra</p>
        </div>
        <a class="section-more-link" href="${esc(root)}guias/index.html">
          <span>Ver todos os guias</span>
          ${icon("arrowRight")}
        </a>
      </div>

      <div class="guides-horizontal-grid">
        ${GUIDES.map((g) => {
          const vis = GUIDE_VISUALS[g.slug] || {};
          const guideHref = `${root}guias/${g.slug}/index.html`;
          const cardTitle = vis.homeTitle || g.title;
          return `
          <article class="guide-h-card">
            <a class="guide-h-thumb" href="${esc(guideHref)}" tabindex="-1" aria-hidden="true">
              <img src="${esc(root + (vis.thumbPath || "/brand/templates/guide-photo-ergonomia.jpg").replace(/^\//, ""))}" alt="${esc(vis.thumbAlt || cardTitle)}" width="440" height="260" loading="lazy">
            </a>
            <div class="guide-h-body">
              <span class="guide-type-pill">${esc(vis.badgeLabel || "Guia de compra")}</span>
              <h3><a href="${esc(guideHref)}">${esc(cardTitle)}</a></h3>
              <div class="guide-h-meta">
                <span>${icon("clock")}<span>Leitura: ${esc(vis.readingMinutes || 6)} min</span></span>
              </div>
            </div>
          </article>`;
        }).join("")}
      </div>
    </div>
  </section>

  <section class="community-showcase-section">
    <div class="container">
      <div class="community-strip-card">
        <div class="community-strip-left">
          <span class="community-plane-icon" aria-hidden="true">${icon("send")}</span>
          <h2>Acompanhe pelos grupos</h2>
        </div>

        <span class="community-strip-divider" aria-hidden="true"></span>

        <p class="community-strip-copy">Canal opcional para quem prefere alertas resumidos de ofertas e novos guias sem abrir o site todo dia.</p>

        <div class="community-strip-actions">
          <a class="btn-strip-tg" href="${esc(SITE_META.telegramUrl)}" target="_blank" rel="noopener" data-analytics-event="community_link_click" data-analytics-detail="telegram-banner">
            ${icon("send")}
            <span>Receber achados no Telegram</span>
          </a>
          <a class="btn-strip-wa" href="${esc(root)}grupos/index.html">
            ${icon("whatsapp")}
            <span>Ver grupos no WhatsApp</span>
          </a>
        </div>
      </div>
    </div>
  </section>`;
}

export function renderOffersListPage(root) {
  return `
  <section class="page-head">
    <div class="container">
      ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Ofertas" }])}
      <span class="kicker">Catálogo de testes · 4 ofertas reais Mercado Livre + fixtures</span>
      <h1>Ofertas e registros observados</h1>
      <p class="hero-lead">Filtre por categoria, faixa de preço e condições documentadas. Itens reais do Mercado Livre trazem link oficial <code>meli.la</code> sinalizado; fixtures de demonstração mantêm o botão desativado.</p>
    </div>
  </section>

  <section class="section-tight">
    <div class="container">
      <div class="notice-box notice-box--warning notice-inline">
        ${icon("info")}
        <p><strong>Composição do catálogo de testes:</strong> 4 ofertas reais de hardware/periféricos do Mercado Livre Afiliados (links oficiais <code>meli.la</code>) + 6 fixtures demonstrativos para testar estados reais do sistema (ativos demo, desatualizado &gt;30 min, expirado histórico e bloqueado por mídia).</p>
      </div>

      <div class="catalog-layout">
        <form id="offers-filter-form" class="filter-bar" aria-label="Filtros de ofertas">
          <details class="filter-details" data-filter-details open>
            <summary class="filter-summary">${icon("search")}<span>Filtros e ordenação</span></summary>
            <div class="filter-grid">
              <div class="field field--wide">
                <label for="f-q">Buscar por produto, marca, modelo ou código</label>
                <input id="f-q" name="q" type="search" placeholder="Ex.: Kingston, Redragon, monitor, 200Hz, notebook">
              </div>
              <div class="field">
                <label for="f-cat">Categoria</label>
                <select id="f-cat" name="categoria">
                  <option value="todas">Todas as categorias</option>
                  ${CATEGORIES.map((c) => `<option value="${esc(c.slug)}">${esc(c.name)}</option>`).join("")}
                </select>
              </div>
              <div class="field">
                <label for="f-band">Faixa de preço</label>
                <select id="f-band" name="faixa">
                  <option value="todas">Qualquer valor</option>
                  <option value="ate-100">Até R$ 100</option>
                  <option value="100-a-150">De R$ 100 a R$ 150</option>
                  <option value="150-a-300">De R$ 150 a R$ 300</option>
                  <option value="acima-de-300">Acima de R$ 300 (hardware)</option>
                </select>
              </div>
              <div class="field">
                <label for="f-cond">Condição</label>
                <select id="f-cond" name="condicao">
                  <option value="todas">Todas as condições</option>
                  <option value="amostra-7-dias">Com série de 7 dias</option>
                  <option value="com-cupom">Com cupom informado</option>
                  <option value="pix-sem-cupom">Sem exigência de cupom</option>
                </select>
              </div>
              <div class="field">
                <label for="f-sort">Ordenação explícita</label>
                <select id="f-sort" name="ordem">
                  <option value="relevance">Relevância editorial</option>
                  <option value="price_asc">Menor preço informado (sem frete)</option>
                  <option value="recent">Observação mais recente</option>
                </select>
              </div>
              <div class="field">
                <label for="f-status">Estado do registro</label>
                <select id="f-status" name="status">
                  <option value="ativas">Somente ativas (ML + demo)</option>
                  <option value="nao-ativas">Desatualizadas / expiradas / bloqueadas</option>
                  <option value="todas">Mostrar todos os estados</option>
                </select>
              </div>
            </div>
            <div class="filter-actions">
              <button type="submit" class="btn btn-petrol">Aplicar filtros</button>
              <button type="button" class="btn btn-ghost" id="offers-reset-btn">Limpar</button>
            </div>
          </details>
        </form>

        <div class="catalog-results">
          <p id="offers-results-count" class="results-count" aria-live="polite">Exibindo catálogo estático inicial.</p>
          <div id="offers-results" class="offer-list">
            ${OFFERS.filter((o) => o.state === "approved")
              .map((offer) => offerRowHtml(root, offer, { headingTag: "h2" }))
              .join("")}
          </div>
        </div>
      </div>
    </div>
  </section>`;
}

export function renderOfferDetailPage(root, offer) {
  const category = CATEGORIES.find((c) => c.slug === offer.categorySlug);
  const rights = MEDIA_RIGHTS.find((m) => m.id === offer.mediaRightsRef);
  const evidence = EVIDENCE_SOURCES.filter((e) => offer.evidenceRefs.includes(e.id));
  const freshness = isOfferFresh(offer, SITE_META.prototypeReferenceTimeUtc);
  const sevenDay = verifySevenDaySample(offer);
  const ctaProps = buildOfficialAffiliateAnchorProps(offer, MEDIA_RIGHTS);
  const alternatives = OFFERS.filter((o) => (offer.alternativeOfferSlugs || []).includes(o.slug));

  let stateAlertHtml = "";
  if (offer.state === "stale") {
    stateAlertHtml = `<div class="notice-box notice-box--warning"><strong>Preço em atualização (mais de 30 minutos):</strong> este registro foi observado há cerca de ${freshness.ageMinutes} minutos em relação à referência do protótipo (${esc(SITE_META.prototypeReferenceTimeLabelBrt)}). Conforme nossa política de frescor, o link comercial permanece pausado até nova checagem.</div>`;
  } else if (offer.state === "expired") {
    stateAlertHtml = `<div class="notice-box notice-box--warning"><strong>Oferta encerrada:</strong> este item pertence ao arquivo histórico e não deve ser tratado como preço vigente.</div>`;
  } else if (offer.state === "missing_rights") {
    stateAlertHtml = `<div class="notice-box notice-box--danger"><strong>Bloqueio por direitos de mídia:</strong> os direitos de imagem do item real ainda não foram confirmados (${esc(rights?.licenseType || "pendente")}). Nenhuma ação de compra é liberada.</div>`;
  } else if (!offer.isDemoFixture && offer.state === "approved") {
    stateAlertHtml = `<div class="notice-box notice-box--live"><strong>Promoção real em teste (${esc(offer.marketplaceLabel)} Afiliados · ${esc(offer.productId || offer.id)}):</strong> dados, especificações e variações capturados em ${esc(offer.observedAtDisplay)} via hub autenticado do proprietário. Preço, frete por CEP e disponibilidade devem ser confirmados na loja oficial ao clicar.</div>`;
  } else {
    stateAlertHtml = `<div class="notice-box"><strong>Item fictício de demonstração:</strong> catálogo ilustrativo sem vínculo com produto real nem link oficial ativo.</div>`;
  }

  const refPrice = offer.referencePriceCents ? formatBrl(offer.referencePriceCents) : "";
  const savingsCents =
    offer.referencePriceCents && offer.referencePriceCents > offer.priceBrlCents
      ? offer.referencePriceCents - offer.priceBrlCents
      : 0;
  const gallery = Array.isArray(offer.galleryPaths) && offer.galleryPaths.length > 0 ? offer.galleryPaths : [];
  const specs = Array.isArray(offer.specs) ? offer.specs : [];
  const variants = Array.isArray(offer.variants) ? offer.variants : [];
  const availabilityPt =
    offer.availability === "InStock"
      ? "Em estoque no anúncio (snapshot)"
      : offer.availability || "";

  return `
  <section class="section-tight">
    <div class="container">
      ${breadcrumbsHtml(root, [
        { label: "Início", href: "index.html" },
        { label: "Ofertas", href: "ofertas/index.html" },
        category
          ? { label: category.name, href: `categorias/${category.slug}/index.html` }
          : { label: "Categoria" },
        { label: offer.title },
      ])}

      ${stateAlertHtml}

      <div class="detail-layout">
        <figure class="detail-figure">
          <div class="detail-main-photo-wrap">
            <img id="detail-main-photo" data-gallery-main src="${esc(root + offer.imagePath.replace(/^\//, ""))}" alt="${esc(offer.imageAlt)}" width="640" height="480">
          </div>
          ${
            gallery.length > 1
              ? `<div class="detail-gallery-thumbs" role="group" aria-label="Fotos disponíveis do produto">
                  ${gallery
                    .map(
                      (img, idx) => `
                    <button
                      type="button"
                      class="detail-thumb-btn${idx === 0 ? " is-active" : ""}"
                      data-gallery-btn
                      data-gallery-src="${esc(root + img.path.replace(/^\//, ""))}"
                      data-gallery-alt="${esc(img.alt)}"
                      aria-label="Visualizar foto ${idx + 1} de ${gallery.length}"
                      aria-pressed="${idx === 0 ? "true" : "false"}">
                      <img src="${esc(root + img.path.replace(/^\//, ""))}" alt="${esc(img.alt)}" width="96" height="96" loading="lazy">
                    </button>`
                    )
                    .join("")}
                </div>`
              : ""
          }
          <figcaption>${esc(rights?.attributionText || "Ilustração técnica demonstrativa.")}</figcaption>
        </figure>

        <div>
          <div class="offer-meta-row" style="margin-bottom:0.5rem;">
            ${statusBadgeHtml(offer)}
            <span>${esc(offer.merchant)}</span>
            ${offer.cardBadge ? `<span class="badge badge-neutral">${esc(offer.cardBadge)}</span>` : ""}
            ${offer.sponsored ? '<span class="badge badge-warning">Patrocinado no ML</span>' : ""}
          </div>
          <h1>${esc(offer.title)}</h1>
          <p class="muted">
            ${offer.brand && offer.model ? `<strong>Marca/Modelo:</strong> ${esc(offer.brand)} · ${esc(offer.model)} · ` : ""}
            <strong>Variante exata:</strong> ${esc(offer.exactVariant)}
          </p>

          <div class="detail-badges-row">
            ${
              sevenDay.eligible
                ? `<span class="badge badge-emerald">✓ ${esc(sevenDay.label)}</span>`
                : `<span class="badge badge-neutral">Observação pontual datada (sem série de 7 dias)</span>`
            }
            ${
              offer.marketplaceRating
                ? `<span class="badge badge-petrol">${icon("star", "inline-star")} Avaliação ML: ${esc(offer.marketplaceRating)}${offer.marketplaceSalesLabel ? ` (${esc(offer.marketplaceSalesLabel)})` : ""}</span>`
                : ""
            }
            ${availabilityPt ? `<span class="badge badge-neutral">${esc(availabilityPt)}</span>` : ""}
          </div>

          <div class="affiliate-box">
            <div>
              <div class="offer-price-block">
                ${
                  refPrice
                    ? `<div class="offer-ref-row"><span class="price-old">De ${esc(refPrice)}</span>${offer.discountPercentLabel ? `<span class="discount-chip">${esc(offer.discountPercentLabel)}</span>` : ""}</div>`
                    : ""
                }
                <div class="offer-price">${esc(formatBrl(offer.priceBrlCents))}</div>
                ${savingsCents > 0 ? `<span class="savings-pill">Economia observada de ${esc(formatBrl(savingsCents))}</span>` : ""}
              </div>
              <p style="margin-top:0.55rem;margin-bottom:0.25rem;"><strong>Condição:</strong> ${esc(offer.paymentConditions)}</p>
              ${
                offer.returnDays
                  ? `<p style="margin-bottom:0.25rem;"><strong>Devolução:</strong> Grátis em até ${esc(offer.returnDays)} dias informada no anúncio oficial.</p>`
                  : ""
              }
              ${
                offer.validUntilDisplay
                  ? `<p class="deadline-banner">${esc(offer.validUntilDisplay)}</p>`
                  : ""
              }
              ${
                offer.couponCode
                  ? `<p style="margin-bottom:0.35rem;"><button type="button" class="coupon-pill" data-copy-code="${esc(offer.couponCode)}">Copiar código do cupom: <strong>${esc(offer.couponCode)}</strong></button></p>`
                  : ""
              }
              ${
                offer.shareId
                  ? `<p style="margin-bottom:0.35rem;"><button type="button" class="coupon-pill" data-copy-code="${esc(offer.shareId)}">Copiar código buscador ML: <strong>${esc(offer.shareId)}</strong></button></p>`
                  : ""
              }
              ${
                offer.couponCondition
                  ? `<p style="margin-bottom:0.25rem;"><strong>Regra do cupom:</strong> ${esc(offer.couponCondition)}</p>`
                  : ""
              }
              <p class="muted" style="margin-bottom:0.25rem;"><strong>Frete:</strong> ${esc(offer.freightStatus)}</p>
              <p class="muted" style="margin-bottom:0;"><strong>Observado em:</strong> ${esc(offer.observedAtDisplay)} · <strong>Fonte:</strong> ${esc(offer.program)}</p>
            </div>

            <div class="btn-row">
              ${
                ctaProps.enabled
                  ? `<a class="btn btn-action" href="${esc(ctaProps.href)}" rel="${esc(ctaProps.rel)}" target="${esc(ctaProps.target)}" data-analytics-event="outbound_offer_click" data-analytics-detail="${esc(offer.slug)}">${esc(ctaProps.label)} ${icon("external")}</a>`
                  : `<button type="button" class="btn" disabled aria-disabled="true">${esc(ctaProps.label)}</button>`
              }
              <button type="button" class="btn btn-secondary" data-save-offer="${esc(offer.slug)}" data-offer-title="${esc(offer.title)}" data-offer-price="${esc(formatBrl(offer.priceBrlCents))}">Salvar item</button>
            </div>

            <div style="border-top:1px solid var(--color-border);padding-top:var(--space-2);font-size:0.88rem;">
              <p style="margin-bottom:0.25rem;"><strong>${esc(SITE_META.affiliateDisclosure)}</strong></p>
              <p class="muted" style="margin-bottom:0.25rem;">${esc(SITE_META.priceChangeNotice)}</p>
              <p class="muted" style="margin-bottom:0;">Estado do link: ${esc(ctaProps.reason)}</p>
            </div>
          </div>
        </div>
      </div>

      <div class="section grid-2" style="margin-top:var(--space-6);">
        <article class="card">
          <h2>Para quem faz sentido</h2>
          <p>${esc(offer.forWhomItMakesSense)}</p>
          <h3>Benefícios com base na ficha</h3>
          <ul>
            ${offer.supportedBenefits.map((b) => `<li>${esc(b)}</li>`).join("")}
          </ul>
        </article>

        <article class="card card-alt">
          <h2>Compatibilidade, limitações e pontos em aberto</h2>
          <ul>
            ${offer.limitations.map((l) => `<li>${esc(l)}</li>`).join("")}
          </ul>
          <h3>O que não sabemos</h3>
          <ul>
            ${offer.unknowns.map((u) => `<li>${esc(u)}</li>`).join("")}
          </ul>
        </article>
      </div>

      ${
        variants.length > 0
          ? `<div class="section">
              <h2>Variantes listadas no catálogo do Mercado Livre (${variants.length})</h2>
              <p class="muted">Variações de SKU, cor, voltagem ou faixa de preço associadas ao grupo deste produto no anúncio oficial.</p>
              <div class="variants-grid">
                ${variants
                  .map(
                    (v) => `
                  <article class="card variant-card">
                    <div class="variant-head">
                      <span class="badge badge-neutral mono">${esc(v.productId)} · SKU ${esc(v.sku)}</span>
                      ${Number.isFinite(v.priceBrlCents) ? `<strong class="variant-price">${esc(formatBrl(v.priceBrlCents))}</strong>` : ""}
                    </div>
                    <h3>${esc(v.name)}</h3>
                    <p class="muted" style="margin-bottom:var(--space-2);">
                      ${v.color ? `Cor: <strong>${esc(v.color)}</strong>` : ""}${v.voltage ? `${v.color ? " · " : ""}Voltagem: <strong>${esc(v.voltage)}</strong>` : ""}${v.returnDays ? ` · Devolução ${esc(v.returnFees || "grátis")} (${esc(v.returnDays)}d)` : ""}
                    </p>
                    ${
                      Array.isArray(v.highlights) && v.highlights.length > 0
                        ? `<ul>${v.highlights.map((h) => `<li>${esc(h)}</li>`).join("")}</ul>`
                        : ""
                    }
                  </article>`
                  )
                  .join("")}
              </div>
            </div>`
          : ""
      }

      ${
        specs.length > 0
          ? `<div class="section">
              <h2>Ficha técnica extraída do anúncio (${specs.length} atributos)</h2>
              <p class="muted">Atributos estruturados coletados da tabela oficial de especificações no momento da captura.</p>
              <div class="specs-grid">
                ${specs
                  .slice(0, 14)
                  .map(
                    (sp) => `
                  <div class="spec-row">
                    <span class="spec-name">${esc(sp.name)}</span>
                    <span class="spec-val">${esc(sp.value)}</span>
                  </div>`
                  )
                  .join("")}
              </div>
              ${
                specs.length > 14
                  ? `<details class="specs-more">
                      <summary>Ver todas as ${specs.length} especificações (${specs.length - 14} restantes)</summary>
                      <div class="specs-grid" style="margin-top:var(--space-3);">
                        ${specs
                          .slice(14)
                          .map(
                            (sp) => `
                          <div class="spec-row">
                            <span class="spec-name">${esc(sp.name)}</span>
                            <span class="spec-val">${esc(sp.value)}</span>
                          </div>`
                          )
                          .join("")}
                      </div>
                    </details>`
                  : ""
              }
            </div>`
          : ""
      }

      ${
        offer.longDescription
          ? `<div class="section">
              <h2>Descrição completa reproduzida do anúncio</h2>
              <article class="card long-desc-card">
                ${renderMultilineParagraphs(offer.longDescription)}
                ${
                  offer.shareText
                    ? `<div class="share-snippet-box">
                        <strong>Texto de compartilhamento oficial gerado pelo hub (${esc(offer.shareId || "")}):</strong>
                        <pre id="ml-share-text-${esc(offer.productId || offer.slug)}" class="code-block">${esc(offer.shareText)}</pre>
                        <button type="button" class="btn btn-secondary btn-sm" data-copy-target="ml-share-text-${esc(offer.productId || offer.slug)}">Copiar mensagem oficial de divulgação</button>
                      </div>`
                    : ""
                }
              </article>
            </div>`
          : ""
      }

      <div class="section">
        <h2>Histórico de observações de preço (${esc(offer.priceObservations.length)})</h2>
        <p class="muted">${esc(offer.sevenDayCoverageNote)}</p>
        <div class="table-wrap table-stack">
          <table>
            <thead>
              <tr>
                <th scope="col">Data/hora (UTC)</th>
                <th scope="col">Variante</th>
                <th scope="col">Vendedor</th>
                <th scope="col">Base de pagamento</th>
                <th scope="col">Frete</th>
                <th scope="col">Valor</th>
              </tr>
            </thead>
            <tbody>
              ${offer.priceObservations
                .map(
                  (obs) => `
                  <tr>
                    <td class="mono" data-label="Data/hora (UTC)">${esc(obs.observedAt)}</td>
                    <td data-label="Variante">${esc(obs.variantId)}</td>
                    <td data-label="Vendedor">${esc(obs.sellerName)}</td>
                    <td data-label="Base de pagamento">${esc(obs.paymentBasis)}</td>
                    <td data-label="Frete">${esc(obs.freightNote)}</td>
                    <td data-label="Valor"><strong>${esc(formatBrl(obs.priceBrlCents))}</strong></td>
                  </tr>`
                )
                .join("")}
            </tbody>
          </table>
        </div>
      </div>

      <div class="section">
        <h2>Evidência e direitos de mídia</h2>
        <div class="grid-2">
          ${evidence
            .map(
              (ev) => `
              <article class="card">
                <span class="badge badge-neutral">${esc(ev.sourceType)}</span>
                <h3>${esc(ev.title)}</h3>
                <p class="muted">${esc(ev.publisher)} · Coletado em ${esc(ev.retrievedAt)}</p>
                <ul>
                  ${ev.verifiedFacts.map((f) => `<li>${esc(f)}</li>`).join("")}
                </ul>
              </article>`
            )
            .join("")}
          <article class="card">
            <span class="badge ${rights?.rightsVerified ? "badge-lime" : "badge-danger"}">${
    rights?.rightsVerified
      ? offer.isDemoFixture
        ? "✓ Uso ilustrativo permitido no protótipo"
        : "✓ Mídia vinculada ao programa Mercado Livre Afiliados"
      : "⛔ Direitos pendentes"
  }</span>
            <h3>Registro de direitos (${esc(rights?.id || "-")})</h3>
            <p><strong>Descrição:</strong> ${esc(rights?.assetDescription || "-")}</p>
            <p><strong>Licença:</strong> ${esc(rights?.licenseType || "-")}</p>
            <p class="muted" style="margin-bottom:0;"><strong>Escopo permitido:</strong> ${esc(rights?.ownerPermittedUse || "-")}</p>
          </article>
        </div>
      </div>

      ${
        alternatives.length > 0
          ? `<div class="section">
              <h2>Alternativas para comparar</h2>
              <div class="grid-2">
                ${alternatives
                  .map(
                    (alt) => `
                    <article class="card">
                      <div>${statusBadgeHtml(alt)}</div>
                      <h3><a href="${esc(root)}ofertas/${esc(alt.slug)}/index.html">${esc(alt.title)}</a></h3>
                      <p class="muted">${esc(alt.exactVariant)}</p>
                      <p>${esc(alt.oneLineReason)}</p>
                      <strong class="offer-price">${esc(formatBrl(alt.priceBrlCents))}</strong>
                    </article>`
                  )
                  .join("")}
              </div>
            </div>`
          : ""
      }
    </div>
  </section>`;
}

export function renderCategoriesIndexPage(root) {
  return `
  <section class="section-tight">
    <div class="container">
      ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Categorias" }])}
      <span class="kicker">Estrutura editável pelo proprietário</span>
      <h1>Categorias do piloto</h1>
      <p class="hero-lead">Estas categorias representam a hipótese inicial de nicho (casa, mesa e tecnologia prática entre R$ 30 e R$ 300) e podem ser editadas antes do lançamento.</p>
      <div class="grid-2">
        ${CATEGORIES.map((cat) => {
          const count = OFFERS.filter((o) => o.categorySlug === cat.slug).length;
          return `
          <article class="card">
            <h2><a href="${esc(root)}categorias/${esc(cat.slug)}/index.html">${esc(cat.name)}</a></h2>
            <p>${esc(cat.description)}</p>
            <p class="muted">${esc(cat.intendedBudgetHint)} · ${count} registro(s) demonstrativo(s)</p>
            <div class="card-foot">
              <a class="btn btn-petrol" href="${esc(root)}categorias/${esc(cat.slug)}/index.html">Ver categoria</a>
            </div>
          </article>`;
        }).join("")}
      </div>
    </div>
  </section>`;
}

export function renderCategoryPage(root, category) {
  const catOffers = OFFERS.filter((o) => o.categorySlug === category.slug);
  const catGuides = GUIDES.filter((g) => g.categorySlug === category.slug);
  return `
  <section class="section-tight">
    <div class="container">
      ${breadcrumbsHtml(root, [
        { label: "Início", href: "index.html" },
        { label: "Categorias", href: "categorias/index.html" },
        { label: category.name },
      ])}
      <span class="kicker">Categoria · Nicho inicial editável</span>
      <h1>${esc(category.name)}</h1>
      <p class="hero-lead">${esc(category.description)}</p>
      <p class="muted">Faixa indicativa inicial: ${esc(category.intendedBudgetHint)}</p>

      <div class="section">
        <h2>Registros demonstrativos nesta categoria (${catOffers.length})</h2>
        <div class="offer-list">
          ${catOffers.map((offer) => offerRowHtml(root, offer)).join("")}
        </div>
      </div>

      ${
        catGuides.length > 0
          ? `<div class="section">
              <h2>Guias relacionados</h2>
              <div class="grid-2">
                ${catGuides
                  .map(
                    (g) => `
                    <article class="card">
                      <h3><a href="${esc(root)}guias/${esc(g.slug)}/index.html">${esc(g.title)}</a></h3>
                      <p>${esc(g.summary)}</p>
                    </article>`
                  )
                  .join("")}
              </div>
            </div>`
          : ""
      }
    </div>
  </section>`;
}

export function renderGuidesIndexPage(root) {
  return `
  <section class="section-tight">
    <div class="container">
      ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Guias" }])}
      <span class="kicker">Conteúdo editorial · Pesquisa de bancada</span>
      <h1>Guias para decidir sem pressa</h1>
      <p class="hero-lead">Critérios técnicos, comparações por cenário e erros comuns explicados em linguagem direta, sem inventar testes físicos.</p>
      <div class="grid-3">
        ${GUIDES.map((g) => {
          const vis = GUIDE_VISUALS[g.slug] || {};
          return `
          <article class="card">
            <a class="guide-card-cover" href="${esc(root)}guias/${esc(g.slug)}/index.html" tabindex="-1" aria-hidden="true">
              <img src="${esc(root + (vis.thumbPath || "/brand/templates/guide-thumb-ergonomia.svg").replace(/^\//, ""))}" alt="${esc(vis.thumbAlt || g.title)}" width="320" height="260" loading="lazy">
            </a>
            <div><span class="guide-type-pill">${esc(vis.badgeLabel || "GUIA DE COMPRA")} · Pesquisa de bancada</span></div>
            <h2><a href="${esc(root)}guias/${esc(g.slug)}/index.html">${esc(g.title)}</a></h2>
            <p>${esc(g.summary)}</p>
            <p class="muted" style="margin-bottom:0;">Publicado: ${esc(g.publishedAt)} · Atualizado: ${esc(g.updatedAt)} · ${esc(vis.readingMinutes || 6)} min</p>
            <div class="card-foot">
              <a class="btn btn-petrol" href="${esc(root)}guias/${esc(g.slug)}/index.html">Ler guia</a>
            </div>
          </article>`;
        }).join("")}
      </div>
      <div class="notice-box" style="margin-top:var(--space-6);">
        <strong>Seleção semanal separada:</strong> veja também o recorte por data em <a href="${esc(root)}selecoes-da-semana/index.html">Seleções da semana</a>.
      </div>
    </div>
  </section>`;
}

export function renderGuideDetailPage(root, guide) {
  const relatedOffers = OFFERS.filter((o) => guide.relatedOfferSlugs.includes(o.slug));
  return `
  <section class="section-tight">
    <div class="container">
      ${breadcrumbsHtml(root, [
        { label: "Início", href: "index.html" },
        { label: "Guias", href: "guias/index.html" },
        { label: guide.title },
      ])}

      <article>
        <header class="article-head">
          <span class="badge badge-warning">📋 ${esc(guide.methodologyLabel)}</span>
          <h1>${esc(guide.title)}</h1>
          <p class="hero-lead">${esc(guide.summary)}</p>
          <div class="article-meta muted">
            <span><strong>Autor:</strong> ${esc(guide.author)}</span>
            <span><strong>Revisor:</strong> ${esc(guide.reviewer)}</span>
            <span><strong>Publicado:</strong> ${esc(guide.publishedAt)}</span>
            <span><strong>Atualizado:</strong> ${esc(guide.updatedAt)}</span>
          </div>
        </header>

        <div class="article-layout">
          <aside class="toc-box" aria-label="Índice do guia">
            <h2>Neste guia</h2>
            <ul class="toc-list">
              ${guide.tableOfContents
                .map((item) => `<li><a href="#${esc(item.id)}">${esc(item.label)}</a></li>`)
                .join("")}
            </ul>
            <p class="toc-disclosure muted">
              ${esc(SITE_META.affiliateDisclosure)}
            </p>
          </aside>

          <div class="prose">
            ${guide.sections
              .map(
                (sec) => `
                <section id="${esc(sec.id)}" style="margin-bottom:var(--space-6);">
                  <h2>${esc(sec.heading)}</h2>
                  ${sec.paragraphs.map((p) => `<p>${esc(p)}</p>`).join("")}
                </section>`
              )
              .join("")}

            <section id="comparacao" style="margin-bottom:var(--space-6);">
              <h2>Comparação de cenários e contrapontos</h2>
              <div class="table-wrap table-stack">
                <table>
                  <thead>
                    <tr>
                      <th scope="col">Critério</th>
                      <th scope="col">Opção A</th>
                      <th scope="col">Opção B</th>
                    </tr>
                  </thead>
                  <tbody>
                    ${guide.comparisonRows
                      .map(
                        (r) => `
                        <tr>
                          <th scope="row">${esc(r.criterio)}</th>
                          <td data-label="Opção A">${esc(r.opcaoA)}</td>
                          <td data-label="Opção B">${esc(r.opcaoB)}</td>
                        </tr>`
                      )
                      .join("")}
                  </tbody>
                </table>
              </div>
            </section>

            <section id="fontes" style="margin-bottom:var(--space-6);">
              <h2>Fontes, metodologia e histórico</h2>
              <p><strong>Metodologia:</strong> ${esc(guide.methodologyLabel)}.</p>
              <ul>
                ${guide.sourceLinks
                  .map(
                    (s) =>
                      `<li><a href="${esc(s.url)}" target="_blank" rel="noopener">${esc(s.label)}</a> (consultado em ${esc(s.consultedAt)})</li>`
                  )
                  .join("")}
              </ul>
              <details class="editorial-details">
                <summary>Histórico de atualizações materiais</summary>
                <ul style="margin-bottom:0;">
                  ${guide.updateHistory.map((h) => `<li>${esc(h)}</li>`).join("")}
                </ul>
              </details>
            </section>

            ${
              relatedOffers.length > 0
                ? `<section>
                    <h2>Registros demonstrativos relacionados</h2>
                    <div class="grid-2">
                      ${relatedOffers
                        .map(
                          (o) => `
                          <article class="card">
                            <div>${statusBadgeHtml(o)}</div>
                            <h3><a href="${esc(root)}ofertas/${esc(o.slug)}/index.html">${esc(o.title)}</a></h3>
                            <p class="muted">${esc(o.exactVariant)}</p>
                            <strong class="offer-price">${esc(formatBrl(o.priceBrlCents))}</strong>
                          </article>`
                        )
                        .join("")}
                    </div>
                  </section>`
                : ""
            }
          </div>
        </div>
      </article>
    </div>
  </section>`;
}

export function renderWeeklySelectionPage(root) {
  const expiredOffer = OFFERS.find((o) => o.slug === WEEKLY_SELECTION.expiredHistoricalReference.offerSlug);
  return `
  <section class="section-tight">
    <div class="container">
      ${breadcrumbsHtml(root, [
        { label: "Início", href: "index.html" },
        { label: "Seleções da semana" },
      ])}
      <span class="kicker">${esc(WEEKLY_SELECTION.dateRangeLabel)}</span>
      <h1>${esc(WEEKLY_SELECTION.title)}</h1>
      <p class="hero-lead">${esc(WEEKLY_SELECTION.scopeExplanation)}</p>

      <div class="grid-3">
        ${WEEKLY_SELECTION.picks
          .map((pick) => {
            const offer = OFFERS.find((o) => o.slug === pick.offerSlug);
            if (!offer) return "";
            return `
            <article class="card">
              <div><span class="badge badge-lime">${esc(pick.roleLabel)}</span></div>
              <h2><a href="${esc(root)}ofertas/${esc(offer.slug)}/index.html">${esc(offer.title)}</a></h2>
              <p class="muted">${esc(offer.exactVariant)}</p>
              <strong class="offer-price">${esc(formatBrl(offer.priceBrlCents))}</strong>
              <p class="muted">Obs.: ${esc(offer.observedAtDisplay)}</p>
              <p>${esc(pick.note)}</p>
              <div class="card-foot">
                <a class="btn btn-petrol" href="${esc(root)}ofertas/${esc(offer.slug)}/index.html">Ver critérios e ficha</a>
              </div>
            </article>`;
          })
          .join("")}
      </div>

      ${
        expiredOffer
          ? `<div class="section">
              <h2>${esc(WEEKLY_SELECTION.expiredHistoricalReference.label)}</h2>
              <div class="notice-box notice-box--warning">
                <p><strong>${esc(expiredOffer.title)}</strong> (${esc(formatBrl(expiredOffer.priceBrlCents))} em ${esc(expiredOffer.observedAtDisplay)}) — ${esc(WEEKLY_SELECTION.expiredHistoricalReference.note)}</p>
                <a href="${esc(root)}ofertas/${esc(expiredOffer.slug)}/index.html">Consultar registro histórico separado</a>
              </div>
            </div>`
          : ""
      }
    </div>
  </section>`;
}

export function renderAiFinderPage(root) {
  return `
  <section class="section-tight">
    <div class="container">
      ${breadcrumbsHtml(root, [
        { label: "Início", href: "index.html" },
        { label: "Encontre com IA" },
      ])}
      <span class="kicker">Seletor guiado · Protótipo determinístico</span>
      <h1>Encontre opções para sua necessidade</h1>
      <div class="notice-box notice-box--petrol">
        <p style="margin-bottom:0.35rem;"><strong>Demonstração — sem consulta ao vivo.</strong></p>
        <p style="margin-bottom:0;">As respostas abaixo são calculadas localmente sobre o catálogo fictício do protótipo. Nenhuma chave de API ou chamada de modelo é executada no navegador, e seus textos não são salvos.</p>
      </div>

      <div class="finder-layout">
        <form id="ai-finder-form" class="card finder-form" aria-label="Formulário guiado de seleção">
          <h2>Conte o que você precisa</h2>
          <div class="field">
            <label for="ai-need">O que você precisa resolver?</label>
            <input id="ai-need" name="need" type="text" maxlength="120" value="notebook postura mesa" placeholder="Ex.: conectar monitor HDMI, melhorar postura, iluminar mesa">
          </div>
          <div class="field">
            <label for="ai-budget">Quanto pretende gastar no máximo? (R$)</label>
            <input id="ai-budget" name="budget" type="number" min="20" max="1000" step="10" value="180">
          </div>
          <div class="field">
            <label for="ai-req">O que não pode faltar?</label>
            <select id="ai-req" name="requirement">
              <option value="qualquer">Sem restrição específica</option>
              <option value="ergonomia">Ergonomia e suporte</option>
              <option value="energia">Energia e cabos</option>
              <option value="iluminacao">Iluminação de bancada</option>
              <option value="sem-cupom">Sem depender de cupom</option>
            </select>
          </div>
          <div class="field">
            <label for="ai-sim">Simular estado do sistema (validação de protótipo)</label>
            <select id="ai-sim" name="simulatedState">
              <option value="normal">Normal (catálogo demonstrativo ativo)</option>
              <option value="source-stale">Catálogo desatualizado (&gt;30 min)</option>
              <option value="service-unavailable">Serviço indisponível</option>
              <option value="budget-cap">Teto mensal de US$ 20 atingido</option>
            </select>
          </div>

          <p class="muted" style="font-size:0.86rem;margin-bottom:0;">
            <strong>Privacidade:</strong> não informe CPF, dados bancários, senhas, endereço completo ou credenciais de conta.
          </p>

          <div class="btn-row">
            <button type="submit" class="btn btn-primary">Buscar no catálogo demo</button>
          </div>

          <div class="finder-presets">
            <p class="muted" style="font-size:0.86rem;margin-bottom:0.5rem;">Atalhos de teste de estados:</p>
            <div class="chip-row">
              <button type="button" class="chip-btn" data-finder-preset data-need="notebook mesa" data-budget="180" data-req="qualquer" data-state="normal">Exemplo com resultado</button>
              <button type="button" class="chip-btn" data-finder-preset data-need="cadeira reclinável" data-budget="40" data-req="qualquer" data-state="normal">Sem correspondência</button>
              <button type="button" class="chip-btn" data-finder-preset data-need="hub usb" data-budget="180" data-req="qualquer" data-state="budget-cap">Simular teto US$ 20</button>
            </div>
          </div>
        </form>

        <div>
          <h2>Resultado da seleção guiada</h2>
          <div id="ai-finder-output" aria-live="polite">
            <div class="notice-box">
              <p><span class="badge badge-petrol">Demonstração — sem consulta ao vivo</span></p>
              <p>Se o JavaScript estiver desativado, consulte diretamente a <a href="${esc(root)}ofertas/index.html">lista completa de ofertas</a> ou os <a href="${esc(root)}guias/index.html">guias de compra</a> sem depender do seletor interativo.</p>
            </div>
          </div>

          <details class="editorial-details" style="margin-top:var(--space-4);">
            <summary>Arquitetura planejada para produção (ainda não conectada)</summary>
            <ul style="margin-bottom:0;">
              <li><strong>Adaptador no servidor:</strong> chaves protegidas fora do navegador; validação e sanitização de entrada curta.</li>
              <li><strong>Catálogo fechado:</strong> recomendações baseadas apenas em ofertas e guias aprovados com timestamp.</li>
              <li><strong>Teto agregado:</strong> US$ 20/mês combinados entre ${esc(AI_BUDGET_POLICY.allowedRoutes.join(" e "))}, Extra Usage desligado e pausa imediata se a telemetria falhar.</li>
            </ul>
          </details>
        </div>
      </div>
    </div>
  </section>`;
}

export function renderGroupsPage(root) {
  return `
  <section class="section-tight">
    <div class="container">
      ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Grupos e comunidades" }])}
      <span class="kicker">Comunidades oficiais do proprietário</span>
      <h1>Canais e grupos para acompanhar ofertas</h1>
      <p class="hero-lead">Diretório exclusivo das comunidades geridas pelo proprietário do Procurali. A entrada é sempre voluntária, sem notificações forçadas nem envio automático em massa.</p>

      <div class="grid-2">
        ${COMMUNITIES.map((comm) => {
          const isTelegramActive = comm.network === "telegram" && comm.availabilityState === "active" && comm.inviteUrl;
          const badgeClass = isTelegramActive ? "badge-lime" : comm.availabilityState === "pending_owner_invite" ? "badge-warning" : "badge-neutral";
          const statusLabel = isTelegramActive
            ? "✓ Canal oficial ativo"
            : comm.availabilityState === "pending_owner_invite"
            ? "⏳ Prévia — aguardando convite do proprietário"
            : comm.availabilityState === "revoked_rotating"
            ? "🔄 Convite em rotação"
            : "⛔ Grupo fechado / indisponível";

          return `
          <article class="card">
            <div class="offer-meta-row">
              <span class="badge badge-petrol">${comm.network === "telegram" ? "Telegram" : "WhatsApp"}</span>
              <span class="badge ${badgeClass}">${esc(statusLabel)}</span>
            </div>
            <h2>${esc(comm.name)}</h2>
            <p class="mono muted" style="margin-bottom:0;">${esc(comm.handle)}</p>
            <p>${esc(comm.purpose)}</p>
            <p><strong>Tema:</strong> ${esc(comm.categoryTopic)}<br><strong>Frequência prevista:</strong> ${esc(comm.intendedFrequency)}</p>
            <div>
              <strong>Regras básicas:</strong>
              <ul>
                ${comm.rules.map((r) => `<li>${esc(r)}</li>`).join("")}
              </ul>
            </div>
            <div class="notice-box" style="margin-bottom:var(--space-2);font-size:0.88rem;">
              <strong>Privacidade antes de entrar:</strong> ${esc(comm.privacyNotice)}
            </div>
            <div class="card-foot">
              ${
                isTelegramActive
                  ? `<a class="btn btn-primary btn-block" href="${esc(comm.inviteUrl)}" target="_blank" rel="noopener" data-analytics-event="community_link_click" data-analytics-detail="${esc(comm.slug)}">${esc(comm.ctaLabel)}</a>`
                  : `<button type="button" class="btn btn-block" disabled aria-disabled="true">${esc(comm.ctaLabel)}</button>`
              }
            </div>
          </article>`;
        }).join("")}
      </div>
    </div>
  </section>`;
}

export function renderTrustPage(root, kind) {
  if (kind === "sobre") {
    return `
    <section class="section-tight">
      <div class="container prose">
        ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Sobre o Procurali" }])}
        <span class="kicker">Identidade e papel do serviço</span>
        <h1>Sobre o Procurali (Procura Ali)</h1>
        <p class="hero-lead">O Procurali é um guia editorial de compras e curadoria de ofertas criado para ajudar compradores no Brasil a comparar especificações, condições e limitações antes de decidir.</p>
        <h2>O que somos — e o que não somos</h2>
        <ul>
          <li><strong>Não somos loja nem marketplace:</strong> não mantemos estoque, não processamos pagamentos, não emitimos nota fiscal de venda e não fornecemos garantia de fabricante.</li>
          <li><strong>A compra acontece na loja oficial:</strong> todo pedido, frete, troca e atendimento pós-venda é responsabilidade exclusiva do vendedor e da plataforma identificada.</li>
          <li><strong>Independência visual:</strong> embora utilize links de programas de afiliados quando aprovados, o Procurali (Procura Ali) é um projeto independente sem vínculo societário com Shopee, AliExpress ou outros varejistas.</li>
        </ul>
        <p class="muted"><strong>Nota de protótipo:</strong> os dados cadastrais finais, e-mail de contato dedicado e aprovação do slogan (“${esc(SITE_META.proposedTagline)}”) serão inseridos pelo proprietário antes do lançamento público.</p>
      </div>
    </section>`;
  }

  if (kind === "como-selecionamos") {
    return `
    <section class="section-tight">
      <div class="container prose">
        ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Como selecionamos" }])}
        <span class="kicker">Critérios editoriais</span>
        <h1>Como o Procurali seleciona ofertas e guias</h1>
        <ol>
          <li><strong>Aderência a uso real:</strong> priorizamos problemas concretos (postura na mesa, falta de portas USB-C, proteção elétrica) em vez de volume diário de links.</li>
          <li><strong>Evidência e restrição de testes:</strong> quando comparamos apenas fichas técnicas e documentação pública, rotulamos explicitamente como <em>pesquisa de bancada</em>. Nunca simulamos testes de bancada física não realizados.</li>
          <li><strong>Frescor de preço (janela de 30 minutos):</strong> em produção, preços anunciados exigem verificação próxima da publicação; registros vencidos passam ao estado desatualizado e bloqueiam o botão de compra.</li>
          <li><strong>Amostra mínima de 7 dias para comparação semanal:</strong> só exibimos “Menor preço observado pelo Procurali nesta semana” quando há série documentada de 7 dias na mesma variante, vendedor e condição de pagamento.</li>
          <li><strong>Direitos de mídia e revisão humana:</strong> ausência de licença de imagem ou alteração de texto invalida a publicação até nova aprovação do proprietário.</li>
        </ol>
      </div>
    </section>`;
  }

  if (kind === "transparencia") {
    return `
    <section class="section-tight">
      <div class="container prose">
        ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Transparência de afiliados" }])}
        <span class="kicker">Modelo comercial e publicidade</span>
        <h1>Transparência de afiliados e divulgação (#publi)</h1>
        <p>O acesso aos guias, seleções e canais do Procurali é gratuito para o leitor. A operação planejada é financiada por comissões de programas de afiliados quando um leitor decide comprar na loja oficial através de um link aprovado.</p>
        <ul>
          <li><strong>Sem custo adicional:</strong> o link de afiliado não acrescenta taxa no seu pedido.</li>
          <li><strong>Identificação próxima ao clique:</strong> exibimos “${esc(SITE_META.affiliateDisclosure)}” junto ao botão de saída e utilizamos <code>#publi</code> ou aviso equivalente em postagens sociais e vídeos.</li>
          <li><strong>Qualificação técnica dos links:</strong> âncoras de afiliado usam <code>rel="sponsored noopener"</code> e preservam integralmente a URL oficial do programa, sem encurtadores opacos nem parâmetros inventados.</li>
          <li><strong>Estado atual do piloto:</strong> a inscrição no programa de afiliados da Shopee com o canal no Telegram segue sob análise; as 4 ofertas reais de hardware do <strong>Mercado Livre Afiliados</strong> presentes neste protótipo utilizam links oficiais <code>meli.la</code> para validação de fluxo, enquanto os demais itens fictícios de demonstração mantêm o botão desativado.</li>
        </ul>
      </div>
    </section>`;
  }

  if (kind === "privacidade") {
    return `
    <section class="section-tight">
      <div class="container prose">
        ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Privacidade" }])}
        <span class="badge badge-warning">Rascunho de política para revisão do proprietário antes da publicação</span>
        <h1 style="margin-top:var(--space-3);">Política de Privacidade (rascunho do protótipo)</h1>
        <p>Este texto descreve o comportamento técnico planejado e implementado neste protótipo. Deve ser revisado e completado com os dados finais do controlador antes do lançamento em <code>procurali.com.br</code>.</p>
        <h2>1. Sem cadastro obrigatório e sem pixels ocultos</h2>
        <p>A navegação pelas ofertas, guias e canais é pública e não exige criação de conta. Este protótipo não carrega pixels de publicidade, rastreadores de terceiros nem scripts externos ocultos.</p>
        <h2>2. Itens salvos e preferências no navegador</h2>
        <p>Ao clicar em “Salvar item” ou ativar a medição demonstrativa abaixo, os dados ficam exclusivamente no <code>localStorage</code> do seu próprio navegador.</p>
        <h2>3. Consultas ao Encontre com IA</h2>
        <p>No protótipo, o seletor funciona localmente sem enviar consultas pela rede. Em produção, a política planejada é processar consultas de forma efêmera, sem reter histórico de conversa nem solicitar CPF, senha, endereço completo ou dados bancários.</p>

        <div class="card" style="margin-top:var(--space-5);">
          <h3>Controle de medição opcional (demonstração local)</h3>
          <p id="analytics-consent-status" class="muted">Medição opcional DESATIVADA por padrão.</p>
          <label style="display:flex;align-items:center;gap:0.6rem;font-weight:700;cursor:pointer;">
            <input type="checkbox" id="analytics-consent-toggle" style="width:20px;height:20px;">
            Permitir registro local demonstrativo de cliques (separa cliques em ofertas de cliques em comunidade; nada sai do navegador)
          </label>
          <div id="analytics-demo-log" style="margin-top:var(--space-3);"></div>
          <div class="btn-row" style="margin-top:var(--space-2);">
            <button type="button" class="btn btn-secondary" id="analytics-clear-log">Limpar log local</button>
          </div>
        </div>
      </div>
    </section>`;
  }

  if (kind === "termos") {
    return `
    <section class="section-tight">
      <div class="container prose">
        ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Termos de uso" }])}
        <span class="badge badge-warning">Rascunho para revisão do proprietário antes do lançamento</span>
        <h1 style="margin-top:var(--space-3);">Termos de Uso (rascunho do protótipo)</h1>
        <ul>
          <li><strong>Natureza informativa:</strong> o Procurali publica guias e observações de oferta baseadas em evidências datadas. Não garantimos disponibilidade contínua, estoque, entrega grátis universal nem manutenção do preço na loja parceira.</li>
          <li><strong>Relação de compra:</strong> pagamentos, prazos de entrega, trocas, devoluções e garantias seguem os termos e a responsabilidade do vendedor e da plataforma onde você concluir o pedido.</li>
          <li><strong>Comunidades externas:</strong> a adesão ao Telegram ou a futuros grupos de WhatsApp é voluntária e regida pelas regras do respectivo aplicativo.</li>
        </ul>
      </div>
    </section>`;
  }

  // contato
  return `
  <section class="section-tight">
    <div class="container prose">
      ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Contato" }])}
      <span class="kicker">Canal de contato · Protótipo</span>
      <h1>Contato e correções editoriais</h1>
      <p>Encontrou um preço desatualizado, erro de compatibilidade em um guia ou dúvida sobre transparência? Enquanto o domínio e o e-mail de suporte não são ativados pelo proprietário, acompanhe os avisos públicos pelo canal <a href="${esc(SITE_META.telegramUrl)}" target="_blank" rel="noopener">${esc(SITE_META.telegramHandle)}</a>.</p>

      <div class="notice-box notice-box--warning">
        <strong>Formulário demonstrativo:</strong> nenhum backend de envio está conectado neste protótipo estático. O botão de envio permanece desabilitado para evitar falsa confirmação de recebimento.
      </div>

      <form id="contact-demo-form" class="card" aria-label="Formulário de contato em demonstração">
        <div class="field">
          <label for="c-name">Seu nome ou apelido</label>
          <input id="c-name" name="name" type="text" placeholder="Opcional">
        </div>
        <div class="field">
          <label for="c-topic">Assunto</label>
          <select id="c-topic" name="topic">
            <option>Correção de ficha técnica ou preço</option>
            <option>Dúvida sobre transparência / afiliados</option>
            <option>Reportar link de grupo expirado</option>
          </select>
        </div>
        <div class="field">
          <label for="c-msg">Mensagem</label>
          <textarea id="c-msg" name="message" placeholder="Não informe CPF, senhas ou dados bancários."></textarea>
        </div>
        <button type="button" class="btn" disabled aria-disabled="true">Enviar mensagem (desabilitado — sem backend configurado)</button>
      </form>
    </div>
  </section>`;
}

export function renderCreativeKitPage(root) {
  return `
  <section class="section-tight">
    <div class="container">
      ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Kit criativo e identidade" }])}
      <span class="kicker">Identidade visual e templates reutilizáveis</span>
      <h1>Sistema visual Procurali e peças de demonstração</h1>
      <p class="hero-lead">Logotipos em SVG, paleta acessível, cards determinísticos, carrossel de 5 slides, roteiro de vídeo vertical e pacotes manuais para Telegram e WhatsApp.</p>

      <div class="section">
        <h2>Variantes de marca e arquivos vetoriais</h2>
        <div class="grid-2">
          <div class="card">
            <img src="${esc(root)}brand/logo-primary-light.svg" alt="Logotipo principal Procurali em fundo claro" width="360" height="88">
            <p class="muted" style="margin-bottom:0;">Versão principal clara (<code>#FFFFFF</code> / <code>#143D2B</code> / CTA <code>#C2410C</code>)</p>
          </div>
          <div class="card card--dark-brand">
            <img src="${esc(root)}brand/logo-primary-dark.svg" alt="Logotipo principal Procurali em fundo verde escuro" width="360" height="88">
            <p style="margin-bottom:0;color:#E5E7EB;">Versão principal escura (<code>#143D2B</code> / <code>#FFFFFF</code> / destaque <code>#A3E635</code>)</p>
          </div>
          <div class="card card--dark-brand">
            <img src="${esc(root)}brand/procurali_icone_branco_transparente.svg" alt="Ícone oficial Procurali branco transparente (favicon do navegador)" width="96" height="96">
            <p style="margin-bottom:0;color:#E5E7EB;">Ícone oficial do navegador (<code>brand/procurali_icone_branco_transparente.svg</code>)</p>
          </div>
          <div class="card">
            <img src="${esc(root)}brand/symbol-mark.svg" alt="Símbolo oficial Procurali verde escuro" width="96" height="96">
            <p class="muted" style="margin-bottom:0;">Símbolo oficial sobre fundo claro (<code>#143D2B</code>)</p>
          </div>
          <div class="card">
            <img src="${esc(root)}brand/logo-monochrome-dark.svg" alt="Logotipo monocromático escuro Procurali" width="360" height="88">
            <p class="muted" style="margin-bottom:0;">Monocromático escuro para carimbos e impressão</p>
          </div>
          <div class="card card--dark-brand">
            <img src="${esc(root)}brand/logo-monochrome-light.svg" alt="Logotipo monocromático claro Procurali" width="360" height="88">
            <p style="margin-bottom:0;color:#E5E7EB;">Monocromático claro sobre superfície escura</p>
          </div>
        </div>
      </div>

      <div class="section">
        <h2>Templates visuais reutilizáveis (SVG determinístico)</h2>
        <div class="grid-2">
          ${CREATIVE_ASSETS.map(
            (asset) => `
            <article class="card">
              <span class="badge badge-neutral">${esc(asset.format)}</span>
              <h3>${esc(asset.title)}</h3>
              <img src="${esc(root + asset.assetPath.replace(/^\//, ""))}" alt="${esc(asset.title)}" loading="lazy" style="border:1px solid var(--color-border);border-radius:var(--radius-sm);">
              <p>${esc(asset.notes)}</p>
              <p class="muted" style="margin-bottom:0;"><strong>Divulgação obrigatória:</strong> ${esc(asset.disclosureText)}</p>
            </article>`
          ).join("")}
        </div>
      </div>

      <div class="section">
        <h2>Roteiro de vídeo curto vertical (25–40 s · Storyboard)</h2>
        <p class="muted">Referência conceitual de etapas separadas (brief → roteiro → arte → revisão → publicação futura), sem alterar perfis nem automações existentes do YoutubeBot.</p>
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Janela</th>
                <th>Visual / Zona segura</th>
                <th>Fala e legenda (pt-BR)</th>
              </tr>
            </thead>
            <tbody>
              ${SHORT_VIDEO_STORYBOARD.map(
                (row) => `
                <tr>
                  <td class="mono"><strong>${esc(row.window)}</strong></td>
                  <td>${esc(row.visual)}</td>
                  <td>${esc(row.voiceAndSubtitles)}</td>
                </tr>`
              ).join("")}
            </tbody>
          </table>
        </div>
      </div>

      <div class="section grid-2">
        <article class="card">
          <h2>Formatação para post no Telegram</h2>
          <pre id="tpl-telegram" class="code-block">${esc(TELEGRAM_POST_TEMPLATE)}</pre>
          <div class="btn-row">
            <button type="button" class="btn btn-petrol" data-copy-target="tpl-telegram">Copiar rascunho Telegram</button>
          </div>
        </article>
        <article class="card">
          <h2>Pacote manual para WhatsApp do proprietário</h2>
          <pre id="tpl-whatsapp" class="code-block">${esc(WHATSAPP_MANUAL_DIGEST_TEMPLATE)}</pre>
          <div class="btn-row">
            <button type="button" class="btn btn-petrol" data-copy-target="tpl-whatsapp">Copiar resumo manual WhatsApp</button>
          </div>
        </article>
      </div>
      <p id="copy-feedback" class="muted" aria-live="polite"></p>
    </div>
  </section>`;
}

export function renderCmsPreviewPage(root) {
  return `
  <section class="section-tight">
    <div class="container">
      ${breadcrumbsHtml(root, [{ label: "Início", href: "index.html" }, { label: "Prévia de contrato CMS (não produtivo)" }])}
      <div class="notice-box notice-box--danger">
        <strong>ÁREA NÃO PRODUTIVA — MOCKUP VISUAL DE CONTRATO EDITORIAL (NOINDEX):</strong> esta página estática apenas ilustra a fila editorial e os contratos de dados (<code>Offer</code>, <code>MediaRights</code>, <code>EditorialApproval</code> e teto de IA). Não existe autenticação aqui e nenhum endpoint de publicação foi exposto.
      </div>
      <h1>Fila de revisão e contratos de integração</h1>
      <p class="hero-lead">Visão ilustrativa do fluxo: fonte permitida → checagem de frescor (30 min) e direitos → aprovação exata por hash → publicação manual ou futura integração autenticada.</p>

      <div class="section">
        <h2>Lote real de testes — Mercado Livre Afiliados (metadados do operador)</h2>
        <p class="muted">Taxas de comissão estimadas e códigos internos ficam restritos a esta prévia de contrato editorial; links públicos usam a URL exata <code>meli.la</code> com <code>rel="sponsored noopener"</code>.</p>
        <div class="table-wrap table-stack">
          <table>
            <thead>
              <tr>
                <th scope="col">ID ML</th>
                <th scope="col">Produto</th>
                <th scope="col">Vendedor</th>
                <th scope="col">Preço</th>
                <th scope="col">Comissão hub</th>
                <th scope="col">Share ID</th>
                <th scope="col">Link oficial</th>
              </tr>
            </thead>
            <tbody>
              ${OFFERS.filter((o) => !o.isDemoFixture)
                .map(
                  (o) => `
                  <tr>
                    <td class="mono" data-label="ID ML">${esc(o.productId || o.id)}</td>
                    <td data-label="Produto">${esc(o.title)}</td>
                    <td data-label="Vendedor">${esc(o.sellerName || o.merchant)}</td>
                    <td data-label="Preço">${esc(formatBrl(o.priceBrlCents))}</td>
                    <td class="mono" data-label="Comissão hub">${esc(o.commissionRate || "-")}</td>
                    <td class="mono" data-label="Share ID">${esc(o.shareId || "-")}</td>
                    <td class="mono" data-label="Link oficial">${esc(o.officialAffiliateUrl || "-")}</td>
                  </tr>`
                )
                .join("")}
            </tbody>
          </table>
        </div>
      </div>

      <div class="section">
        <h2>Registros de aprovação (EditorialApproval)</h2>
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Revisão</th>
                <th>Alvo</th>
                <th>Status</th>
                <th>Bloqueadores atuais</th>
              </tr>
            </thead>
            <tbody>
              ${EDITORIAL_APPROVALS.map(
                (ap) => `
                <tr>
                  <td class="mono">${esc(ap.revision)}</td>
                  <td class="mono">${esc(ap.targetType)} · ${esc(ap.targetId)}</td>
                  <td><strong>${esc(ap.status)}</strong></td>
                  <td>${ap.blockers.map((b) => `<div>• ${esc(b)}</div>`).join("")}</td>
                </tr>`
              ).join("")}
            </tbody>
          </table>
        </div>
      </div>

      <div class="section grid-2">
        <article class="card">
          <h2>Teto mensal de IA planejado</h2>
          <ul>
            <li><strong>Teto combinado:</strong> US$ ${esc(AI_BUDGET_POLICY.monthlyCeilingUsd)}/mês</li>
            <li><strong>Extra Usage:</strong> ${AI_BUDGET_POLICY.extraUsageDisabled ? "Desligado (sem cobrança extra automática)" : "Ativo"}</li>
            <li><strong>Limite diário de chamadas:</strong> Sem teto diário fixo; sujeito ao teto monetário mensal</li>
            <li><strong>Rotas autorizadas:</strong> ${esc(AI_BUDGET_POLICY.allowedRoutes.join(" | "))}</li>
            <li><strong>Reconciliação:</strong> Pausa novas chamadas se a telemetria de uso estiver desconhecida</li>
          </ul>
        </article>
        <article class="card">
          <h2>Configuração do Google Search Console (nota ao proprietário)</h2>
          <ul>
            <li>Nenhuma propriedade conectada nem token de verificação foi inserido neste protótipo.</li>
            <li>Antes de publicar em <code>procurali.com.br</code>, remova a meta tag global de staging <code>noindex</code> nas páginas públicas e mantenha <code>/cms-preview/</code> protegido e fora do índice.</li>
            <li>Verifique a propriedade do domínio no Search Console após configurar DNS/TLS e envie <code>sitemap.xml</code>.</li>
          </ul>
        </article>
      </div>
    </div>
  </section>`;
}

export function renderNotFoundPage(root) {
  return `
  <section class="section-tight">
    <div class="container prose">
      <span class="kicker">Erro 404</span>
      <h1>Página não encontrada</h1>
      <p class="hero-lead">O endereço acessado não existe ou foi movido neste protótipo do Procurali.</p>
      <div class="btn-row">
        <a class="btn btn-primary" href="${esc(root)}index.html">Voltar para a página inicial</a>
        <a class="btn btn-secondary" href="${esc(root)}ofertas/index.html">Explorar ofertas (demo)</a>
        <a class="btn btn-secondary" href="${esc(root)}guias/index.html">Ver guias de compra</a>
      </div>
    </div>
  </section>`;
}
