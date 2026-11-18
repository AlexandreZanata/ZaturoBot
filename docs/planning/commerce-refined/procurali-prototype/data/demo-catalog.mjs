/**
 * Procurali — Catalog Data & Contract Fixtures (v0.2.0-prototype)
 *
 * IMPORTANT:
 * - Demo fixtures carry `isDemoFixture: true` with `officialAffiliateUrl: null`
 *   to validate stale/expired/missing-rights guardrails without real purchases.
 * - Four real Mercado Livre affiliate hardware/peripheral test promotions
 *   (`isDemoFixture: false`, imported from `./mercadolivre-hardware-offers.mjs`)
 *   supply live test links (`https://meli.la/...`), specs, variants, and photos.
 * - WhatsApp group invite URLs remain `null` until supplied by the owner.
 * - Real external community URL: https://t.me/procurali
 */

import {
  ML_CATEGORY,
  ML_DAILY_SHOWCASE_CARDS,
  ML_EDITORIAL_APPROVALS,
  ML_EVIDENCE_SOURCES,
  ML_MEDIA_RIGHTS,
  ML_OFFERS,
} from "./mercadolivre-hardware-offers.mjs";

export const SITE_META = {
  brandSignature: "Procura Ali",
  wordmark: "procurali",
  proposedTagline: "Achados que fazem sentido.",
  taglineProposalNote:
    "Proposta de assinatura e slogan para aprovação do proprietário.",
  domain: "procurali.com.br",
  telegramUrl: "https://t.me/procurali",
  telegramHandle: "@procurali",
  botIdentity: "@ProcuraAliBot",
  prototypeReferenceTimeUtc: "2026-11-15T18:32:00Z",
  prototypeReferenceTimeLabelBrt: "15/11/2026 às 14:32 (horário de Cuiabá/AMT)",
  affiliateDisclosure: "Link de afiliado: podemos receber comissão se você comprar.",
  priceChangeNotice:
    "Preço e disponibilidade podem mudar. Confirme as condições na loja.",
  isDemoStaging: false,
};

export const AI_BUDGET_POLICY = {
  monthlyCeilingUsd: 20,
  extraUsageDisabled: true,
  dailyCallLimit: null,
  allowedRoutes: [
    "DeepSeek Flash 4.1 (ID exato do provedor pendente de confirmação)",
    "Muse Spark 1.3 contributor (ID exato do provedor pendente de confirmação)",
  ],
  pauseOnUnknownTelemetry: true,
  budgetMonthTimezone: "America/Cuiaba",
};

export const CATEGORIES = [
  ML_CATEGORY,
  {
    slug: "ergonomia-e-suportes",
    name: "Ergonomia e suportes",
    description:
      "Suportes articulados, elevadores de notebook e apoios pensados para ajustar a altura de tela em mesas compactas ou home office.",
    editableByOwner: true,
    intendedBudgetHint: "R$ 45 a R$ 220 (hipótese inicial editável)",
  },
  {
    slug: "energia-e-cabos",
    name: "Energia e cabos",
    description:
      "Hubs USB-C, carregadores GaN, cabos certificados e filtros de linha com proteção contra surtos (DPS) para proteger equipamentos.",
    editableByOwner: true,
    intendedBudgetHint: "R$ 35 a R$ 280 (hipótese inicial editável)",
  },
  {
    slug: "iluminacao-de-bancada",
    name: "Iluminação de bancada",
    description:
      "Barras de LED assimétricas para monitor e luminárias de tarefa que reduzem fadiga visual sem gerar reflexo direto na tela.",
    editableByOwner: true,
    intendedBudgetHint: "R$ 80 a R$ 260 (hipótese inicial editável)",
  },
  {
    slug: "organizacao-de-mesa",
    name: "Organização de mesa",
    description:
      "Calhas sob o tampo, bandejas metálicas e acessórios modulares para tirar fontes e fios do chão e liberar área útil.",
    editableByOwner: true,
    intendedBudgetHint: "R$ 30 a R$ 190 (hipótese inicial editável)",
  },
];

export const EVIDENCE_SOURCES = [
  ...ML_EVIDENCE_SOURCES,
  {
    id: "ev-demo-stand-spec",
    sourceType: "manufacturer_spec",
    title: "Ficha técnica ilustrativa — Suporte Articulado Alumínio V2",
    publisher: "Fabricante Exemplo (fixture de demonstração)",
    sourceUrl: null,
    retrievedAt: "2026-11-15T18:10:00Z",
    verifiedFacts: [
      "Dimensões fechadas ilustrativas: 26 × 22 × 4,5 cm; massa aproximada: 780 g.",
      "Carga declarada no exemplo: até 4 kg; compatibilidade sugerida para telas de 11\" a 16\".",
      "Abertura na base permite passagem de ar sob o chassi do notebook.",
    ],
  },
  {
    id: "ev-demo-hub-spec",
    sourceType: "manufacturer_spec",
    title: "Especificação ilustrativa — Hub USB-C 7 em 1 PD 100W",
    publisher: "Fabricante Exemplo (fixture de demonstração)",
    sourceUrl: null,
    retrievedAt: "2026-11-15T18:20:00Z",
    verifiedFacts: [
      "Entrada pass-through USB-C PD declarada até 100W (com reserva de ~15W para o próprio hub).",
      "Porta HDMI especificada para até 4K a 30 Hz em notebooks com USB-C DisplayPort Alt Mode.",
      "Leitor SD/microSD e duas portas USB-A 3.0 para periféricos.",
    ],
  },
  {
    id: "ev-demo-lamp-spec",
    sourceType: "owner_export",
    title: "Registro ilustrativo — Luminária Barra Assimétrica 40 cm",
    publisher: "Exportação demonstrativa de catálogo",
    sourceUrl: null,
    retrievedAt: "2026-11-15T18:12:00Z",
    verifiedFacts: [
      "Largura da barra: 40 cm; alimentação via cabo USB-A/USB-C 5V 1A.",
      "Clipe contrapeso projetado para monitores planos com borda superior entre 0,8 cm e 3,2 cm.",
      "Cupom demonstrativo condicionado a pedido mínimo e elegibilidade da conta.",
    ],
  },
  {
    id: "ev-demo-dps-spec",
    sourceType: "public_docs",
    title: "Referência ilustrativa — Filtro de Linha DPS 6 Tomadas (registro antigo)",
    publisher: "Documentação técnica demonstrativa",
    sourceUrl: null,
    retrievedAt: "2026-11-15T16:10:00Z",
    verifiedFacts: [
      "Padrão NBR 14136 2P+T (10A), disjuntor rearmável e proteção varistor.",
      "Registro propositalmente envelhecido (>30 min) para validar bloqueio de preço desatualizado.",
    ],
  },
  {
    id: "ev-demo-tray-spec",
    sourceType: "owner_export",
    title: "Registro histórico encerrado — Bandeja Organizadora Sob Mesa",
    publisher: "Exportação demonstrativa de catálogo",
    sourceUrl: null,
    retrievedAt: "2026-11-08T15:00:00Z",
    verifiedFacts: [
      "Aço carbono pintado com fixação por morsa sem perfuração (tampo entre 1,2 cm e 4,5 cm).",
      "Registro expirado intencionalmente para demonstrar histórico separado de oferta ativa.",
    ],
  },
  {
    id: "ev-demo-gan-spec",
    sourceType: "manufacturer_spec",
    title: "Ficha ilustrativa — Carregador GaN 65W 3 Portas",
    publisher: "Fabricante Exemplo (fixture de demonstração)",
    sourceUrl: null,
    retrievedAt: "2026-11-15T18:15:00Z",
    verifiedFacts: [
      "Potência máxima combinada de 65W; ao usar duas portas USB-C juntas, divide para 45W + 20W.",
      "Direitos de imagem marcados como não verificados para demonstrar bloqueio editorial.",
    ],
  },
];

export const MEDIA_RIGHTS = [
  {
    id: "mr-svg-demo-stand",
    assetDescription: "Ilustração técnica vetorial original (Procurali demo) — Suporte Articulado",
    licenseType: "Original prototype technical illustration (non-photographic)",
    rightsVerified: true,
    ownerPermittedUse: "Exibição apenas como esquema ilustrativo no protótipo.",
    attributionText: "Ilustração técnica demonstrativa — não representa foto real do lote.",
  },
  {
    id: "mr-svg-demo-hub",
    assetDescription: "Ilustração técnica vetorial original (Procurali demo) — Hub USB-C",
    licenseType: "Original prototype technical illustration (non-photographic)",
    rightsVerified: true,
    ownerPermittedUse: "Exibição apenas como esquema ilustrativo no protótipo.",
    attributionText: "Ilustração técnica demonstrativa — não representa foto real do lote.",
  },
  {
    id: "mr-svg-demo-lamp",
    assetDescription: "Ilustração técnica vetorial original (Procurali demo) — Luminária Monitor",
    licenseType: "Original prototype technical illustration (non-photographic)",
    rightsVerified: true,
    ownerPermittedUse: "Exibição apenas como esquema ilustrativo no protótipo.",
    attributionText: "Ilustração técnica demonstrativa — não representa foto real do lote.",
  },
  {
    id: "mr-svg-demo-clamp",
    assetDescription: "Ilustração técnica vetorial original (Procurali demo) — Filtro DPS",
    licenseType: "Original prototype technical illustration (non-photographic)",
    rightsVerified: true,
    ownerPermittedUse: "Exibição apenas como esquema ilustrativo no protótipo.",
    attributionText: "Ilustração técnica demonstrativa — não representa foto real do lote.",
  },
  {
    id: "mr-svg-demo-organizer",
    assetDescription: "Ilustração técnica vetorial original (Procurali demo) — Bandeja Cabos",
    licenseType: "Original prototype technical illustration (non-photographic)",
    rightsVerified: true,
    ownerPermittedUse: "Exibição apenas como esquema ilustrativo no protótipo.",
    attributionText: "Ilustração técnica demonstrativa — não representa foto real do lote.",
  },
  {
    id: "mr-pending-gan",
    assetDescription: "Mídia de fabricante pendente de confirmação de direitos — Carregador GaN",
    licenseType: "Pending verification",
    rightsVerified: false,
    ownerPermittedUse: "Bloqueado até confirmação formal de licença ou autorização do programa.",
    attributionText: "Direitos de imagem pendentes — publicação comercial bloqueada.",
  },
  ...ML_MEDIA_RIGHTS,
];

export const OFFERS = [
  ...ML_OFFERS,
  {
    id: "offer-demo-001",
    slug: "suporte-articulado-notebook-aluminio-v2",
    isDemoFixture: true,
    title: "Suporte Articulado de Alumínio para Notebook",
    exactVariant: "Versão 2 · Liga de alumínio anodizado prata · Dobradiça dupla",
    categorySlug: "ergonomia-e-suportes",
    merchant: "Shopee · Loja Oficial Exemplo (Demonstração)",
    marketplaceLabel: "Shopee",
    storeShort: "Loja Oficial Exemplo",
    program: "Shopee Afiliados (inscrição em análise)",
    officialAffiliateUrl: null,
    imagePath: "/brand/templates/product-illustration-stand.svg",
    imageAlt:
      "Ilustração técnica demonstrativa em linha escura de um suporte articulado inclinado para notebook sobre fundo creme.",
    priceBrlCents: 8990,
    referencePriceCents: 10490,
    currency: "BRL",
    paymentConditions: "À vista no Pix ou cartão em até 2× sem juros na loja exemplo",
    shortCondition: "À vista no Pix ou 2× s/ juros · frete grátis ≥ R$ 29",
    shortUseLine: "Alumínio articulado em 2 eixos · eleva a tela até a linha dos olhos.",
    shortCheckedAt: "15/11 às 14h15",
    ctaDestLabel: "Ver oferta na Shopee",
    couponCondition: null,
    freightStatus: "Frete calculado por CEP na finalização da loja; não incluído neste valor.",
    observedAt: "2026-11-15T18:15:00Z",
    observedAtDisplay: "15/11/2026 às 14:15 (horário de Cuiabá)",
    validUntil: null,
    freshnessTtlMinutes: 30,
    evidenceRefs: ["ev-demo-stand-spec"],
    mediaRightsRef: "mr-svg-demo-stand",
    state: "approved",
    approvalRevision: "rev-demo-20261115-01",
    oneLineReason:
      "Eleva a tela até a linha dos olhos com ajuste de ângulo e recorte para ventilação em mesas fixas.",
    forWhomItMakesSense:
      "Quem trabalha várias horas por dia em mesa fixa ou home office usando notebook de até 16 polegadas em conjunto com teclado e mouse externos.",
    supportedBenefits: [
      "Ajuste duplo de altura e inclinação ajuda a alinhar a tela sem empilhar livros.",
      "Base vazada mantém circulação de ar sob notebooks que aquecem em chamadas ou compilação.",
      "Apoios em silicone evitam riscar o chassi e reduzem deslizamento na mesa.",
    ],
    limitations: [
      "Com cerca de 780 g, é menos prático para carregar todo dia na mochila do que suportes dobráveis simples.",
      "Não é confortável digitar diretamente no teclado do notebook quando elevado; exige teclado e mouse separados.",
      "Parafusos podem pedir reaperto periódico com chave Allen após meses de ajustes frequentes.",
    ],
    unknowns: [
      "Prazo e valor de frete variam conforme o CEP e regras de envio da conta na Shopee.",
      "Rigidez da dobradiça após uso prolongado depende do lote real — dado não testado fisicamente nesta pesquisa de bancada.",
    ],
    tags: ["ergonomia", "notebook", "mesa", "home office", "postura", "aluminio"],
    hasVerifiedSevenDaySample: true,
    sevenDayCoverageNote:
      "Amostra demonstrativa de 7 dias corridos (09/11 a 15/11/2026) na mesma variante V2 Prata, mesmo vendedor exemplo e pagamento no Pix, sem frete incluso.",
    priceObservations: [
      {
        id: "obs-001-d7",
        offerId: "offer-demo-001",
        variantId: "v2-prata",
        sellerName: "Loja Oficial Exemplo",
        paymentBasis: "À vista no Pix",
        freightNote: "Frete não incluído",
        priceBrlCents: 8990,
        observedAt: "2026-11-15T18:15:00Z",
        evidenceId: "ev-demo-stand-spec",
      },
      {
        id: "obs-001-d6",
        offerId: "offer-demo-001",
        variantId: "v2-prata",
        sellerName: "Loja Oficial Exemplo",
        paymentBasis: "À vista no Pix",
        freightNote: "Frete não incluído",
        priceBrlCents: 9490,
        observedAt: "2026-11-14T16:00:00Z",
        evidenceId: "ev-demo-stand-spec",
      },
      {
        id: "obs-001-d5",
        offerId: "offer-demo-001",
        variantId: "v2-prata",
        sellerName: "Loja Oficial Exemplo",
        paymentBasis: "À vista no Pix",
        freightNote: "Frete não incluído",
        priceBrlCents: 9790,
        observedAt: "2026-11-13T15:30:00Z",
        evidenceId: "ev-demo-stand-spec",
      },
      {
        id: "obs-001-d4",
        offerId: "offer-demo-001",
        variantId: "v2-prata",
        sellerName: "Loja Oficial Exemplo",
        paymentBasis: "À vista no Pix",
        freightNote: "Frete não incluído",
        priceBrlCents: 9990,
        observedAt: "2026-11-12T17:10:00Z",
        evidenceId: "ev-demo-stand-spec",
      },
      {
        id: "obs-001-d3",
        offerId: "offer-demo-001",
        variantId: "v2-prata",
        sellerName: "Loja Oficial Exemplo",
        paymentBasis: "À vista no Pix",
        freightNote: "Frete não incluído",
        priceBrlCents: 10200,
        observedAt: "2026-11-11T14:40:00Z",
        evidenceId: "ev-demo-stand-spec",
      },
      {
        id: "obs-001-d2",
        offerId: "offer-demo-001",
        variantId: "v2-prata",
        sellerName: "Loja Oficial Exemplo",
        paymentBasis: "À vista no Pix",
        freightNote: "Frete não incluído",
        priceBrlCents: 9990,
        observedAt: "2026-11-10T15:00:00Z",
        evidenceId: "ev-demo-stand-spec",
      },
      {
        id: "obs-001-d1",
        offerId: "offer-demo-001",
        variantId: "v2-prata",
        sellerName: "Loja Oficial Exemplo",
        paymentBasis: "À vista no Pix",
        freightNote: "Frete não incluído",
        priceBrlCents: 10490,
        observedAt: "2026-11-09T14:00:00Z",
        evidenceId: "ev-demo-stand-spec",
      },
    ],
    alternativeOfferSlugs: [
      "luminaria-barra-monitor-assimetrica-40cm",
      "hub-usb-c-7-em-1-pd-100w-cinza",
    ],
  },
  {
    id: "offer-demo-002",
    slug: "hub-usb-c-7-em-1-pd-100w-cinza",
    isDemoFixture: true,
    title: "Hub USB-C 7 em 1 com HDMI 4K e Power Delivery 100W",
    exactVariant: "Cor Cinza Espacial · 7 portas (HDMI + 2× USB-A + SD/TF + USB-C PD + dados)",
    categorySlug: "energia-e-cabos",
    merchant: "Mercado Livre · Distribuidor Tech Exemplo (Demonstração)",
    marketplaceLabel: "Mercado Livre",
    storeShort: "Distribuidor Tech Exemplo",
    program: "Programa de Afiliados (inscrição em análise)",
    officialAffiliateUrl: null,
    imagePath: "/brand/templates/product-illustration-hub.svg",
    imageAlt:
      "Ilustração técnica demonstrativa de hub USB-C cinza com duas portas USB-A, saída HDMI 4K e entrada PD 100W.",
    priceBrlCents: 13990,
    currency: "BRL",
    paymentConditions: "À vista no Pix ou boleto na loja exemplo",
    shortCondition: "À vista no Pix ou boleto · frete grátis a partir de R$ 79",
    shortUseLine: "HDMI 4K, energia PD 100W e USB 3.0 em um único cabo na mesa.",
    shortCheckedAt: "15/11 às 14h20",
    ctaDestLabel: "Ver oferta no Mercado Livre",
    couponCondition: null,
    freightStatus: "Frete não verificado para seu CEP; confira na etapa de envio.",
    observedAt: "2026-11-15T18:20:00Z",
    observedAtDisplay: "15/11/2026 às 14:20 (horário de Cuiabá)",
    validUntil: null,
    freshnessTtlMinutes: 30,
    evidenceRefs: ["ev-demo-hub-spec"],
    mediaRightsRef: "mr-svg-demo-hub",
    state: "approved",
    approvalRevision: "rev-demo-20261115-02",
    oneLineReason:
      "Reúne saída de vídeo HDMI, leitura de cartão SD e passagem de carregamento em uma porta USB-C.",
    forWhomItMakesSense:
      "Quem usa notebook fino com poucas portas e quer conectar monitor externo, mouse com receptor USB-A e carregador sem ficar trocando cabos toda hora.",
    supportedBenefits: [
      "Pass-through Power Delivery declarado até 100W (aproximadamente 85W úteis ao notebook após reserva do hub).",
      "Duas portas USB-A 3.0 permitem manter teclado/mouse ou pendrive conectados simultaneamente.",
      "Carcaça em alumínio ajuda na dissipação passiva de calor em uso contínuo.",
    ],
    limitations: [
      "A saída HDMI só funciona se a porta USB-C do seu notebook suportar vídeo (DisplayPort Alt Mode).",
      "Limite típico de 4K a 30 Hz nessa faixa de preço; quem precisa de 4K a 60 Hz deve buscar modelo específico.",
      "Não inclui porta Ethernet RJ45 nem carregador de parede.",
    ],
    unknowns: [
      "Sem amostra de 7 dias nesta demonstração: exibindo apenas a observação datada mais recente.",
      "Aquecimento real sob carregamento simultâneo de 85W + leitura de cartão depende de avaliação prática.",
    ],
    tags: ["usb-c", "hub", "hdmi", "notebook", "adaptador", "energia", "pd"],
    hasVerifiedSevenDaySample: false,
    sevenDayCoverageNote:
      "Apenas 1 observação datada neste registro demonstrativo — não permite afirmar menor preço da semana.",
    priceObservations: [
      {
        id: "obs-002-d1",
        offerId: "offer-demo-002",
        variantId: "7em1-cinza",
        sellerName: "Distribuidor Tech Exemplo",
        paymentBasis: "À vista no Pix",
        freightNote: "Frete não incluído",
        priceBrlCents: 13990,
        observedAt: "2026-11-15T18:20:00Z",
        evidenceId: "ev-demo-hub-spec",
      },
    ],
    alternativeOfferSlugs: [
      "suporte-articulado-notebook-aluminio-v2",
      "carregador-gan-65w-3-portas-branco",
    ],
  },
  {
    id: "offer-demo-003",
    slug: "luminaria-barra-monitor-assimetrica-40cm",
    isDemoFixture: true,
    title: "Luminária Barra de Monitor com Ótica Assimétrica 40 cm",
    exactVariant: "Modelo 40 cm · USB 5V · Controle por toque (3 temperaturas de cor)",
    categorySlug: "iluminacao-de-bancada",
    merchant: "Shopee · Casa & Mesa Exemplo (Demonstração)",
    marketplaceLabel: "Shopee",
    storeShort: "Casa & Mesa Exemplo",
    program: "Shopee Afiliados (inscrição em análise)",
    officialAffiliateUrl: null,
    imagePath: "/brand/templates/product-illustration-lamp.svg",
    imageAlt:
      "Ilustração técnica demonstrativa de barra de luz posicionada sobre o topo do monitor projetando feixe direcionado para a mesa.",
    priceBrlCents: 16900,
    referencePriceCents: 18400,
    currency: "BRL",
    paymentConditions: "Valor com cupom demonstrativo da loja para pedidos acima de R$ 150",
    shortCondition: "Com cupom LUZ15 (−R$ 15 em pedidos ≥ R$ 150)",
    shortUseLine: "Luz assimétrica sem reflexo na tela com dimmer contínuo.",
    shortCheckedAt: "15/11 às 14h12",
    ctaDestLabel: "Ver oferta na Shopee",
    couponCode: "LUZ15",
    validUntilDisplay: "Cupom válido até 23h59 de 15/11 · horário de Brasília",
    couponCondition:
      "Cupom exemplo da loja (R$ 15 de abatimento) sujeito a elegibilidade da conta, estoque de cupons e regras da Shopee.",
    freightStatus: "Frete e eventual cupom de frete variam por conta e região.",
    observedAt: "2026-11-15T18:12:00Z",
    observedAtDisplay: "15/11/2026 às 14:12 (horário de Cuiabá)",
    validUntil: "2026-11-16T02:59:00Z",
    freshnessTtlMinutes: 30,
    evidenceRefs: ["ev-demo-lamp-spec"],
    mediaRightsRef: "mr-svg-demo-lamp",
    state: "approved",
    approvalRevision: "rev-demo-20261115-03",
    oneLineReason:
      "Ilumina teclado e caderno à noite sem ocupar espaço na mesa nem criar reflexo direto no monitor plano.",
    forWhomItMakesSense:
      "Quem trabalha ou estuda à noite em mesa pequena e quer iluminar a superfície de trabalho sem luminária de base ocupando espaço.",
    supportedBenefits: [
      "Feixe inclinado para a frente reduz brilho direto na tela em comparação com abajures comuns.",
      "Alimentação via USB 5V permite ligar na própria porta do monitor ou em adaptador simples.",
      "Três faixas de temperatura de cor (quente, neutra e fria) para ajustar conforme o horário.",
    ],
    limitations: [
      "Clipe contrapeso funciona melhor em monitores planos com borda de 0,8 cm a 3,2 cm; telas muito curvas ou com traseira arredondada podem ficar instáveis.",
      "Não substitui iluminação geral do cômodo e não inclui adaptador de tomada.",
      "Sem cupom elegível na conta, o valor na loja sobe para a faixa sem desconto (R$ 184,00 no exemplo).",
    ],
    unknowns: [
      "Elegibilidade exata do cupom depende da conta logada no momento da compra.",
      "Índice de reprodução de cor (IRC/CRI) real não foi aferido em laboratório nesta pesquisa de bancada.",
    ],
    tags: ["iluminacao", "luminaria", "monitor", "mesa", "cupom", "home office"],
    hasVerifiedSevenDaySample: false,
    sevenDayCoverageNote:
      "Observação única com condição de cupom informada; sem série histórica de 7 dias.",
    priceObservations: [
      {
        id: "obs-003-d1",
        offerId: "offer-demo-003",
        variantId: "40cm-preta-usb",
        sellerName: "Casa & Mesa Exemplo",
        paymentBasis: "Com cupom condicional da loja",
        freightNote: "Frete e cupom variam por conta",
        priceBrlCents: 16900,
        observedAt: "2026-11-15T18:12:00Z",
        evidenceId: "ev-demo-lamp-spec",
      },
    ],
    alternativeOfferSlugs: [
      "suporte-articulado-notebook-aluminio-v2",
      "filtro-de-linha-dps-6-tomadas-cabo-2m",
    ],
  },
  {
    id: "offer-demo-004",
    slug: "filtro-de-linha-dps-6-tomadas-cabo-2m",
    isDemoFixture: true,
    title: "Filtro de Linha com DPS 6 Tomadas e Cabo de 2 m",
    exactVariant: "6 tomadas espaçadas NBR 14136 · Cabo 2 m · Preto",
    categorySlug: "energia-e-cabos",
    merchant: "Mercado Livre · Elétrica Segura Exemplo (Demonstração)",
    marketplaceLabel: "Mercado Livre",
    storeShort: "Elétrica Segura Exemplo",
    program: "Programa de Afiliados (inscrição em análise)",
    officialAffiliateUrl: null,
    imagePath: "/brand/templates/product-illustration-clamp.svg",
    imageAlt:
      "Ilustração técnica demonstrativa de régua de 6 tomadas com disjuntor rearmável e proteção DPS.",
    priceBrlCents: 6490,
    currency: "BRL",
    paymentConditions: "Registro fora da janela de 30 min — preço em atualização antes de exibir como oferta ativa",
    shortCondition: "Pix ou boleto · proteção DPS 12 kA verificada antes da pausa",
    shortUseLine: "Proteção contra surtos elétricos com disjuntor e cabo 2 m.",
    shortCheckedAt: "15/11 às 12h10 · >2h",
    ctaDestLabel: "Conferir status do item",
    couponCondition: null,
    freightStatus: "Desconhecido (checagem expirada há mais de 30 minutos).",
    observedAt: "2026-11-15T16:10:00Z",
    observedAtDisplay: "15/11/2026 às 12:10 (há mais de 2 horas — fora da janela de 30 min)",
    validUntil: null,
    freshnessTtlMinutes: 30,
    evidenceRefs: ["ev-demo-dps-spec"],
    mediaRightsRef: "mr-svg-demo-clamp",
    state: "stale",
    approvalRevision: "rev-demo-20261115-04-stale",
    oneLineReason:
      "Protege bancada com 6 tomadas afastadas e disjuntor, mas este registro passou do limite de 30 minutos de frescor.",
    forWhomItMakesSense:
      "Quem precisa ligar monitor, carregador de notebook e luminária com proteção básica contra surtos, após nova verificação de preço.",
    supportedBenefits: [
      "Espaçamento maior entre tomadas reduz bloqueio por fontes volumosas.",
      "Disjuntor rearmável evita troca de fusível após sobrecarga simples.",
    ],
    limitations: [
      "ESTADO DESATUALIZADO (STALE): a última observação ocorreu há mais de 30 minutos e a ação de compra foi bloqueada pela política de frescor.",
      "Capacidade padrão 10A (1270W em 127V / 2200W em 220V): não indicado para equipamentos térmicos pesados.",
    ],
    unknowns: [
      "Preço atual na loja após 12:10 (horário de Cuiabá) não revalidado.",
      "Existência de aterramento efetivo na tomada da parede do imóvel do comprador.",
    ],
    tags: ["dps", "filtro de linha", "energia", "tomadas", "protecao"],
    hasVerifiedSevenDaySample: false,
    sevenDayCoverageNote: "Registro propositalmente desatualizado para teste do estado stale.",
    priceObservations: [
      {
        id: "obs-004-d1",
        offerId: "offer-demo-004",
        variantId: "6-tomadas-2m",
        sellerName: "Elétrica Segura Exemplo",
        paymentBasis: "À vista (não revalidado)",
        freightNote: "Não verificado",
        priceBrlCents: 6490,
        observedAt: "2026-11-15T16:10:00Z",
        evidenceId: "ev-demo-dps-spec",
      },
    ],
    alternativeOfferSlugs: ["hub-usb-c-7-em-1-pd-100w-cinza"],
  },
  {
    id: "offer-demo-005",
    slug: "organizador-cabos-bandeja-sob-mesa-aco",
    isDemoFixture: true,
    title: "Bandeja Organizadora de Cabos em Aço Sob a Mesa",
    exactVariant: "40 cm · Fixação por morsa sem furo · Aço preto fosco",
    categorySlug: "organizacao-de-mesa",
    merchant: "Mesa Prática Exemplo (Shopee — Demonstração)",
    program: "Shopee Afiliados (inscrição em análise)",
    officialAffiliateUrl: null,
    imagePath: "/brand/templates/product-illustration-organizer.svg",
    imageAlt:
      "Ilustração técnica demonstrativa de bandeja metálica presa por grampos sob o tampo da mesa.",
    priceBrlCents: 11200,
    currency: "BRL",
    paymentConditions: "Registro histórico encerrado (prazo expirado em 08/11/2026)",
    couponCondition: null,
    freightStatus: "Indisponível — oferta encerrada.",
    observedAt: "2026-11-08T15:00:00Z",
    observedAtDisplay: "08/11/2026 às 11:00 (histórico expirado)",
    validUntil: "2026-11-10T23:59:00Z",
    freshnessTtlMinutes: 30,
    evidenceRefs: ["ev-demo-tray-spec"],
    mediaRightsRef: "mr-svg-demo-organizer",
    state: "expired",
    approvalRevision: "rev-demo-20261108-expired",
    oneLineReason:
      "Exemplo de oferta expirada mantida apenas como referência histórica separada das ofertas ativas.",
    forWhomItMakesSense:
      "Quem busca organizar filtros de linha e fontes sob mesas alugadas sem furar o tampo, quando houver novo lote ativo.",
    supportedBenefits: [
      "Instalação por morsa evita furos em tampos de MDF ou madeira.",
      "Mantém régua e excesso de cabos suspensos longe da poeira e da vassoura.",
    ],
    limitations: [
      "OFERTA EXPIRADA: este registro não está ativo para compra e permanece separado da vitrine vigente.",
      "Não encaixa em mesas com saia/travessa estrutural colada na borda traseira.",
    ],
    unknowns: [
      "Disponibilidade atual ou novo preço do vendedor após o término da campanha em 10/11/2026.",
    ],
    tags: ["cabos", "bandeja", "organizacao", "mesa", "morsa"],
    hasVerifiedSevenDaySample: false,
    sevenDayCoverageNote: "Preço histórico encerrado; separado de ofertas vigentes.",
    priceObservations: [
      {
        id: "obs-005-d1",
        offerId: "offer-demo-005",
        variantId: "40cm-preto",
        sellerName: "Mesa Prática Exemplo",
        paymentBasis: "Registro histórico expirado",
        freightNote: "Encerrado",
        priceBrlCents: 11200,
        observedAt: "2026-11-08T15:00:00Z",
        evidenceId: "ev-demo-tray-spec",
      },
    ],
    alternativeOfferSlugs: ["suporte-articulado-notebook-aluminio-v2"],
  },
  {
    id: "offer-demo-006",
    slug: "carregador-gan-65w-3-portas-branco",
    isDemoFixture: true,
    title: "Carregador GaN 65W 3 Portas (2× USB-C + 1× USB-A)",
    exactVariant: "65W · Plugue padrão brasileiro NBR 14136 · Branco",
    categorySlug: "energia-e-cabos",
    merchant: "Energia Compacta Exemplo (Shopee — Demonstração)",
    program: "Shopee Afiliados (inscrição em análise)",
    officialAffiliateUrl: null,
    imagePath: "/brand/templates/product-illustration-charger.svg",
    imageAlt:
      "Ilustração técnica demonstrativa de carregador compacto com pinos padrão brasileiro, duas portas USB-C e uma USB-A.",
    priceBrlCents: 15490,
    currency: "BRL",
    paymentConditions: "Bloqueado editorialmente: direitos de imagem do lote real não confirmados",
    couponCondition: null,
    freightStatus: "Não aplicável enquanto direitos de mídia estiverem pendentes.",
    observedAt: "2026-11-15T18:15:00Z",
    observedAtDisplay: "15/11/2026 às 14:15 (horário de Cuiabá)",
    validUntil: null,
    freshnessTtlMinutes: 30,
    evidenceRefs: ["ev-demo-gan-spec"],
    mediaRightsRef: "mr-pending-gan",
    state: "missing_rights",
    approvalRevision: "rev-demo-20261115-blocked-rights",
    oneLineReason:
      "Exemplo de bloqueio por direitos de imagem pendentes: mesmo com preço recente, não libera CTA de compra.",
    forWhomItMakesSense:
      "Quem precisa reduzir carregadores na mochila com uma única fonte de 65W — sujeito à liberação de mídia licenciada.",
    supportedBenefits: [
      "Substitui carregadores separados quando o notebook aceita recarga via USB-C PD até 65W.",
      "Plugue padrão brasileiro dispensa adaptadores frouxos na tomada.",
    ],
    limitations: [
      "DIREITOS DE MÍDIA PENDENTES: a publicação e o link comercial ficam bloqueados até verificação de licença de imagem.",
      "Potência de 65W vale para uso individual na porta USB-C1; conectando dois aparelhos, divide para 45W + 20W.",
    ],
    unknowns: [
      "Autorização formal para uso de imagens oficiais do fabricante ou do programa de afiliados.",
    ],
    tags: ["gan", "carregador", "65w", "usb-c", "energia"],
    hasVerifiedSevenDaySample: false,
    sevenDayCoverageNote: "Bloqueado por ausência de verificação de direitos de mídia.",
    priceObservations: [
      {
        id: "obs-006-d1",
        offerId: "offer-demo-006",
        variantId: "65w-branco-nbr",
        sellerName: "Energia Compacta Exemplo",
        paymentBasis: "Referência bloqueada",
        freightNote: "Bloqueado",
        priceBrlCents: 15490,
        observedAt: "2026-11-15T18:15:00Z",
        evidenceId: "ev-demo-gan-spec",
      },
    ],
    alternativeOfferSlugs: ["hub-usb-c-7-em-1-pd-100w-cinza"],
  },
];

export const GUIDES = [
  {
    id: "guide-001",
    slug: "como-escolher-suporte-para-notebook-e-monitor",
    isDemoFixture: true,
    title: "Como escolher suporte para notebook sem errar na altura e no peso",
    metaDescription:
      "Critérios práticos de ergonomia, peso máximo e contrapontos entre suporte dobrável e articulado antes de comprar. Pesquisa de bancada em pt-BR.",
    categorySlug: "ergonomia-e-suportes",
    summary:
      "Antes de comprar pelo menor preço, verifique se a dobradiça aguenta o peso do seu aparelho, se há recorte de ventilação e se você já tem teclado e mouse externos.",
    methodologyLabel: "Pesquisa de bancada (desk research — sem testes físicos de laboratório)",
    author: "Equipe Editorial Procurali (protótipo)",
    reviewer: "Revisão editorial pendente de proprietário",
    publishedAt: "2026-11-10",
    updatedAt: "2026-11-15",
    updateHistory: [
      "15/11/2026: Adicionada tabela comparativa entre suporte dobrável leve e articulado de alumínio.",
      "10/11/2026: Publicação inicial da estrutura de guia demonstrativo.",
    ],
    tableOfContents: [
      { id: "resposta-direta", label: "Resposta direta" },
      { id: "criterios", label: "Quatro critérios objetivos" },
      { id: "comparacao", label: "Comparação por cenário de uso" },
      { id: "limites", label: "Quando não comprar" },
      { id: "fontes", label: "Fontes e transparência" },
    ],
    sections: [
      {
        id: "resposta-direta",
        heading: "Resposta direta",
        paragraphs: [
          "Se você trabalha em uma mesa fixa por mais de duas horas seguidas, um suporte articulado mais pesado (como modelos em liga de alumínio com base larga) costuma oferecer altura maior para aproximar a tela da linha dos olhos. Já quem leva o notebook para fora de casa todos os dias tende a aproveitar melhor um modelo dobrável fino de até 250 g, aceitando menor elevação.",
          "Importante: esta análise é baseada em pesquisa de bancada (especificações de fabricantes e critérios ergonômicos conhecidos). Não realizamos testes mecânicos destrutivos nem medição de durabilidade em laboratório.",
        ],
      },
      {
        id: "criterios",
        heading: "Quatro critérios objetivos para checar no anúncio",
        paragraphs: [
          "1. Peso real do notebook versus carga declarada: procure margem de folga, especialmente em notebooks gamer ou corporativos acima de 2 kg, porque a força ao apoiar os braços ou ajustar a tela multiplica o esforço na dobradiça.",
          "2. Aberturas de ventilação alinhadas: muitos notebooks puxam ar frio por baixo. Bases fechadas sem recortes podem aumentar o ruído da ventoinha.",
          "3. Apoio frontal discreto: a aba que segura o aparelho não pode machucar o pulso nem bloquear a abertura da tampa.",
          "4. Uso obrigatório de teclado e mouse externos: elevar a tela acima de 10 cm e continuar digitando no teclado inclinado do notebook transfere tensão para os punhos.",
        ],
      },
      {
        id: "limites",
        heading: "Quando um suporte articulado não é a melhor saída",
        paragraphs: [
          "Se a profundidade da sua mesa for menor que 45 cm, um suporte articulado que projeta o notebook para frente pode deixar a tela perto demais dos olhos depois de colocar um teclado externo. Nesses casos, um elevador compacto ou braço articulado de monitor externo resolve melhor.",
        ],
      },
    ],
    comparisonRows: [
      {
        criterio: "Cenário ideal",
        opcaoA: "Suporte articulado alumínio (mesa fixa)",
        opcaoB: "Suporte dobrável compacto (mochila)",
      },
      {
        criterio: "Faixa típica de altura",
        opcaoA: "Elevação ampla até a linha dos olhos",
        opcaoB: "Inclinação baixa/média para postura parcial",
      },
      {
        criterio: "Portabilidade",
        opcaoA: "Baixa (~700–900 g)",
        opcaoB: "Alta (~200–300 g)",
      },
      {
        criterio: "Exige teclado externo?",
        opcaoA: "Sim, no uso elevado",
        opcaoB: "Recomendado em sessões longas",
      },
    ],
    relatedOfferSlugs: ["suporte-articulado-notebook-aluminio-v2"],
    sourceLinks: [
      {
        label: "NR-17 / Ergonomia no trabalho informatizado (Ministério do Trabalho — referência conceitual de postura)",
        url: "https://www.gov.br/trabalho-e-emprego/pt-br",
        consultedAt: "2026-11-15",
      },
    ],
    state: "approved",
    approvalRevision: "guide-rev-001",
  },
  {
    id: "guide-002",
    slug: "guia-hub-usb-c-e-carregadores-gan-sem-erro",
    isDemoFixture: true,
    title: "Hub USB-C e carregador GaN: o que verificar antes de comprar",
    metaDescription:
      "Entenda DisplayPort Alt Mode, HDMI 4K30 vs 4K60 e divisão de potência em carregadores GaN antes de escolher acessórios USB-C.",
    categorySlug: "energia-e-cabos",
    summary:
      "Nem toda porta USB-C transmite vídeo, e um carregador de 65W raramente entrega 65W em todas as portas ao mesmo tempo. Veja o checklist técnico.",
    methodologyLabel: "Pesquisa de bancada (desk research — sem medições elétricas próprias)",
    author: "Equipe Editorial Procurali (protótipo)",
    reviewer: "Revisão editorial pendente de proprietário",
    publishedAt: "2026-11-12",
    updatedAt: "2026-11-15",
    updateHistory: [
      "15/11/2026: Incluído alerta sobre divisão de watts em múltiplas portas.",
    ],
    tableOfContents: [
      { id: "resposta-direta", label: "Resumo prático" },
      { id: "compatibilidade-video", label: "Porta USB-C com vídeo e 4K30 vs 4K60" },
      { id: "divisao-potencia", label: "Potência total versus potência por porta" },
      { id: "fontes", label: "Fontes" },
    ],
    sections: [
      {
        id: "resposta-direta",
        heading: "Resumo prático",
        paragraphs: [
          "Antes de comprar um hub USB-C com HDMI, confirme no manual do seu notebook se a porta USB-C possui suporte a saída de vídeo (DisplayPort Alt Mode ou Thunderbolt). Em notebooks de entrada, algumas portas USB-C servem apenas para dados.",
          "Da mesma forma, em carregadores GaN multipontas, o número grande na caixa (como 65W) representa a soma máxima ou a potência com apenas uma porta conectada. Ao plugar celular e notebook juntos, a potência é redistribuída.",
        ],
      },
      {
        id: "compatibilidade-video",
        heading: "HDMI 4K a 30 Hz ou 60 Hz faz diferença?",
        paragraphs: [
          "Se você usa um monitor Full HD (1920×1080), praticamente todo hub HDMI 4K30 entrega 60 Hz em 1080p sem problemas. Já em monitores 4K ou ultrawide de resolução maior, um hub limitado a 4K 30 Hz deixa o movimento do cursor menos fluido. Confira essa especificação linha por linha antes de comprar.",
        ],
      },
      {
        id: "divisao-potencia",
        heading: "Atenção à divisão de watts e ao cabo utilizado",
        paragraphs: [
          "Um hub com Power Delivery costuma reservar de 10W a 15W para alimentar suas próprias portas. Logo, se você conecta um carregador de 65W no hub, chegam cerca de 50W ao notebook — o que pode ser suficiente para modelos leves, mas lento para máquinas que pedem 90W ou mais.",
        ],
      },
    ],
    comparisonRows: [
      {
        criterio: "Uso principal",
        opcaoA: "Hub USB-C 7 em 1 (conexões + passagem PD)",
        opcaoB: "Carregador GaN 65W (fonte de energia)",
      },
      {
        criterio: "Requisito crítico",
        opcaoA: "Porta USB-C com DisplayPort Alt Mode",
        opcaoB: "Cabo USB-C compatível com 60W/100W (e-Marker acima de 60W)",
      },
      {
        criterio: "Erro comum",
        opcaoA: "Esperar 4K 60 Hz em modelo 4K 30 Hz",
        opcaoB: "Somar potência total esperando 65W em duas portas",
      },
    ],
    relatedOfferSlugs: [
      "hub-usb-c-7-em-1-pd-100w-cinza",
      "carregador-gan-65w-3-portas-branco",
    ],
    sourceLinks: [
      {
        label: "USB Implementers Forum — USB Type-C & Power Delivery specifications overview",
        url: "https://www.usb.org/usb-charger-pd",
        consultedAt: "2026-11-15",
      },
    ],
    state: "approved",
    approvalRevision: "guide-rev-002",
  },
  {
    id: "guide-003",
    slug: "organizacao-de-cabos-e-energia-na-bancada",
    isDemoFixture: true,
    title: "Organização de cabos e proteção elétrica na mesa sem gastar além da conta",
    metaDescription:
      "Diferença entre extensão simples, filtro com fusível e DPS, além de cuidados com bandejas sob a mesa sem perfuração.",
    categorySlug: "organizacao-de-mesa",
    summary:
      "Arrumar os fios começa pela segurança elétrica e pela medida do tampo. Veja o que observar antes de prender réguas e bandejas sob a mesa.",
    methodologyLabel: "Pesquisa de bancada (desk research — consulte eletricista em caso de dúvida na rede)",
    author: "Equipe Editorial Procurali (protótipo)",
    reviewer: "Revisão editorial pendente de proprietário",
    publishedAt: "2026-11-13",
    updatedAt: "2026-11-15",
    updateHistory: [
      "15/11/2026: Destacada diferença entre proteção DPS e régua sem proteção.",
    ],
    tableOfContents: [
      { id: "resposta-direta", label: "O que priorizar primeiro" },
      { id: "dps-vs-extensao", label: "DPS não é extensão comum" },
      { id: "medidas-mesa", label: "Como medir a mesa para bandejas de morsa" },
      { id: "fontes", label: "Fontes" },
    ],
    sections: [
      {
        id: "resposta-direta",
        heading: "O que priorizar primeiro",
        paragraphs: [
          "Antes de comprar canaletas e velcros, resolva a alimentação elétrica: dê preferência a filtro de linha com DPS (Dispositivo de Proteção contra Surtos) certificado e disjuntor, evitando réguas simples sem proteção ou adaptadores em T ligados em série.",
        ],
      },
      {
        id: "dps-vs-extensao",
        heading: "Filtro com DPS x régua comum",
        paragraphs: [
          "Réguas baratas apenas replicam a tomada. Modelos com DPS combinam varistores e disjuntor ou fusível para ajudar a atenuar picos de tensão. Vale lembrar que a eficiência máxima contra surtos depende também das condições da instalação elétrica e do aterramento do local.",
        ],
      },
      {
        id: "medidas-mesa",
        heading: "Medidas que evitam devolução em bandejas sem furo",
        paragraphs: [
          "Meça a espessura do tampo e verifique se há barra metálica ou madeira logo abaixo da borda traseira. Muitas bandejas por morsa exigem pelo menos 3 cm a 5 cm de área plana livre para o aperto firme do grampo.",
        ],
      },
    ],
    comparisonRows: [
      {
        criterio: "Função na bancada",
        opcaoA: "Filtro de linha DPS 6 tomadas",
        opcaoB: "Bandeja sob a mesa com morsa",
      },
      {
        criterio: "Ponto de atenção",
        opcaoA: "Corrente nominal (10A) e comprimento do cabo",
        opcaoB: "Espessura e borda livre do tampo da mesa",
      },
    ],
    relatedOfferSlugs: [
      "filtro-de-linha-dps-6-tomadas-cabo-2m",
      "organizador-cabos-bandeja-sob-mesa-aco",
    ],
    sourceLinks: [
      {
        label: "INMETRO — Plugues e tomadas padrão brasileiro NBR 14136",
        url: "https://www.gov.br/inmetro/pt-br",
        consultedAt: "2026-11-15",
      },
    ],
    state: "approved",
    approvalRevision: "guide-rev-003",
  },
];

export const WEEKLY_SELECTION = {
  title: "Seleção da semana (demonstração)",
  dateRangeLabel: "Semana de 09/11/2026 a 15/11/2026",
  scopeExplanation:
    "Recorte demonstrativo focado em home office e organização de mesa até R$ 200, usando exclusivamente dados fictícios de protótipo. Não representa cobertura completa da internet nem confirmação de menor preço global.",
  picks: [
    {
      offerSlug: "suporte-articulado-notebook-aluminio-v2",
      roleLabel: "Para postura em mesa fixa",
      note: "Único item da demonstração com série fictícia de 7 dias completos na mesma variante.",
    },
    {
      offerSlug: "hub-usb-c-7-em-1-pd-100w-cinza",
      roleLabel: "Para expandir portas em notebook fino",
      note: "Observação única datada; checar suporte a vídeo na porta USB-C do notebook.",
    },
    {
      offerSlug: "luminaria-barra-monitor-assimetrica-40cm",
      roleLabel: "Para iluminar o tampo à noite",
      note: "Inclui condição de cupom sujeita à elegibilidade da conta.",
    },
  ],
  expiredHistoricalReference: {
    offerSlug: "organizador-cabos-bandeja-sob-mesa-aco",
    label: "Registro histórico encerrado (não disponível como oferta ativa)",
    note: "Mantido separado para mostrar que preços antigos não são misturados com ofertas vigentes.",
  },
};

export const COMMUNITIES = [
  {
    id: "comm-telegram-procurali",
    slug: "telegram-procurali",
    network: "telegram",
    name: "Canal oficial Procurali no Telegram",
    handle: "@procurali",
    purpose:
      "Divulgação curada de achados explicados, resumos semanais de guias e atualizações de ofertas com contexto quando ativado.",
    categoryTopic: "Geral · Organização, mesa e acessórios úteis",
    intendedFrequency: "Proposta inicial: até 2 envios por dia + 1 resumo semanal (após ativação)",
    rules: [
      "Canal público somente leitura para assinantes; sem spam de terceiros.",
      "Toda publicação com link comercial traz identificação #publi e aviso de afiliado.",
      "Você entra e sai quando quiser, sem necessidade de cadastro no site.",
    ],
    availabilityState: "active",
    inviteUrl: "https://t.me/procurali",
    ctaLabel: "Entrar no canal @procurali",
    privacyNotice:
      "O Telegram funciona sob os termos e configurações de privacidade do próprio aplicativo. Em canais públicos, outros assinantes não veem seu número de telefone se a sua privacidade de conta estiver configurada para ocultar.",
  },
  {
    id: "comm-wa-setup-mesa",
    slug: "whatsapp-organizacao-mesa",
    network: "whatsapp",
    name: "Grupo WhatsApp — Organização e Mesa Prática (prévia)",
    handle: "Link será fornecido pelo proprietário",
    purpose:
      "Seleção enxuta de acessórios de organização e ergonomia com contexto rápido de compatibilidade.",
    categoryTopic: "Casa, mesa e ergonomia (R$ 30–300)",
    intendedFrequency: "Proposta inicial: cerca de 3 resumos por semana, compartilhados manualmente pelo proprietário",
    rules: [
      "Entrada 100% voluntária por link de convite oficial do grupo criado pelo proprietário.",
      "Sem envio automático por bot ou mensagens privadas não solicitadas aos participantes.",
      "Convites podem ser rotacionados ou suspensos a qualquer momento pelo proprietário.",
    ],
    availabilityState: "pending_owner_invite",
    inviteUrl: null,
    ctaLabel: "Convite aguardando criação pelo proprietário",
    privacyNotice:
      "Atenção antes de entrar no futuro: dependendo das configurações do grupo e da sua conta no WhatsApp, entrar no grupo pode expor seu número de telefone, nome e foto de perfil aos demais participantes. O Procurali não promete anonimato em grupos de WhatsApp.",
  },
  {
    id: "comm-wa-energia-cabos",
    slug: "whatsapp-energia-cabos",
    network: "whatsapp",
    name: "Grupo WhatsApp — Energia, Cabos e Acessórios (prévia de rotação)",
    handle: "Estado demonstrativo: link em rotação",
    purpose:
      "Alertas pontuais e critérios rápidos de hubs, carregadores e proteção elétrica.",
    categoryTopic: "Energia, cabos e periféricos",
    intendedFrequency: "Proposta inicial: cerca de 2 a 3 resumos por semana",
    rules: [
      "Este cartão demonstra o estado de convite revogado ou em rotação controlada pelo proprietário.",
      "Quando um link vaza ou sofre abuso, o proprietário revoga e gera novo convite manualmente.",
    ],
    availabilityState: "revoked_rotating",
    inviteUrl: null,
    ctaLabel: "Link temporariamente revogado para rotação",
    privacyNotice:
      "Ao participar de grupos no WhatsApp no futuro, considere que telefone e perfil podem ficar visíveis aos membros conforme o tipo de grupo.",
  },
  {
    id: "comm-wa-lotado-exemplo",
    slug: "whatsapp-geral-indisponivel",
    network: "whatsapp",
    name: "Grupo WhatsApp — Turma Piloto (exemplo indisponível)",
    handle: "Estado demonstrativo: grupo fechado/indisponível",
    purpose:
      "Demonstra o comportamento visual quando um grupo atinge capacidade máxima ou é fechado pelo proprietário (sem exibir contagem falsa).",
    categoryTopic: "Geral",
    intendedFrequency: "Pausado",
    rules: [
      "Não exibimos números falsos de 'restam 2 vagas' nem contadores simulados de membros.",
      "Quando indisponível, o visitante é direcionado para o canal aberto no Telegram.",
    ],
    availabilityState: "unavailable",
    inviteUrl: null,
    ctaLabel: "Grupo indisponível no momento",
    privacyNotice:
      "Grupos encerrados ou indisponíveis não coletam lista de espera nem exportam contatos.",
  },
];

export const CREATIVE_ASSETS = [
  {
    id: "cr-feed-45",
    offerId: "offer-demo-001",
    format: "feed_1080x1350",
    title: "Card Feed Vertical 1080×1350 (4:5)",
    assetPath: "/brand/templates/feed-card-1080x1350.svg",
    disclosureText: "#publi · Link de afiliado: podemos receber comissão se você comprar.",
    mediaRightsId: "mr-svg-demo-stand",
    approvalRevision: "rev-demo-20261115-01",
    isIllustrativeDemo: true,
    notes:
      "Hierarquia determinística: badge #publi no topo, benefício específico, zona de mídia licenciada, faixa de preço e condição com horário BRT, alerta ⚠️ de compatibilidade e CTA único.",
  },
  {
    id: "cr-feed-11",
    offerId: "offer-demo-002",
    format: "feed_1080x1080",
    title: "Card Feed Quadrado 1080×1080 (1:1)",
    assetPath: "/brand/templates/feed-card-1080x1080.svg",
    disclosureText: "#publi · Link de afiliado: podemos receber comissão se você comprar.",
    mediaRightsId: "mr-svg-demo-hub",
    approvalRevision: "rev-demo-20261115-02",
    isIllustrativeDemo: true,
    notes:
      "Adaptação quadrada para reutilizar os mesmos dados de preço e condição sem criar cópias divergentes.",
  },
  {
    id: "cr-carousel-5",
    offerId: "offer-demo-001",
    format: "carousel_5_slides",
    title: "Carrossel educativo de 5 slides (Necessidade → Critérios → Escolha → Condição → CTA)",
    assetPath: "/brand/templates/carousel-5-slides.svg",
    disclosureText: "#publi visível nos slides 1 e 5, com aviso de afiliado legível.",
    mediaRightsId: "mr-svg-demo-stand",
    approvalRevision: "rev-demo-20261115-01",
    isIllustrativeDemo: true,
    notes:
      "Cada slide funciona de forma compreensível isoladamente e explicita limitações reais antes do CTA.",
  },
  {
    id: "cr-story-916",
    offerId: "offer-demo-003",
    format: "story_1080x1920",
    title: "Capa Vertical 1080×1920 (Story / Reels / Shorts) com zonas seguras",
    assetPath: "/brand/templates/story-cover-1080x1920.svg",
    disclosureText: "Publicidade / link de afiliado visível dentro da zona segura superior.",
    mediaRightsId: "mr-svg-demo-lamp",
    approvalRevision: "rev-demo-20261115-03",
    isIllustrativeDemo: true,
    notes:
      "Reserva 220 px superiores e 340 px inferiores livres de texto essencial. No YouTube Shorts, lembra que URL na descrição não é clicável.",
  },
];

export const TELEGRAM_POST_TEMPLATE = `#publi
🔎 Suporte Articulado de Alumínio (Versão 2 · Prata): opção para elevar o notebook na altura dos olhos em mesa fixa.
Base vazada favorece ventilação e apoios de silicone ajudam a estabilizar telas de até 16" (até 4 kg).
R$ 89,90 à vista no Pix (valor demonstrativo fictício), observado em 15/11/2026 às 14:15 (horário de Cuiabá). Frete depende do CEP.
⚠️ Com ~780 g, é menos prático para mochila e pede teclado e mouse externos para uso elevado.
Ver oferta na Shopee: [aguardando URL oficial aprovada — exemplo de protótipo]
Link de afiliado: podemos receber comissão se você comprar. Preço e disponibilidade podem mudar; confirme as condições na loja.`;

export const WHATSAPP_MANUAL_DIGEST_TEMPLATE = `*procurali — Resumo manual (dados fictícios de protótipo)*
#publi

1) *Suporte Articulado de Alumínio V2 (Prata)*
• Para quem faz sentido: mesa fixa com teclado/mouse externos (telas até 16").
• Observação demo: R$ 89,90 no Pix em 15/11/2026 14:15 (frete por CEP).
• Atenção: pesa ~780 g; pouco prático para levar toda hora na mochila.
• Link oficial: [aguardando aprovação Shopee e revisão do proprietário]

2) *Hub USB-C 7 em 1 PD 100W (Cinza)*
• Para quem faz sentido: conectar monitor HDMI e USB-A em notebook com USB-C de vídeo.
• Observação demo: R$ 139,90 no Pix em 15/11/2026 14:20.
• Atenção: exige porta USB-C com DisplayPort Alt Mode; HDMI limitado a 4K 30 Hz.
• Link oficial: [aguardando aprovação Shopee e revisão do proprietário]

_Link de afiliado: podemos receber comissão se você comprar. Preços e estoque mudam na loja._`;

export const SHORT_VIDEO_STORYBOARD = [
  {
    window: "0–3 s",
    visual: "Capa vertical com selo 'Publicidade / link de afiliado' + gancho de texto.",
    voiceAndSubtitles:
      "Antes de comprar luminária de monitor, confira a espessura da borda e se a sua tela é curva.",
  },
  {
    window: "3–10 s",
    visual: "Esquema ilustrativo de mesa com reflexo de abajur vs feixe inclinado.",
    voiceAndSubtitles:
      "Muita gente compra luminária comum de mesa e acaba ganhando reflexo direto no monitor à noite.",
  },
  {
    window: "10–21 s",
    visual: "Ilustração técnica apontando o clipe contrapeso e largura de 40 cm.",
    voiceAndSubtitles:
      "Dois pontos para checar no anúncio: abertura do grampo (neste exemplo, 0,8 a 3,2 cm) e alimentação USB 5V.",
  },
  {
    window: "21–29 s",
    visual: "Cartão determinístico com valor e horário da observação (ou critério atemporal).",
    voiceAndSubtitles:
      "Na demonstração, o registro com cupom da loja era de R$ 169 em 15/11, mas cupons variam por conta.",
  },
  {
    window: "29–35 s",
    visual: "Alerta ⚠️ de limitação em tela curva + CTA compatível com a plataforma.",
    voiceAndSubtitles:
      "Se seu monitor tiver traseira curva grossa, confira as medidas antes. O guia completo fica no link do perfil.",
  },
];

export const EDITORIAL_APPROVALS = [
  ...ML_EDITORIAL_APPROVALS,
  {
    revision: "rev-demo-20261115-01",
    targetType: "offer",
    targetId: "offer-demo-001",
    status: "pending_owner_review",
    contentHash: "demo-sha256-stand-v2-01",
    mediaHash: "demo-sha256-svg-stand-01",
    approvedBy: null,
    approvedAt: null,
    blockers: [
      "Item fictício de demonstração (isDemoFixture: true).",
      "Aguardando aprovação da inscrição Shopee Afiliados e URL oficial real.",
    ],
  },
  {
    revision: "rev-demo-20261115-04-stale",
    targetType: "offer",
    targetId: "offer-demo-004",
    status: "invalidated_stale",
    contentHash: "demo-sha256-dps-04",
    mediaHash: "demo-sha256-svg-clamp-04",
    approvedBy: null,
    approvedAt: null,
    blockers: [
      "Observação de preço com mais de 30 minutos (estado stale bloqueia publicação automática).",
    ],
  },
  {
    revision: "rev-demo-20261115-blocked-rights",
    targetType: "offer",
    targetId: "offer-demo-006",
    status: "rejected",
    contentHash: "demo-sha256-gan-06",
    mediaHash: "pending-unverified",
    approvedBy: null,
    approvedAt: null,
    blockers: [
      "MediaRights.rightsVerified === false (direitos de imagem não confirmados).",
    ],
  },
];

export const HOME_SHOWCASE_META = {
  featuredSlug: "suporte-articulado-notebook-aluminio-v2",
  featuredMeta: {
    topBadge: "Destaque · Menor preço 7d",
    referencePriceBrlCents: 10490,
    savingsText: "Economia de R$ 15,00 (amostra 7d)",
    shortDesc: "Alumínio articulado em 2 eixos · eleva a tela e libera espaço na mesa.",
    imagePath: "/brand/templates/featured-stand-photo.jpg",
    ctaLabel: "Ver oferta na Shopee",
  },
  dailySlugs: ML_OFFERS.map((offer) => offer.slug),
  dailyCards: ML_DAILY_SHOWCASE_CARDS,
  cardMetaBySlug: {
    "suporte-articulado-notebook-aluminio-v2": {
      topBadge: "Menor preço 7d · −14%",
      badgeTone: "emerald",
      marketplaceLabel: "Shopee",
      shortTitle: "Suporte Articulado de Alumínio para Notebook",
      shortDesc: "Ajuste duplo de altura e recorte para ventilação em mesa fixa.",
      storePill: "Shopee · Loja Oficial Exemplo",
      refPriceCents: 10490,
      mintText: "Economia de R$ 15,00 (amostra 7d)",
      photoPath: "/brand/templates/card-stand-photo.jpg",
    },
    "hub-usb-c-7-em-1-pd-100w-cinza": {
      topBadge: "Compra funcional",
      badgeTone: "neutral",
      marketplaceLabel: "Mercado Livre",
      shortTitle: "Hub USB-C 7 em 1 (HDMI 4K + PD 100W)",
      shortDesc: "HDMI 4K30, leitor SD/TF, 2× USB-A 3.0 e carga pass-through.",
      storePill: "Mercado Livre · Distribuidor Tech Exemplo",
      refPriceCents: null,
      mintText: "Corpo em alumínio · 7 portas úteis",
      photoPath: "/brand/templates/card-hub-photo.jpg",
    },
    "luminaria-barra-monitor-assimetrica-40cm": {
      topBadge: "Cupom −R$ 15",
      badgeTone: "emerald",
      marketplaceLabel: "Shopee",
      shortTitle: "Luminária Barra para Monitor 40 cm Assimétrica",
      shortDesc: "Ótica direcionada USB 5V para teclado sem reflexo na tela.",
      storePill: "Shopee · Casa & Mesa Exemplo",
      refPriceCents: 18400,
      mintText: "Economia de R$ 15,00 com cupom LUZ15",
      deadlineText: "Cupom válido até 23h59 de 15/11 · Brasília",
      photoPath: "/brand/templates/card-lamp-photo.jpg",
    },
    "filtro-de-linha-dps-6-tomadas-cabo-2m": {
      topBadge: "Preço em atualização",
      badgeTone: "warning",
      marketplaceLabel: "Mercado Livre",
      shortTitle: "Filtro de Linha DPS 6 Tomadas Cabo 2 m",
      shortDesc: "Proteção varistor e disjuntor; exemplo de registro >30 min.",
      storePill: "Mercado Livre · Elétrica Segura Exemplo",
      refPriceCents: null,
      mintText: null,
      deadlineText: "Última checagem há >2h · preço em atualização",
      photoPath: "/brand/templates/card-dps-photo.jpg",
    },
    "organizador-cabos-bandeja-sob-mesa-aco": {
      topBadge: "Prazo encerrado",
      badgeTone: "neutral",
      marketplaceLabel: "Shopee",
      shortTitle: "Organizador de Cabos Bandeja Sob a Mesa em Aço",
      shortDesc: "Exemplo demonstrativo de anúncio expirado mantido fora de destaque.",
      storePill: "Shopee · Loja Arquitetura Mesa",
      refPriceCents: null,
      mintText: null,
      photoPath: "/brand/templates/card-dps-photo.jpg",
    },
    "carregador-gan-65w-3-portas-branco": {
      topBadge: "Mídia pendente",
      badgeTone: "danger",
      marketplaceLabel: "Shopee",
      shortTitle: "Carregador GaN 65W 3 Portas",
      shortDesc: "Plugue BR 2× USB-C + USB-A; bloqueio por licença pendente.",
      storePill: "Bloqueado Demo",
      refPriceCents: null,
      mintText: "Direitos pendentes",
      photoPath: "/brand/templates/card-hub-photo.jpg",
    },
  },
};

export const GUIDE_VISUALS = {
  "como-escolher-suporte-para-notebook-e-monitor": {
    thumbPath: "/brand/templates/guide-photo-ergonomia.jpg",
    thumbAlt: "Braço articulado e suporte ergonômico instalado em monitor de mesa",
    badgeLabel: "Guia de compra",
    homeTitle: "Como escolher suporte articulado para notebook e monitor sem errar",
    readingMinutes: 8,
    updatedLabel: "Atualizado em 15/11",
  },
  "guia-hub-usb-c-e-carregadores-gan-sem-erro": {
    thumbPath: "/brand/templates/card-hub-photo.jpg",
    thumbAlt: "Hub USB-C 7 em 1 em alumínio ao lado de cabos de carregamento e vídeo",
    badgeLabel: "Comparativo técnico",
    homeTitle: "Hub USB-C ou carregador GaN 65W: o que conferir antes de comprar",
    readingMinutes: 7,
    updatedLabel: "Atualizado em 15/11",
  },
  "organizacao-de-cabos-e-energia-na-bancada": {
    thumbPath: "/brand/templates/card-dps-photo.jpg",
    thumbAlt: "Filtro de linha DPS de 6 tomadas com cabo aterrado para organização da mesa",
    badgeLabel: "Guia de segurança",
    homeTitle: "Organização de cabos e proteção elétrica DPS na bancada sem exageros",
    readingMinutes: 6,
    updatedLabel: "Atualizado em 15/11",
  },
};
