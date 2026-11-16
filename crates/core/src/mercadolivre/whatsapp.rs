use super::telegram::truncate_chars;
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsappAssistedDraft {
    pub delivery_id: String,
    pub product_id: String,
    pub slug: String,
    pub title: String,
    pub price_brl_cents: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_price_cents: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_label: Option<String>,
    pub share_link: String,
    pub share_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_public_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site_offer_url: Option<String>,
    pub whatsapp_text: String,
    pub operator_checklist: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatsappAssistedPackage {
    pub generated_at: String,
    pub mode: String,
    pub channel_id: String,
    pub draft_count: usize,
    pub combined_digest_text: String,
    pub drafts: Vec<WhatsappAssistedDraft>,
}

fn sanitize_whatsapp_inline(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(['*', '_', '~', '`'], "")
        .trim()
        .to_owned()
}

pub fn build_whatsapp_assisted_draft(
    item: &TelegramOfferQueueItem,
    site_base_url: Option<&str>,
) -> WhatsappAssistedDraft {
    let slug = slugify_pt_br(&item.title, &item.product_id);
    let site_offer_url = build_site_offer_url(site_base_url, &item.title, &item.product_id);
    let safe_title = sanitize_whatsapp_inline(&item.title);
    let safe_brand = sanitize_whatsapp_inline(&item.brand);
    let price_str = format_brl_cents(item.price_brl_cents);

    let price_line = match (item.reference_price_cents, item.discount_label.as_deref()) {
        (Some(ref_cents), Some(discount)) if ref_cents > item.price_brl_cents => {
            let safe_disc = sanitize_whatsapp_inline(discount);
            format!(
                "💰 *{price_str}* (~De {}~ · *{safe_disc}*)",
                format_brl_cents(ref_cents)
            )
        }
        (Some(ref_cents), None) if ref_cents > item.price_brl_cents => {
            format!("💰 *{price_str}* (~De {}~)", format_brl_cents(ref_cents))
        }
        (_, Some(discount)) => {
            let safe_disc = sanitize_whatsapp_inline(discount);
            format!("💰 *{price_str}* (*{safe_disc}*)")
        }
        _ => format!("💰 *{price_str}*"),
    };

    let mut lines = vec![
        format!("_{}_", AFFILIATE_DISCLOSURE_PT_BR),
        String::new(),
        format!("🔥 *{safe_title}*"),
        match &item.model {
            Some(model) => format!(
                "Marca: *{safe_brand}* · Modelo: *{}*",
                sanitize_whatsapp_inline(model)
            ),
            None => format!("Marca: *{safe_brand}*"),
        },
        price_line,
    ];

    for extra in item.message_text.lines().filter(|line| {
        let t = line.trim();
        t.starts_with("📉") || t.starts_with("💳") || t.starts_with("🏪") || t.starts_with("⚠️")
    }) {
        lines.push(sanitize_whatsapp_inline(extra));
    }

    lines.push(format!(
        "_Observado em {}_",
        format_cuiaba_display(item.updated_at_ms)
    ));
    lines.push(String::new());
    lines.push(format!("🔎 Código ML: ```{}```", item.share_id.trim()));
    lines.push(format!("🔗 Oferta oficial: {}", item.share_link.trim()));
    if let Some(url) = &site_offer_url {
        lines.push(format!("📋 Ficha completa no Procurali: {url}"));
    }

    let operator_checklist = vec![
        "Envio humano assistido no WhatsApp Web (proibido usar API da Meta ou automação não oficial no WhatsApp Web).".to_owned(),
        format!(
            "Conferir se preço ({price_str}) e estoque no link oficial {} ainda estão válidos (< 30 min).",
            item.share_link
        ),
        match &item.image_public_url {
            Some(img) => format!("Opcional: anexar foto estática verificada do MinIO ({img}) ou aguardar preview Open Graph do link Procurali."),
            None => "Sem foto estática: aguardar card de pré-visualização Open Graph do link Procurali ao colar no WhatsApp Web.".to_owned(),
        },
        "Colar a mensagem manualmente no grupo autorizado pelo proprietário no WhatsApp Web e revisar antes de apertar Enviar.".to_owned(),
    ];

    WhatsappAssistedDraft {
        delivery_id: item.delivery_id.clone(),
        product_id: item.product_id.clone(),
        slug,
        title: item.title.clone(),
        price_brl_cents: item.price_brl_cents,
        reference_price_cents: item.reference_price_cents,
        discount_label: item.discount_label.clone(),
        share_link: item.share_link.clone(),
        share_id: item.share_id.clone(),
        image_public_url: item.image_public_url.clone(),
        site_offer_url,
        whatsapp_text: lines.join("\n"),
        operator_checklist,
    }
}

pub fn build_whatsapp_assisted_package(
    items: &[TelegramOfferQueueItem],
    channel_id: &str,
    site_base_url: Option<&str>,
    now_ms: i64,
) -> WhatsappAssistedPackage {
    let mut seen_products = std::collections::HashSet::with_capacity(items.len());
    let drafts: Vec<WhatsappAssistedDraft> = items
        .iter()
        .filter(|item| seen_products.insert(item.product_id.as_str()))
        .map(|item| build_whatsapp_assisted_draft(item, site_base_url))
        .collect();

    let combined_digest_text = if drafts.is_empty() {
        format!(
            "_Procurali · Resumo manual para WhatsApp Web_\nNenhuma oferta elegível na fila do canal {}.",
            sanitize_whatsapp_inline(channel_id)
        )
    } else {
        let mut digest = vec![
            format!(
                "*Seleção Procurali · Achados verificados* ({})",
                format_cuiaba_short(now_ms)
            ),
            format!("_{}_", AFFILIATE_DISCLOSURE_PT_BR),
            String::new(),
        ];
        for (idx, draft) in drafts.iter().enumerate() {
            let safe_title = sanitize_whatsapp_inline(&truncate_chars(&draft.title, 90));
            let price_str = format_brl_cents(draft.price_brl_cents);
            let badge = draft
                .discount_label
                .as_deref()
                .map(|d| format!(" ({})", sanitize_whatsapp_inline(d)))
                .unwrap_or_default();
            digest.push(format!("{}. *{safe_title}*", idx + 1));
            digest.push(format!(
                "   💰 *{price_str}*{badge} · 🔗 {}",
                draft.share_link
            ));
            if let Some(url) = &draft.site_offer_url {
                digest.push(format!("   📋 Ficha: {url}"));
            }
        }
        digest.push(String::new());
        digest.push(
            "_Revisão humana obrigatória no WhatsApp Web antes de enviar (sem API da Meta)._"
                .to_owned(),
        );
        digest.join("\n")
    };

    WhatsappAssistedPackage {
        generated_at: format_utc_iso_ms(now_ms),
        mode: "whatsapp_web_human_assisted_no_meta_api".to_owned(),
        channel_id: channel_id.to_owned(),
        draft_count: drafts.len(),
        combined_digest_text,
        drafts,
    }
}
