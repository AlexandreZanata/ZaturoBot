use std::collections::BTreeSet;
use zaturo_core::{
    MercadoLivreBotProduct, MercadoLivreSpec, MercadoLivreVariant, format_brl_cents,
    no_local_path_leak, parse_brl_amount_cents, valid_slug,
};

pub(super) fn normalize_availability(raw: Option<&str>) -> String {
    raw.map(|s| s.trim().rsplit('/').next().unwrap_or("InStock"))
        .filter(|s| !s.is_empty() && s.len() <= 64 && no_local_path_leak(s))
        .unwrap_or("InStock")
        .to_owned()
}

pub(super) fn build_payment_conditions(
    raw_price_cents: i64,
    pix_price_cents: i64,
    installment_summary: Option<&str>,
) -> String {
    let base = if pix_price_cents < raw_price_cents {
        format!(
            "{} no Pix ou {} no anúncio",
            format_brl_cents(pix_price_cents),
            format_brl_cents(raw_price_cents)
        )
    } else {
        format!("{} à vista", format_brl_cents(raw_price_cents))
    };
    let full = match installment_summary {
        Some(inst) => format!("{base} · {}", clean_text(inst, 80)),
        None => base,
    };
    truncate_utf8(&full, 280)
}

pub(super) fn build_short_condition(
    raw_price_cents: i64,
    pix_price_cents: i64,
    installment_summary: Option<&str>,
    return_days: Option<u16>,
) -> String {
    let mut parts = Vec::new();
    if pix_price_cents < raw_price_cents {
        parts.push("Desconto no Pix".to_owned());
    } else if let Some(inst) = installment_summary {
        parts.push(clean_text(inst, 64));
    } else {
        parts.push("À vista no anúncio".to_owned());
    }
    if let Some(days) = return_days {
        parts.push(format!("devolução grátis {days}d"));
    }
    truncate_utf8(&parts.join(" · "), 160)
}

pub(super) fn build_short_description(
    raw: &MercadoLivreBotProduct,
    specs: &[MercadoLivreSpec],
) -> String {
    if let Some(short) = raw.short_description.as_deref() {
        let cleaned = clean_text(short, 280);
        if !cleaned.is_empty() && no_local_path_leak(&cleaned) {
            return cleaned;
        }
    }
    let summary_specs: Vec<String> = specs
        .iter()
        .take(3)
        .map(|s| format!("{}: {}", s.name, s.value))
        .collect();
    if !summary_specs.is_empty() {
        return truncate_utf8(&summary_specs.join(" · "), 280);
    }
    clean_or_fallback(&raw.title, 280, "Oferta observada no Mercado Livre")
}

pub(super) fn build_long_description(raw: &MercadoLivreBotProduct, fallback: &str) -> String {
    if let Some(long) = raw.long_description.as_deref() {
        let cleaned = clean_text(long, 16_384);
        if !cleaned.is_empty() && no_local_path_leak(&cleaned) {
            return cleaned;
        }
    }
    fallback.to_owned()
}

pub(super) fn infer_for_whom(title: &str, specs: &[MercadoLivreSpec]) -> String {
    let lower = title.to_ascii_lowercase();
    if lower.contains("ssd") || lower.contains("nvme") {
        "Quem quer acelerar notebook ou desktop com slot M.2 compatível.".into()
    } else if lower.contains("teclado") {
        "Quem busca teclado compacto/mecânico com layout e switches compatíveis com seu uso.".into()
    } else if lower.contains("monitor") {
        "Quem precisa de monitor para trabalho ou jogos com taxa de atualização e conexões compatíveis.".into()
    } else if lower.contains("headset") || lower.contains("fone") {
        "Quem busca headset ou fone para chamadas e jogos verificando peso e conexões.".into()
    } else if !specs.is_empty() {
        "Compradores que conferiram ficha técnica, vendedor e compatibilidade para o setup.".into()
    } else {
        "Quem precisa do item descrito conferindo medidas e condições do anúncio.".into()
    }
}

pub(super) fn build_supported_benefits(
    specs: &[MercadoLivreSpec],
    short_description: &str,
    title: &str,
) -> Vec<String> {
    let mut benefits = Vec::new();
    for spec in specs.iter().take(4) {
        benefits.push(truncate_utf8(
            &format!("{}: {}", spec.name, spec.value),
            300,
        ));
    }
    if benefits.is_empty() {
        benefits.push(clean_or_fallback(short_description, 300, title));
    }
    benefits
}

pub(super) fn build_honest_limitations(
    title: &str,
    long_description: &str,
    specs: &[MercadoLivreSpec],
    variants: &[MercadoLivreVariant],
    raw_price_cents: i64,
    installment_summary: Option<&str>,
    card_text: Option<&str>,
) -> Vec<String> {
    let mut limitations = Vec::new();

    // 1. Detect interest-bearing installments when card_text / installment summary lacks "sem juros"
    //    or when (N * installment) meaningfully exceeds cash price.
    if let Some(inst) = installment_summary {
        let inst_lower = inst.to_ascii_lowercase();
        let lacks_sem_juros = !inst_lower.contains("sem juros");
        let total_exceeds_cash =
            parse_installment_total_cents(inst).is_some_and(|total| total > raw_price_cents + 500);
        if lacks_sem_juros || total_exceeds_cash {
            limitations.push(
                "Parcelamento no anúncio inclui acréscimo/juros em relação ao preço à vista."
                    .into(),
            );
        }
    } else if card_text
        .map(|t| t.to_ascii_lowercase())
        .is_some_and(|t| t.contains("com juros"))
    {
        limitations.push(
            "Parcelamento informado no card pode incluir acréscimo em relação ao preço à vista."
                .into(),
        );
    }

    // 2. Detect spec vs description contradictions (water resistance, wireless claims).
    let combined_text = format!("{} {}", title, long_description).to_ascii_lowercase();
    let spec_waterproof_no = specs.iter().any(|s| {
        let n = s.name.to_ascii_lowercase();
        let v = s.value.to_ascii_lowercase();
        (n.contains("resistente à água") || n.contains("resistente a agua") || n.contains("prova"))
            && (v == "não" || v == "nao")
    });
    let text_claims_waterproof = combined_text.contains("resistente a salpicos")
        || combined_text.contains("resistente à água")
        || combined_text.contains("resistente a água")
        || combined_text.contains("à prova d'água");
    if spec_waterproof_no && text_claims_waterproof {
        limitations.push(
            "Divergência no anúncio: a ficha técnica marca resistência à água como 'Não', enquanto a descrição cita proteção contra salpicos."
                .into(),
        );
    }

    let spec_wireless_no = specs.iter().any(|s| {
        s.name.to_ascii_lowercase().contains("sem fio")
            && matches!(s.value.to_ascii_lowercase().as_str(), "não" | "nao")
    });
    if spec_wireless_no
        && (combined_text.contains("bluetooth") || combined_text.contains("tri-mode"))
    {
        limitations.push(
            "Divergência no anúncio: ficha técnica indica 'sem fio: Não', apesar de menção a modo sem fio na descrição."
                .into(),
        );
    }

    // 3. Detect multi-voltage variants requiring explicit selection.
    let voltages: BTreeSet<&str> = variants
        .iter()
        .filter_map(|v| {
            v.voltage
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })
        .collect();
    if voltages.len() > 1 {
        limitations.push(
            "Variantes com voltagens distintas no anúncio; confira a tensão antes de concluir."
                .into(),
        );
    }

    if limitations.is_empty() {
        limitations.push(
            "Condição observada pontualmente; frete e prazo variam conforme CEP no Mercado Livre."
                .into(),
        );
    }
    limitations.truncate(8);
    limitations
}

fn parse_installment_total_cents(summary: &str) -> Option<i64> {
    let lower = summary.to_ascii_lowercase();
    let x_pos = lower.find('×').or_else(|| lower.find('x'))?;
    let count_str: String = lower[..x_pos]
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    let count: i64 = count_str.parse().ok()?;
    let r_pos = summary.find("R$")?;
    let per_inst = parse_brl_amount_cents(&summary[r_pos..])?;
    count.checked_mul(per_inst)
}

pub(super) fn build_unknowns(return_days: Option<u16>, specs: &[MercadoLivreSpec]) -> Vec<String> {
    let mut unknowns = vec![
        "Série contínua de 7 dias ainda em acumulação para comprovação histórica neste lote."
            .into(),
    ];
    if return_days.is_none() {
        unknowns.push("Prazo explícito de devolução não retornado no JSON-LD desta coleta.".into());
    }
    if specs.len() < 3 {
        unknowns.push(
            "Ficha técnica reduzida nesta captura; valide detalhes na página oficial.".into(),
        );
    }
    unknowns
}

pub(super) fn build_tags(brand: &str, title: &str, category_slug: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut tags = Vec::new();

    for raw in std::iter::once("mercado-livre")
        .chain(std::iter::once(category_slug))
        .chain(std::iter::once(brand))
        .chain(title.split_whitespace().take(6))
    {
        let mut slug = String::new();
        let mut prev_dash = true;
        for ch in raw.chars() {
            let mapped = match ch {
                'a'..='z' | '0'..='9' => Some(ch),
                'A'..='Z' => Some(ch.to_ascii_lowercase()),
                'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => Some('a'),
                'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => Some('e'),
                'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => Some('i'),
                'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => Some('o'),
                'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => Some('u'),
                'ç' | 'Ç' => Some('c'),
                _ => None,
            };
            if let Some(c) = mapped {
                if slug.len() < 48 {
                    slug.push(c);
                    prev_dash = false;
                }
            } else if !prev_dash && !slug.is_empty() && slug.len() < 48 {
                slug.push('-');
                prev_dash = true;
            }
        }
        while slug.ends_with('-') {
            slug.pop();
        }
        if valid_slug(&slug) && slug.len() >= 2 && seen.insert(slug.clone()) {
            tags.push(slug);
            if tags.len() >= 10 {
                break;
            }
        }
    }
    tags
}

pub(super) fn clean_text(raw: &str, max_bytes: usize) -> String {
    let sanitized = raw
        .replace('\0', " ")
        .replace("oc_sk_", "[redacted]")
        .replace("-----BEGIN ", "[redacted]")
        .replace("AGE-SECRET-", "[redacted]");
    let collapsed = sanitized.trim();
    let truncated = truncate_utf8(collapsed, max_bytes);
    if no_local_path_leak(&truncated) {
        truncated
    } else {
        // Strip potential local segments conservatively without ever passing a local path upward.
        truncate_utf8(
            &truncated
                .split_whitespace()
                .filter(|tok| no_local_path_leak(tok))
                .collect::<Vec<_>>()
                .join(" "),
            max_bytes,
        )
    }
}

pub(super) fn clean_or_fallback(raw: &str, max_bytes: usize, fallback: &str) -> String {
    let cleaned = clean_text(raw, max_bytes);
    if cleaned.is_empty() {
        truncate_utf8(fallback.trim(), max_bytes)
    } else {
        cleaned
    }
}

pub(super) fn truncate_utf8(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_owned();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].trim_end().to_owned()
}
