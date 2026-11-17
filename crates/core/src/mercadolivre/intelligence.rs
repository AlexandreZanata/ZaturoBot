use super::*;
use chrono::{DateTime, FixedOffset, Utc};

impl ValidatedMercadoLivreProduct {
    pub fn observed_at_display_for_telegram(&self) -> String {
        format_cuiaba_display(self.observed_at_ms)
    }

    pub fn to_site_catalog_offer(
        &self,
        evidence_id: &str,
        media_rights_id: &str,
    ) -> SiteCatalogOffer {
        let lead = &self.images[0];
        let is_in_stock = self.availability == "InStock";
        let return_note = self
            .return_days
            .map(|d| format!("Devolução grátis {d}d"))
            .unwrap_or_else(|| "Devolução conforme anúncio".into());
        let short_use_line = if self.short_description.chars().count() > 140 {
            let truncated: String = self.short_description.chars().take(137).collect();
            format!("{truncated}...")
        } else {
            self.short_description.clone()
        };
        let mut offer = SiteCatalogOffer {
            id: format!("offer-ml-{}", self.product_id),
            slug: self.slug.clone(),
            is_demo_fixture: false,
            product_id: self.product_id.clone(),
            brand: self.brand.clone(),
            model: self.model.clone(),
            title: self.title.clone(),
            exact_variant: self.exact_variant.clone(),
            category_slug: self.category_slug.clone(),
            subcategory_slug: String::new(),
            subcategory_name: String::new(),
            merchant: format!("Mercado Livre · {}", self.seller_name),
            marketplace_label: "Mercado Livre".into(),
            seller_name: self.seller_name.clone(),
            store_short: self.seller_name.clone(),
            program: "Mercado Livre Afiliados (ingestão validada via Hub)".into(),
            official_affiliate_url: self.share_link.clone(),
            canonical_url: self.canonical_url.clone(),
            share_id: self.share_id.clone(),
            share_text: self.share_text.clone(),
            sponsored: self.sponsored,
            commission_rate: self.commission_label.clone(),
            marketplace_rating: self.marketplace_rating.clone(),
            rating_milli: None,
            marketplace_sales_label: self.marketplace_sales_label.clone(),
            sales_volume_estimate: 0,
            card_badge: self.card_badge.clone(),
            discount_percent_label: self.discount_label.clone(),
            discount_pct: 0,
            trust_badges: Vec::new(),
            installment_summary: self.installment_summary.clone(),
            return_days: self.return_days,
            availability: self.availability.clone(),
            price_valid_until: self.price_valid_until.clone(),
            image_path: lead.public_url.clone(),
            image_alt: lead.alt_text.clone(),
            gallery_paths: self
                .images
                .iter()
                .map(|img| SiteCatalogGalleryItem {
                    path: img.public_url.clone(),
                    alt: img.alt_text.clone(),
                    bucket: Some(img.bucket.clone()),
                    object_key: Some(img.object_key.clone()),
                    sha256: Some(img.sha256_hex.clone()),
                })
                .collect(),
            price_brl_cents: self.price_brl_cents,
            pix_price_cents: self.pix_price_cents,
            reference_price_cents: self.reference_price_cents,
            currency: self.currency.clone(),
            payment_conditions: self.payment_conditions.clone(),
            short_condition: self.short_condition.clone(),
            short_use_line,
            short_checked_at: format_cuiaba_short(self.observed_at_ms),
            cta_dest_label: "Ver oferta no Mercado Livre".into(),
            coupon_condition: None,
            freight_status: if is_in_stock {
                self.freight_status.clone()
            } else {
                format!("{} · {return_note}", self.freight_status)
            },
            observed_at: format_utc_iso_ms(self.observed_at_ms),
            observed_at_display: format_cuiaba_display(self.observed_at_ms),
            valid_until: self
                .price_valid_until
                .as_ref()
                .map(|d| format!("{d}T23:59:59-03:00")),
            valid_until_display: self
                .price_valid_until
                .as_ref()
                .map(|d| format!("Validade registrada até {d}")),
            freshness_ttl_minutes: self.freshness_ttl_minutes,
            evidence_refs: vec![evidence_id.to_owned()],
            media_rights_ref: media_rights_id.to_owned(),
            state: self.state.as_str().to_owned(),
            approval_revision: format!(
                "ml-ingested-{}",
                self.evidence_sha256.chars().take(12).collect::<String>()
            ),
            one_line_reason: self.one_line_reason.clone(),
            for_whom_it_makes_sense: self.for_whom.clone(),
            supported_benefits: self.supported_benefits.clone(),
            limitations: self.limitations.clone(),
            unknowns: self.unknowns.clone(),
            tags: self.tags.clone(),
            has_verified_seven_day_sample: false,
            seven_day_coverage_note:
                "Ingestão validada via Hub Mercado Livre; série contínua de 7 dias acumulada progressivamente."
                    .into(),
            price_observations: vec![SiteCatalogPriceObservation {
                id: format!("obs-ml-{}-{}", self.product_id, self.observed_at_ms),
                offer_id: format!("offer-ml-{}", self.product_id),
                variant_id: self
                    .model
                    .clone()
                    .unwrap_or_else(|| self.product_id.clone()),
                seller_name: self.seller_name.clone(),
                payment_basis: self.payment_conditions.clone(),
                freight_note: self.short_condition.clone(),
                price_brl_cents: self.price_brl_cents,
                observed_at: format_utc_iso_ms(self.observed_at_ms),
                evidence_id: evidence_id.to_owned(),
            }],
            specs: self.specs.clone(),
            variants: self.variants.clone(),
            long_description: self.long_description.clone(),
            price_intelligence: None,
            dynamic_faq: Vec::new(),
            similar_offer_ids: Vec::new(),
        };
        enrich_site_catalog_offer(&mut offer, self.observed_at_ms);
        offer
    }
}

/// Classifies an offer into a two-level category/subcategory taxonomy inspired by cloo.be.
pub fn classify_product_subcategory(
    title: &str,
    category_slug: &str,
    specs: &[MercadoLivreSpec],
) -> (String, String) {
    let mut hay = title.to_ascii_lowercase();
    for spec in specs.iter().take(12) {
        hay.push(' ');
        hay.push_str(&spec.name.to_ascii_lowercase());
        hay.push(' ');
        hay.push_str(&spec.value.to_ascii_lowercase());
    }
    let rules: &[(&[&str], &str, &str)] = &[
        (
            &[
                "monitor",
                "ultragear",
                "aoc agon",
                "144hz",
                "200hz",
                "displayport",
            ],
            "monitores",
            "Monitores",
        ),
        (
            &[
                "ssd",
                "nvme",
                "m.2",
                "pcie",
                "disco sólido",
                "armazenamento",
                "hd externo",
            ],
            "ssd-e-armazenamento",
            "SSDs e Armazenamento",
        ),
        (
            &[
                "teclado",
                "mouse",
                "kumara",
                "switch",
                "mecanico",
                "mecânico",
            ],
            "teclados-e-mouses",
            "Teclados e Mouses",
        ),
        (
            &[
                "headset",
                "fone",
                "audio",
                "áudio",
                "microfone",
                "drivers 53mm",
            ],
            "headsets-e-audio",
            "Headsets e Áudio",
        ),
        (
            &[
                "carregador",
                "gan",
                "cabo",
                "filtro de linha",
                "dps",
                "tomada",
                "hub usb",
            ],
            "cabos-e-energia",
            "Cabos, Hubs e Energia",
        ),
        (
            &["suporte", "ergonomia", "articulado", "braço", "stand"],
            "suportes-e-ergonomia",
            "Suportes e Ergonomia",
        ),
        (
            &["luminaria", "luminária", "lightbar", "led"],
            "iluminacao",
            "Iluminação de Bancada",
        ),
    ];
    for (keywords, slug, label) in rules {
        if keywords.iter().any(|kw| hay.contains(kw)) {
            return ((*slug).to_owned(), (*label).to_owned());
        }
    }
    match category_slug {
        "ergonomia-e-suportes" => ("suportes-e-ergonomia".into(), "Suportes e Ergonomia".into()),
        "energia-e-cabos" => ("cabos-e-energia".into(), "Cabos, Hubs e Energia".into()),
        "iluminacao-de-bancada" => ("iluminacao".into(), "Iluminação de Bancada".into()),
        _ => (
            "acessorios-gerais".into(),
            "Acessórios e Periféricos".into(),
        ),
    }
}

/// Parses marketplace rating string such as `"4,9 / 5"` into milli-stars (`4900`).
pub fn parse_rating_milli(rating: Option<&str>) -> Option<u16> {
    let raw = rating?.trim();
    let score_part = raw.split('/').next()?.trim().replace(',', ".");
    let parsed: f64 = score_part.parse().ok()?;
    if !(0.0..=5.0).contains(&parsed) {
        return None;
    }
    Some((parsed * 1000.0).round() as u16)
}

/// Parses marketplace sales labels such as `"+10 mil vendidos no ML"` into integer volume.
pub fn parse_sales_volume_estimate(sales: Option<&str>) -> u32 {
    let Some(raw) = sales else {
        return 0;
    };
    let lower = raw.to_ascii_lowercase();
    let digits: String = lower
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .filter(|c| c.is_ascii_digit())
        .collect();
    let Ok(base) = digits.parse::<u32>() else {
        return 0;
    };
    if lower.contains("mil") && base < 1000 {
        base.saturating_mul(1000)
    } else {
        base
    }
}

/// Resolves integer discount percentage (`0..=99`) from prices or explicit badge text.
pub fn compute_discount_pct(
    price_cents: i64,
    reference_cents: Option<i64>,
    discount_label: Option<&str>,
) -> u8 {
    if let Some(ref_c) = reference_cents
        && ref_c > price_cents
        && price_cents > 0
    {
        let pct = ((ref_c - price_cents) as f64 / ref_c as f64 * 100.0).round() as u32;
        return u8::try_from(pct.clamp(1, 99)).unwrap_or(99);
    }
    if let Some(lbl) = discount_label {
        let digits: String = lbl
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(pct) = digits.parse::<u8>() {
            return pct.min(99);
        }
    }
    0
}

pub fn format_verified_at_cuiaba(ms: i64) -> String {
    let offset = FixedOffset::west_opt(4 * 3600).expect("valid UTC-4 offset");
    let dt = DateTime::<Utc>::from_timestamp_millis(ms)
        .unwrap_or_default()
        .with_timezone(&offset);
    format!("Preços verificados em {}", dt.format("%d/%m/%Y, %H:%M"))
}

/// Renders a zero-dependency server-side `<svg viewBox="0 0 300 80">` sparkline chart.
pub fn build_price_sparkline_svg(
    chronological_prices: &[i64],
    lowest_cents: i64,
    current_cents: i64,
) -> String {
    let samples: Vec<i64> = if chronological_prices.len() >= 2 {
        chronological_prices.to_vec()
    } else if chronological_prices.len() == 1 && lowest_cents != current_cents {
        vec![lowest_cents.max(current_cents), current_cents]
    } else {
        vec![current_cents, current_cents]
    };

    let min_v = samples.iter().copied().min().unwrap_or(current_cents);
    let max_v = samples.iter().copied().max().unwrap_or(current_cents);
    let span = (max_v - min_v).max(1) as f64;
    let width = 280.0f64;
    let pad_x = 10.0f64;
    let height = 56.0f64;
    let pad_top = 12.0f64;
    let count = samples.len().max(2);

    let mut coords = Vec::with_capacity(samples.len());
    let mut area_points = Vec::with_capacity(samples.len() + 2);
    let mut min_idx = 0usize;
    for (idx, price) in samples.iter().enumerate() {
        if *price == min_v {
            min_idx = idx;
        }
        let x = pad_x + (idx as f64 / (count - 1) as f64) * width;
        let norm_y = if max_v == min_v {
            0.5
        } else {
            (*price - min_v) as f64 / span
        };
        let y = pad_top + (1.0 - norm_y) * height;
        coords.push((x, y));
        area_points.push(format!("{x:.1},{y:.1}"));
    }

    let points_attr = area_points.join(" ");
    let last_x = coords.last().map(|p| p.0).unwrap_or(290.0);
    let first_x = coords.first().map(|p| p.0).unwrap_or(10.0);
    let base_y = pad_top + height + 6.0;
    let fill_attr = format!("{first_x:.1},{base_y:.1} {points_attr} {last_x:.1},{base_y:.1}");
    let (min_x, min_y) = coords[min_idx];
    let (cur_x, cur_y) = coords[coords.len() - 1];

    format!(
        "<svg viewBox=\"0 0 300 80\" width=\"300\" height=\"80\" role=\"img\" aria-label=\"Histórico de preço observado nos últimos 30 dias\" xmlns=\"http://www.w3.org/2000/svg\"><polygon points=\"{fill_attr}\" fill=\"rgba(22, 101, 52, 0.10)\"/><polyline fill=\"none\" stroke=\"#166534\" stroke-width=\"2.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\" points=\"{points_attr}\"/><circle cx=\"{min_x:.1}\" cy=\"{min_y:.1}\" r=\"3.5\" fill=\"#65a30d\"/><circle cx=\"{cur_x:.1}\" cy=\"{cur_y:.1}\" r=\"4\" fill=\"#c2410c\"/></svg>"
    )
}

/// Computes 30-day window pricing intelligence, classification, savings, and SVG sparkline.
pub fn compute_price_intelligence(
    current_price_cents: i64,
    reference_price_cents: Option<i64>,
    discount_label: Option<&str>,
    observed_at_ms: i64,
    observations: &[SiteCatalogPriceObservation],
) -> PriceIntelligence {
    let mut chronological: Vec<i64> = observations
        .iter()
        .rev()
        .map(|obs| obs.price_brl_cents)
        .filter(|c| *c > 0)
        .collect();
    if chronological.is_empty() && current_price_cents > 0 {
        chronological.push(current_price_cents);
    }

    let obs_min = chronological
        .iter()
        .copied()
        .min()
        .unwrap_or(current_price_cents);
    let obs_max = chronological
        .iter()
        .copied()
        .max()
        .unwrap_or(current_price_cents);
    let obs_sum: i128 = chronological.iter().map(|c| i128::from(*c)).sum();
    let avg_cents = if chronological.is_empty() {
        current_price_cents
    } else {
        i64::try_from(obs_sum / i128::try_from(chronological.len()).unwrap_or(1))
            .unwrap_or(current_price_cents)
    };

    let lowest_price_30d_cents = obs_min.min(current_price_cents);
    let highest_price_30d_cents = reference_price_cents
        .filter(|r| *r > 0)
        .map_or(obs_max.max(current_price_cents), |r| {
            obs_max.max(current_price_cents).max(r)
        });
    let discount_pct =
        compute_discount_pct(current_price_cents, reference_price_cents, discount_label);
    let savings_vs_reference_cents = reference_price_cents
        .filter(|r| *r > current_price_cents)
        .map_or(0, |r| r - current_price_cents);
    let savings_vs_average_cents = (avg_cents - current_price_cents).max(0);

    // Price drop occurs if the prior observed snapshot was higher or discount is meaningful.
    let previous_price_cents = observations
        .get(1)
        .map(|prev| prev.price_brl_cents)
        .filter(|c| *c > 0);
    let (price_trend, price_delta_cents, price_delta_pct) =
        PriceTrendKind::from_prices(current_price_cents, previous_price_cents);
    let is_price_drop = price_trend == PriceTrendKind::Down || discount_pct >= 20;
    let first_observed_at_display = observations
        .last()
        .map(|obs| obs.observed_at.clone())
        .unwrap_or_else(|| format_utc_iso_ms(observed_at_ms));
    let observation_count = chronological.len().max(1);

    let price_level = if chronological.len() >= 2 {
        if current_price_cents <= lowest_price_30d_cents && current_price_cents < avg_cents {
            PriceLevelKind::LowestPrice
        } else if current_price_cents.saturating_mul(100) <= avg_cents.saturating_mul(92)
            || discount_pct >= 20
        {
            PriceLevelKind::GoodPrice
        } else {
            PriceLevelKind::TypicalPrice
        }
    } else if discount_pct >= 25 {
        PriceLevelKind::LowestPrice
    } else if discount_pct >= 10 {
        PriceLevelKind::GoodPrice
    } else {
        PriceLevelKind::TypicalPrice
    };

    let sparkline_points = if chronological.len() >= 2 {
        chronological
    } else if let Some(ref_c) = reference_price_cents.filter(|r| *r > current_price_cents) {
        vec![ref_c, current_price_cents]
    } else {
        vec![current_price_cents]
    };
    let svg_sparkline = build_price_sparkline_svg(
        &sparkline_points,
        highest_price_30d_cents,
        current_price_cents,
    );

    PriceIntelligence {
        window_days: 30,
        current_price_cents,
        lowest_price_30d_cents,
        average_price_30d_cents: avg_cents,
        highest_price_30d_cents,
        reference_price_cents,
        previous_price_cents,
        price_delta_cents,
        price_delta_pct,
        observation_count,
        price_trend,
        first_observed_at_display,
        discount_pct,
        savings_vs_reference_cents,
        savings_vs_average_cents,
        price_level,
        price_level_label: price_level.label_pt_br().to_owned(),
        is_price_drop,
        verified_at_display: format_verified_at_cuiaba(observed_at_ms),
        svg_sparkline,
    }
}

pub fn build_trust_badges(offer: &SiteCatalogOffer, intel: &PriceIntelligence) -> Vec<String> {
    let mut badges = Vec::new();
    if intel.discount_pct >= 5 {
        badges.push(format!("-{}%", intel.discount_pct));
    }
    let seller_lower = offer.seller_name.to_ascii_lowercase();
    if seller_lower.contains("oficial") {
        badges.push("OFICIAL".to_owned());
    }
    if offer.card_badge.is_some() {
        badges.push("MAIS VENDIDO".to_owned());
    }
    if intel.price_level == PriceLevelKind::LowestPrice {
        badges.push("MENOR PREÇO".to_owned());
    } else if intel.is_price_drop {
        badges.push("BAIXOU DE PREÇO".to_owned());
    }
    if offer.return_days.is_some_and(|d| d >= 7) {
        badges.push("DEVOLUÇÃO GRÁTIS".to_owned());
    }
    badges
}

pub fn build_dynamic_product_faq(
    offer: &SiteCatalogOffer,
    intel: &PriceIntelligence,
) -> Vec<ProductFaqItem> {
    let cur = format_brl_cents(offer.price_brl_cents);
    let low = format_brl_cents(intel.lowest_price_30d_cents);
    let avg = format_brl_cents(intel.average_price_30d_cents);
    let mut faq = vec![ProductFaqItem {
        question: format!("O preço atual de {} está bom hoje?", offer.brand),
        answer: format!(
            "No monitoramento dos últimos {} dias, a oferta atual ({cur}) recebeu classificação \"{}\". O menor preço registrado no período foi {low} e a média observada é {avg}. {}",
            intel.window_days, intel.price_level_label, intel.verified_at_display
        ),
    }];
    faq.push(ProductFaqItem {
        question: "O vendedor e o link de compra são oficiais?".to_owned(),
        answer: format!(
            "Sim. O produto {} ({}) direciona exclusivamente para o anúncio oficial {} em {} sem encurtadores paralelos.",
            offer.product_id, offer.seller_name, offer.official_affiliate_url, offer.marketplace_label
        ),
    });
    faq.push(ProductFaqItem {
        question: "Qual é a política de pagamento, frete e devolução?".to_owned(),
        answer: format!("{} · {}.", offer.payment_conditions, offer.freight_status),
    });
    if let Some(lim) = offer.limitations.first() {
        faq.push(ProductFaqItem {
            question: "O que devo conferir antes de comprar?".to_owned(),
            answer: lim.clone(),
        });
    }
    faq
}

/// Populates taxonomy, ratings, sales volume, trust badges, price intelligence, and FAQ cleanly.
pub fn enrich_site_catalog_offer(offer: &mut SiteCatalogOffer, observed_at_ms: i64) {
    let (sub_slug, sub_name) =
        classify_product_subcategory(&offer.title, &offer.category_slug, &offer.specs);
    offer.subcategory_slug = sub_slug;
    offer.subcategory_name = sub_name;
    offer.rating_milli = parse_rating_milli(offer.marketplace_rating.as_deref());
    offer.sales_volume_estimate =
        parse_sales_volume_estimate(offer.marketplace_sales_label.as_deref());
    let intel = compute_price_intelligence(
        offer.pix_price_cents.min(offer.price_brl_cents),
        offer.reference_price_cents,
        offer.discount_percent_label.as_deref(),
        observed_at_ms,
        &offer.price_observations,
    );
    offer.discount_pct = intel.discount_pct;
    offer.trust_badges = build_trust_badges(offer, &intel);
    offer.dynamic_faq = build_dynamic_product_faq(offer, &intel);
    offer.price_intelligence = Some(intel);
}

/// Renders zero-hydration Cloobe-style `<article class="offer-card">` HTML fragments for incremental pagination.
pub fn render_offer_cards_html_fragment(offers: &[SiteCatalogOffer], start_pos: usize) -> String {
    let mut html = String::with_capacity(offers.len().saturating_mul(640));
    for (idx, offer) in offers.iter().enumerate() {
        let pos = start_pos.saturating_add(idx).saturating_add(1);
        let badges_html = offer
            .trust_badges
            .iter()
            .take(3)
            .map(|b| format!("<span class=\"badge\">{}</span>", escape_telegram_html(b)))
            .collect::<Vec<_>>()
            .join("");
        let ref_html = offer
            .reference_price_cents
            .filter(|r| *r > offer.price_brl_cents)
            .map(|r| format!("<del>{}</del>", escape_telegram_html(&format_brl_cents(r))))
            .unwrap_or_default();
        let intel_badge = offer
            .price_intelligence
            .as_ref()
            .map(|i| escape_telegram_html(&i.price_level_label))
            .unwrap_or_default();
        html.push_str(&format!(
            "<article class=\"offer-card\" data-store=\"mercadolivre\" data-id=\"{}\" data-pos=\"{pos}\" data-subcat=\"{}\"><a class=\"offer-card__link\" href=\"/ofertas/{}/index.html\"><img src=\"{}\" alt=\"{}\" loading=\"lazy\" decoding=\"async\"><div class=\"offer-card__badges\">{badges_html}</div><h3>{}</h3><p class=\"offer-card__price\">{ref_html}<strong>{}</strong></p><p class=\"offer-card__meta\">{} · {}</p></a></article>",
            escape_telegram_html(&offer.product_id),
            escape_telegram_html(&offer.subcategory_slug),
            escape_telegram_html(&offer.slug),
            escape_telegram_html(&offer.image_path),
            escape_telegram_html(&offer.image_alt),
            escape_telegram_html(&offer.title),
            escape_telegram_html(&format_brl_cents(offer.price_brl_cents)),
            intel_badge,
            escape_telegram_html(&offer.short_condition),
        ));
    }
    html
}

/// Formats a concise Portuguese summary comparing an offer's current spot price against
/// historical observations in the 30-day catalog window.
pub fn format_price_comparison_summary(
    current_price_cents: i64,
    previous_price_cents: Option<i64>,
    lowest_30d_cents: i64,
    average_30d_cents: i64,
    observation_count: usize,
    trend: PriceTrendKind,
    delta_pct: i16,
) -> String {
    let cur = format_brl_cents(current_price_cents);
    let low = format_brl_cents(lowest_30d_cents);
    let avg = format_brl_cents(average_30d_cents);
    match (trend, previous_price_cents) {
        (PriceTrendKind::Down, Some(prev)) => format!(
            "Queda para {cur} ({delta_pct}% vs anterior {}; mín 30d {low}, méd {avg}, {observation_count} obs)",
            format_brl_cents(prev)
        ),
        (PriceTrendKind::Up, Some(prev)) => format!(
            "Alta para {cur} (+{delta_pct}% vs anterior {}; mín 30d {low}, méd {avg}, {observation_count} obs)",
            format_brl_cents(prev)
        ),
        (PriceTrendKind::Stable, _) => {
            format!("Estável em {cur} (mín 30d {low}, méd {avg}, {observation_count} obs)")
        }
        _ => format!("Primeira captura em {cur} (mín 30d {low}, méd {avg})"),
    }
}

/// Projects a compact `CommerceReferenceItem` from an enriched `SiteCatalogOffer` and
/// its aggregate Telegram delivery telemetry.
pub fn build_commerce_reference_item(
    offer: &SiteCatalogOffer,
    telegram_published_count: u32,
    telegram_last_status: Option<String>,
) -> CommerceReferenceItem {
    let current_eff = offer.pix_price_cents.min(offer.price_brl_cents);
    let intel = offer.price_intelligence.clone().unwrap_or_else(|| {
        compute_price_intelligence(
            current_eff,
            offer.reference_price_cents,
            offer.discount_percent_label.as_deref(),
            parse_iso_to_ms(&offer.observed_at).unwrap_or(1),
            &offer.price_observations,
        )
    });
    let short_summary = format_price_comparison_summary(
        current_eff,
        intel.previous_price_cents,
        intel.lowest_price_30d_cents,
        intel.average_price_30d_cents,
        intel.observation_count,
        intel.price_trend,
        intel.price_delta_pct,
    );
    CommerceReferenceItem {
        product_id: offer.product_id.clone(),
        slug: offer.slug.clone(),
        title: offer.title.clone(),
        brand: offer.brand.clone(),
        model: offer.model.clone(),
        exact_variant: offer.exact_variant.clone(),
        category_slug: offer.category_slug.clone(),
        subcategory_slug: offer.subcategory_slug.clone(),
        seller_name: offer.seller_name.clone(),
        state: offer.state.clone(),
        current_price_cents: offer.price_brl_cents,
        pix_price_cents: offer.pix_price_cents,
        reference_price_cents: offer.reference_price_cents,
        discount_pct: intel.discount_pct,
        lowest_price_30d_cents: intel.lowest_price_30d_cents,
        average_price_30d_cents: intel.average_price_30d_cents,
        highest_price_30d_cents: intel.highest_price_30d_cents,
        previous_price_cents: intel.previous_price_cents,
        price_delta_cents: intel.price_delta_cents,
        price_delta_pct: intel.price_delta_pct,
        observation_count: intel.observation_count,
        price_trend: intel.price_trend,
        price_level: intel.price_level,
        price_level_label: intel.price_level_label,
        observed_at: offer.observed_at.clone(),
        observed_at_ms: parse_iso_to_ms(&offer.observed_at).unwrap_or(0),
        official_affiliate_url: offer.official_affiliate_url.clone(),
        short_summary,
        telegram_published_count,
        telegram_last_status,
    }
}
