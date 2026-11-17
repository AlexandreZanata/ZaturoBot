use super::*;
use serde::{Deserialize, Serialize};

pub const TELEGRAM_CAPTION_MAX_CHARS: usize = 1024;
pub const TELEGRAM_MESSAGE_MAX_CHARS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramInlineButton {
    pub text: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramInlineKeyboardMarkup {
    pub inline_keyboard: Vec<Vec<TelegramInlineButton>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TelegramBotCommand {
    Start,
    Help,
    Offers,
    Hardware,
    Search(String),
    WhatsappDigest,
}

impl TelegramBotCommand {
    pub fn parse(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        if !trimmed.starts_with('/') {
            return None;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let cmd_raw = parts.next()?.trim();
        let arg = parts.next().map(str::trim).unwrap_or("");
        let cmd = cmd_raw.split('@').next().unwrap_or("").to_ascii_lowercase();
        match cmd.as_str() {
            "/start" => Some(Self::Start),
            "/ajuda" | "/help" => Some(Self::Help),
            "/ofertas" | "/achados" => Some(Self::Offers),
            "/hardware" => Some(Self::Hardware),
            "/buscar" | "/procurar" => {
                if arg.is_empty() {
                    None
                } else {
                    Some(Self::Search(truncate_chars(arg, 80)))
                }
            }
            "/whatsapp" => Some(Self::WhatsappDigest),
            _ => None,
        }
    }
}

pub fn escape_telegram_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

pub(super) fn truncate_chars(input: &str, max_chars: usize) -> String {
    let trimmed = input.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_owned();
    }
    let keep = max_chars.saturating_sub(3);
    let mut out: String = trimmed.chars().take(keep).collect();
    out.push_str("...");
    out
}

pub(super) fn rendered_html_char_count(html: &str) -> usize {
    let mut count = 0usize;
    let mut chars = html.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            for inner in chars.by_ref() {
                if inner == '>' {
                    break;
                }
            }
        } else if ch == '&' {
            for inner in chars.by_ref() {
                if inner == ';' {
                    break;
                }
            }
            count += 1;
        } else {
            count += 1;
        }
    }
    count
}

pub fn build_site_offer_url(
    site_base_url: Option<&str>,
    title: &str,
    product_id: &str,
) -> Option<String> {
    let base = site_base_url?.trim().trim_end_matches('/');
    if !(base.starts_with("https://") || base.starts_with("http://")) {
        return None;
    }
    let slug = slugify_pt_br(title, product_id);
    Some(format!("{base}/ofertas/{slug}/index.html"))
}

pub fn build_telegram_offer_keyboard(
    item: &TelegramOfferQueueItem,
    site_base_url: Option<&str>,
) -> TelegramInlineKeyboardMarkup {
    let mut rows = vec![vec![TelegramInlineButton {
        text: "🔥 PEGAR OFERTA NO MERCADO LIVRE 🛒".to_owned(),
        url: item.share_link.clone(),
    }]];
    if let Some(offer_url) = build_site_offer_url(site_base_url, &item.title, &item.product_id) {
        rows.push(vec![TelegramInlineButton {
            text: "📋 Ver Ficha Completa no Procurali".to_owned(),
            url: offer_url,
        }]);
    }
    TelegramInlineKeyboardMarkup {
        inline_keyboard: rows,
    }
}

pub fn format_telegram_html_caption_pt_br(
    item: &TelegramOfferQueueItem,
    site_base_url: Option<&str>,
) -> String {
    let render = |title_limit: usize, include_extra: bool| {
        let mut lines = Vec::new();
        lines.push("🚨 <b>OFERTA RELÂMPAGO · ACHADO NO MERCADO LIVRE!</b> 🔥".to_owned());
        lines.push(String::new());
        lines.push(format!(
            "📦 <b>{}</b>",
            escape_telegram_html(&truncate_chars(&item.title, title_limit))
        ));

        let mut id_parts = vec![format!(
            "🏷️ <b>Marca:</b> {}",
            escape_telegram_html(&item.brand)
        )];
        if let Some(model) = &item.model {
            id_parts.push(format!("<b>Modelo:</b> {}", escape_telegram_html(model)));
        }
        id_parts.push(format!(
            "<b>Cód. ML:</b> <code>{}</code>",
            escape_telegram_html(&item.product_id)
        ));
        lines.push(id_parts.join(" · "));
        lines.push(String::new());

        let price_str = format_brl_cents(item.price_brl_cents);
        match item.reference_price_cents {
            Some(ref_cents) if ref_cents > item.price_brl_cents => {
                let savings_cents = ref_cents - item.price_brl_cents;
                let savings_str = format_brl_cents(savings_cents);
                let computed_pct =
                    ((savings_cents as f64 / ref_cents as f64) * 100.0).round() as u32;
                let badge = item
                    .discount_label
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| format!("{computed_pct}% OFF"));
                lines.push(format!(
                    "❌ De: <s>{}</s>",
                    escape_telegram_html(&format_brl_cents(ref_cents))
                ));
                lines.push(format!(
                    "✅ <b>POR APENAS: {}</b> 🔥 <b>({} · Economia de {}!)</b>",
                    escape_telegram_html(&price_str),
                    escape_telegram_html(&badge),
                    escape_telegram_html(&savings_str),
                ));
            }
            _ => {
                if let Some(discount) = item
                    .discount_label
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    lines.push(format!(
                        "✅ <b>PREÇO EM OFERTA: {}</b> 🔥 <b>({})</b>",
                        escape_telegram_html(&price_str),
                        escape_telegram_html(discount),
                    ));
                } else {
                    lines.push(format!(
                        "✅ <b>PREÇO EM OFERTA: {}</b> 🔥",
                        escape_telegram_html(&price_str),
                    ));
                }
            }
        }

        if include_extra {
            for extra in item.message_text.lines().filter(|line| {
                let t = line.trim();
                t.starts_with("📉")
                    || t.starts_with("💳")
                    || t.starts_with("🏪")
                    || t.starts_with("⚠️")
            }) {
                lines.push(escape_telegram_html(&truncate_chars(extra.trim(), 140)));
            }
        }

        lines.push(format!(
            "🕐 Atualizado em {}",
            escape_telegram_html(&format_cuiaba_display(item.updated_at_ms))
        ));
        lines.push(String::new());
        lines.push(
            "⏳ <b>CORRA! Preço e estoque promocional sujeitos a alteração a qualquer momento!</b>"
                .to_owned(),
        );
        lines.push(format!(
            "👉 <b>GARANTA O SEU AQUI:</b> <a href=\"{}\">Clique para abrir a oferta oficial no Mercado Livre</a>",
            escape_telegram_html(&item.share_link)
        ));
        lines.push(format!(
            "🔍 <b>Código rápido no app ML:</b> <code>{}</code>",
            escape_telegram_html(&item.share_id)
        ));
        if let Some(offer_url) = build_site_offer_url(site_base_url, &item.title, &item.product_id)
        {
            lines.push(format!(
                "📋 <a href=\"{}\">Ver ficha técnica e histórico no Procurali</a>",
                escape_telegram_html(&offer_url)
            ));
        }
        lines.push(format!(
            "<i>{}</i>",
            escape_telegram_html(AFFILIATE_DISCLOSURE_PT_BR)
        ));
        lines.join("\n")
    };

    for (title_limit, include_extra) in [
        (160usize, true),
        (110usize, true),
        (110usize, false),
        (75usize, false),
    ] {
        let candidate = render(title_limit, include_extra);
        if rendered_html_char_count(&candidate) <= TELEGRAM_CAPTION_MAX_CHARS {
            return candidate;
        }
    }
    render(55, false)
}

pub fn format_telegram_bot_command_reply_pt_br(
    command: &TelegramBotCommand,
    items: &[TelegramOfferQueueItem],
    site_base_url: Option<&str>,
) -> (String, Option<TelegramInlineKeyboardMarkup>) {
    // Public bot replies only repeat offers that were already published; queued, unapproved,
    // claimed, or failed deliveries are never disclosed to arbitrary Telegram users.
    let active_items: Vec<&TelegramOfferQueueItem> = items
        .iter()
        .filter(|i| i.status == TelegramDeliveryStatus::Published)
        .collect();

    let catalog_button_row = || -> Option<Vec<TelegramInlineButton>> {
        let base = site_base_url?.trim().trim_end_matches('/');
        if base.starts_with("https://") || base.starts_with("http://") {
            Some(vec![TelegramInlineButton {
                text: "🌐 Abrir catálogo completo no Procurali".to_owned(),
                url: format!("{base}/ofertas/index.html"),
            }])
        } else {
            None
        }
    };

    match command {
        TelegramBotCommand::Start => {
            let mut lines = vec![
                "👋 <b>Bem-vindo ao bot oficial do Procurali!</b>".to_owned(),
                format!("<i>{}</i>", escape_telegram_html(AFFILIATE_DISCLOSURE_PT_BR)),
                String::new(),
                "Comandos disponíveis:".to_owned(),
                "• <code>/ofertas</code> — últimas ofertas verificadas".to_owned(),
                "• <code>/hardware</code> — achados de hardware e periféricos".to_owned(),
                "• <code>/buscar &lt;termo&gt;</code> — procurar produto (ex.: <code>/buscar ssd</code>)".to_owned(),
                "• <code>/whatsapp</code> — gerar rascunho pronto para colar manualmente no WhatsApp Web".to_owned(),
                "• <code>/ajuda</code> — regras de transparência e links oficiais".to_owned(),
            ];
            let mut rows = Vec::new();
            if !active_items.is_empty() {
                lines.push(String::new());
                lines.push("🔥 <b>Destaques recentes:</b>".to_owned());
                for item in active_items.iter().take(3) {
                    let short = truncate_chars(&item.title, 65);
                    lines.push(format!(
                        "• <b>{}</b> — <b>{}</b> · <a href=\"{}\">Ver no ML</a>",
                        escape_telegram_html(&short),
                        escape_telegram_html(&format_brl_cents(item.price_brl_cents)),
                        escape_telegram_html(&item.share_link),
                    ));
                    rows.push(vec![TelegramInlineButton {
                        text: format!("🛒 {}", truncate_chars(&item.title, 36)),
                        url: item.share_link.clone(),
                    }]);
                }
            }
            if let Some(cat_row) = catalog_button_row() {
                rows.push(cat_row);
            }
            let markup = (!rows.is_empty()).then_some(TelegramInlineKeyboardMarkup {
                inline_keyboard: rows,
            });
            (
                truncate_chars(&lines.join("\n"), TELEGRAM_MESSAGE_MAX_CHARS),
                markup,
            )
        }
        TelegramBotCommand::Help => {
            let lines = [
                "ℹ️ <b>Como funciona o Procurali (@procurali)</b>".to_owned(),
                format!("<i>{}</i>", escape_telegram_html(AFFILIATE_DISCLOSURE_PT_BR)),
                String::new(),
                "• Todos os links do Mercado Livre preservam o endereço oficial <code>meli.la</code> sem alterações.".to_owned(),
                "• Preços são registrados com horário de observação em Cuiabá e podem variar após o envio.".to_owned(),
                "• Para WhatsApp: use <code>/whatsapp</code> para copiar o pacote assistido e enviar manualmente no WhatsApp Web (sem API da Meta).".to_owned(),
            ];
            let mut rows = Vec::new();
            if let Some(cat_row) = catalog_button_row() {
                rows.push(cat_row);
            }
            let markup = (!rows.is_empty()).then_some(TelegramInlineKeyboardMarkup {
                inline_keyboard: rows,
            });
            (
                truncate_chars(&lines.join("\n"), TELEGRAM_MESSAGE_MAX_CHARS),
                markup,
            )
        }
        TelegramBotCommand::Offers | TelegramBotCommand::Hardware => {
            if active_items.is_empty() {
                return (
                    "Ainda não há ofertas ativas na fila do Hub no momento. Tente novamente em instantes.".to_owned(),
                    catalog_button_row().map(|row| TelegramInlineKeyboardMarkup {
                        inline_keyboard: vec![row],
                    }),
                );
            }
            let header = if matches!(command, TelegramBotCommand::Hardware) {
                "🖥️ <b>Achados de Hardware e Periféricos</b>"
            } else {
                "⚡ <b>Ofertas ativas verificadas pelo Procurali</b>"
            };
            let mut lines = vec![
                header.to_owned(),
                format!(
                    "<i>{}</i>",
                    escape_telegram_html(AFFILIATE_DISCLOSURE_PT_BR)
                ),
                String::new(),
            ];
            let mut rows = Vec::new();
            for item in active_items.iter().take(4) {
                let disc = item
                    .discount_label
                    .as_deref()
                    .map(|d| format!(" ({})", escape_telegram_html(d)))
                    .unwrap_or_default();
                lines.push(format!(
                    "• <b>{}</b>\n  💰 <b>{}</b>{disc} · Cód: <code>{}</code> · <a href=\"{}\">Abrir oferta</a>",
                    escape_telegram_html(&truncate_chars(&item.title, 85)),
                    escape_telegram_html(&format_brl_cents(item.price_brl_cents)),
                    escape_telegram_html(&item.share_id),
                    escape_telegram_html(&item.share_link),
                ));
                rows.push(vec![TelegramInlineButton {
                    text: format!("🛒 {}", truncate_chars(&item.title, 36)),
                    url: item.share_link.clone(),
                }]);
            }
            if let Some(cat_row) = catalog_button_row() {
                rows.push(cat_row);
            }
            (
                truncate_chars(&lines.join("\n"), TELEGRAM_MESSAGE_MAX_CHARS),
                Some(TelegramInlineKeyboardMarkup {
                    inline_keyboard: rows,
                }),
            )
        }
        TelegramBotCommand::Search(term) => {
            let needle = term.to_ascii_lowercase();
            let matches: Vec<&TelegramOfferQueueItem> = active_items
                .iter()
                .copied()
                .filter(|item| {
                    let hay = format!(
                        "{} {} {} {} {}",
                        item.title,
                        item.brand,
                        item.model.as_deref().unwrap_or(""),
                        item.product_id,
                        item.share_id
                    )
                    .to_ascii_lowercase();
                    hay.contains(&needle)
                })
                .take(4)
                .collect();
            if matches.is_empty() {
                return (
                    format!(
                        "Nenhum achado encontrado para <code>{}</code> agora. Use <code>/ofertas</code> para listar todos os itens ativos.",
                        escape_telegram_html(term)
                    ),
                    catalog_button_row().map(|row| TelegramInlineKeyboardMarkup {
                        inline_keyboard: vec![row],
                    }),
                );
            }
            let mut lines = vec![
                format!(
                    "🔎 <b>Resultados para &quot;{}&quot;</b>",
                    escape_telegram_html(term)
                ),
                format!(
                    "<i>{}</i>",
                    escape_telegram_html(AFFILIATE_DISCLOSURE_PT_BR)
                ),
                String::new(),
            ];
            let mut rows = Vec::new();
            for item in matches {
                lines.push(format!(
                    "• <b>{}</b> — <b>{}</b> · <a href=\"{}\">Ver no ML</a>",
                    escape_telegram_html(&truncate_chars(&item.title, 80)),
                    escape_telegram_html(&format_brl_cents(item.price_brl_cents)),
                    escape_telegram_html(&item.share_link),
                ));
                rows.push(vec![TelegramInlineButton {
                    text: format!("🛒 {}", truncate_chars(&item.title, 36)),
                    url: item.share_link.clone(),
                }]);
            }
            (
                truncate_chars(&lines.join("\n"), TELEGRAM_MESSAGE_MAX_CHARS),
                Some(TelegramInlineKeyboardMarkup {
                    inline_keyboard: rows,
                }),
            )
        }
        TelegramBotCommand::WhatsappDigest => {
            let owned: Vec<TelegramOfferQueueItem> =
                active_items.iter().take(3).copied().cloned().collect();
            let now_ms = owned.first().map(|i| i.updated_at_ms).unwrap_or(1);
            let pkg = build_whatsapp_assisted_package(&owned, "@procurali", site_base_url, now_ms);
            let text = format!(
                "📋 <b>Rascunho pronto para auxiliar humano no WhatsApp Web</b>\n<i>Sem API da Meta e sem automação no WhatsApp Web — copie e revise antes de colar no grupo:</i>\n\n<pre>{}</pre>",
                escape_telegram_html(&truncate_chars(&pkg.combined_digest_text, 3400))
            );
            (truncate_chars(&text, TELEGRAM_MESSAGE_MAX_CHARS), None)
        }
    }
}

pub fn format_telegram_caption_pt_br(product: &ValidatedMercadoLivreProduct) -> String {
    let mut lines = Vec::new();
    lines.push("🚨 OFERTA RELÂMPAGO · ACHADO NO MERCADO LIVRE! 🔥".to_owned());
    lines.push(String::new());
    lines.push(format!("📦 {}", product.title));

    let mut identity_parts = vec![format!("🏷️ Marca: {}", product.brand)];
    if let Some(model) = &product.model {
        identity_parts.push(format!("Modelo: {model}"));
    }
    identity_parts.push(format!("Cód. ML: {}", product.product_id));
    lines.push(identity_parts.join(" · "));
    lines.push(String::new());

    let price_str = format_brl_cents(product.price_brl_cents);
    match product.reference_price_cents {
        Some(ref_cents) if ref_cents > product.price_brl_cents => {
            let savings_cents = ref_cents - product.price_brl_cents;
            let savings_str = format_brl_cents(savings_cents);
            let computed_pct = ((savings_cents as f64 / ref_cents as f64) * 100.0).round() as u32;
            let badge = product
                .discount_label
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| format!("{computed_pct}% OFF"));
            lines.push(format!("❌ De: {}", format_brl_cents(ref_cents)));
            lines.push(format!(
                "✅ POR APENAS: {price_str} 🔥 ({badge} · Economia de {savings_str}!)"
            ));
        }
        _ => {
            if let Some(discount) = product
                .discount_label
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                lines.push(format!("✅ PREÇO EM OFERTA: {price_str} 🔥 ({discount})"));
            } else {
                lines.push(format!("✅ PREÇO EM OFERTA: {price_str} 🔥"));
            }
        }
    }

    lines.push(format!("💳 {}", product.payment_conditions));

    let mut store_parts = vec![format!("🏪 {}", product.seller_name)];
    if let Some(rating) = &product.marketplace_rating {
        let sales = product
            .marketplace_sales_label
            .as_deref()
            .map(|s| format!(" ({s})"))
            .unwrap_or_default();
        store_parts.push(format!("⭐ {rating}{sales}"));
    }
    lines.push(store_parts.join(" · "));

    if let Some(first_limitation) = product.limitations.first() {
        lines.push(format!("⚠️ Atenção antes de comprar: {first_limitation}"));
    }

    lines.push(format!(
        "🕐 Atualizado em {}",
        product.observed_at_display_for_telegram()
    ));
    lines.push(String::new());
    lines.push(
        "⏳ CORRA! Preço e estoque promocional sujeitos a alteração a qualquer momento!".to_owned(),
    );
    lines.push(format!("👉 GARANTA O SEU AQUI: {}", product.share_link));
    lines.push(format!("🔍 Código rápido no app ML: {}", product.share_id));
    lines.push(AFFILIATE_DISCLOSURE_PT_BR.to_owned());
    lines.join("\n")
}

/// Formats Telegram caption lines adding a historical price-drop banner when an offer dropped
/// below its prior tracked capture in the Hub database.
pub fn format_telegram_caption_with_price_history_pt_br(
    product: &ValidatedMercadoLivreProduct,
    comparison: Option<&MercadoLivrePriceComparison>,
) -> String {
    let base = format_telegram_caption_pt_br(product);
    let Some(cmp) = comparison else {
        return base;
    };
    if cmp.trend != PriceTrendKind::Down {
        return base;
    }
    let Some(prev) = cmp.previous_price_cents else {
        return base;
    };
    let drop_line = format!(
        "📉 BAIXOU DE PREÇO! Antes: {} → Agora: {} ({}% vs captura anterior · Mín. 30d: {})",
        format_brl_cents(prev),
        format_brl_cents(cmp.current_price_cents),
        cmp.price_delta_pct,
        format_brl_cents(cmp.lowest_price_30d_cents),
    );
    format!("{drop_line}\n{base}")
}
