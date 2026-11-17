use super::*;
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};

pub fn format_utc_sigv4(unix_seconds: i64) -> Result<(String, String), String> {
    let dt = DateTime::<Utc>::from_timestamp(unix_seconds, 0)
        .ok_or_else(|| "invalid UTC timestamp".to_owned())?;
    Ok((
        dt.format("%Y%m%dT%H%M%SZ").to_string(),
        dt.format("%Y%m%d").to_string(),
    ))
}

pub fn format_utc_iso_ms(ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(ms)
        .unwrap_or_default()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

pub fn parse_iso_to_ms(value: &str) -> Result<i64, String> {
    let trimmed = value.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.timestamp_millis());
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%dT%H:%M:%S") {
        return Ok(naive.and_utc().timestamp_millis());
    }
    Err("expected ISO-8601 timestamp".into())
}

pub fn format_cuiaba_short(ms: i64) -> String {
    let offset = FixedOffset::west_opt(4 * 3600).expect("valid UTC-4 offset");
    let dt = DateTime::<Utc>::from_timestamp_millis(ms)
        .unwrap_or_default()
        .with_timezone(&offset);
    dt.format("%d/%m às %Hh%M").to_string()
}

pub fn format_cuiaba_display(ms: i64) -> String {
    let offset = FixedOffset::west_opt(4 * 3600).expect("valid UTC-4 offset");
    let dt = DateTime::<Utc>::from_timestamp_millis(ms)
        .unwrap_or_default()
        .with_timezone(&offset);
    dt.format("%d/%m/%Y às %H:%M (horário de Cuiabá)")
        .to_string()
}

pub fn format_brl_cents(cents: i64) -> String {
    let safe = cents.max(0);
    let reais = safe / 100;
    let rem = safe % 100;
    let whole_str = reais.to_string();
    let mut grouped = String::with_capacity(whole_str.len() + whole_str.len() / 3);
    for (idx, ch) in whole_str.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    let formatted_whole: String = grouped.chars().rev().collect();
    format!("R$ {formatted_whole},{rem:02}")
}

pub fn validate_ml_product_id(value: &str) -> bool {
    value.strip_prefix("MLB").is_some_and(|digits| {
        (6..=16).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit())
    })
}

pub fn validate_meli_share_link(value: &str) -> bool {
    let Some(slug) = value.strip_prefix("https://meli.la/") else {
        return false;
    };
    !slug.is_empty()
        && slug.len() <= 32
        && slug
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

pub fn validate_ml_canonical_url(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    if rest.is_empty()
        || value.len() > 1024
        || !no_local_path_leak(value)
        || value
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '@' | '?' | '#' | '\\'))
    {
        return false;
    }
    let host = rest.split('/').next().unwrap_or("");
    matches!(
        host,
        "www.mercadolivre.com.br" | "mercadolivre.com.br" | "produto.mercadolivre.com.br"
    )
}

pub fn validate_ml_share_id(value: &str) -> bool {
    (4..=32).contains(&value.len())
        && value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
}

pub fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn valid_minio_bucket(value: &str) -> bool {
    (3..=63).contains(&value.len())
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn valid_minio_object_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.starts_with('/')
        && !value.contains("..")
        && !value.contains('\\')
        && no_local_path_leak(value)
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.'))
}

pub fn validate_asset_public_url(value: &str) -> bool {
    if value.len() > 1024
        || !no_local_path_leak(value)
        || value
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || matches!(c, '@' | '?' | '#' | '\\'))
    {
        return false;
    }
    if let Some(rest) = value.strip_prefix("https://") {
        let host = rest.split(['/', ':']).next().unwrap_or("");
        return !host.is_empty() && rest.contains('/');
    }
    if let Some(rest) = value.strip_prefix("http://") {
        return (rest.starts_with("127.0.0.1") || rest.starts_with("[::1]")) && rest.contains('/');
    }
    false
}

pub fn valid_telegram_channel(value: &str) -> bool {
    if let Some(handle) = value.strip_prefix('@') {
        return (5..=32).contains(&handle.len())
            && handle
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_');
    }
    if let Some(id) = value.strip_prefix('-') {
        return (6..=23).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_digit());
    }
    false
}

pub fn no_local_path_leak(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    !value.contains('\0')
        && !lower.contains(".local/")
        && !lower.contains("/home/")
        && !lower.contains("file://")
        && !lower.contains("javascript:")
        && !lower.contains(".secrets")
        && !lower.contains("matt_tool=")
}

pub fn slugify_pt_br(title: &str, product_id: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = true;
    for ch in title.chars() {
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
            if slug.len() < 72 {
                slug.push(c);
                prev_dash = false;
            }
        } else if !prev_dash && slug.len() < 72 {
            slug.push('-');
            prev_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    let suffix = product_id.to_ascii_lowercase();
    if slug.is_empty() {
        suffix
    } else if slug.ends_with(&suffix) {
        slug
    } else {
        format!("{slug}-{suffix}")
    }
}

pub fn extract_product_id_from_url(url: &str) -> Option<String> {
    for part in url.split(['/', '?', '#']) {
        if validate_ml_product_id(part) {
            return Some(part.to_owned());
        }
    }
    None
}

pub fn parse_commission_bp(value: &str) -> Option<u16> {
    let clean = value.trim().trim_end_matches('%').trim().replace(',', ".");
    if clean.is_empty() {
        return None;
    }
    let cents = parse_decimal_to_hundredths(&clean)?;
    u16::try_from(cents).ok().filter(|bp| *bp <= 10_000)
}

pub fn parse_price_value_cents(value: &serde_json::Value) -> Option<i64> {
    match value {
        serde_json::Value::Number(num) => {
            if let Some(whole) = num.as_i64() {
                return whole.checked_mul(100);
            }
            parse_decimal_to_hundredths(&num.to_string())
        }
        serde_json::Value::String(s) => parse_brl_amount_cents(s),
        _ => None,
    }
}

pub fn parse_brl_amount_cents(raw: &str) -> Option<i64> {
    let trimmed = raw
        .trim()
        .trim_start_matches("R$")
        .trim()
        .replace('\u{a0}', "");
    if trimmed.is_empty() {
        return None;
    }
    let normalized = if trimmed.contains(',') {
        trimmed.replace('.', "").replace(',', ".")
    } else if let Some((left, right)) = trimmed.split_once('.') {
        // Distinguish "1.379" (thousands) from "699.90" (cents)
        if right.len() == 3 && left.chars().all(|c| c.is_ascii_digit()) {
            format!("{left}{right}")
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    parse_decimal_to_hundredths(&normalized)
}

fn parse_decimal_to_hundredths(raw: &str) -> Option<i64> {
    let s = raw.trim();
    if s.is_empty() || s.starts_with('-') {
        return None;
    }
    let (whole, frac) = match s.split_once('.') {
        Some((w, f)) => (w, f),
        None => (s, ""),
    };
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
        || frac.len() > 2
    {
        return None;
    }
    let whole_val: i64 = whole.parse().ok()?;
    let frac_val: i64 = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()? * 10,
        2 => frac.parse().ok()?,
        _ => return None,
    };
    whole_val.checked_mul(100)?.checked_add(frac_val)
}

/// Parses raw Mercado Livre search-result card text into structured badges, social proof,
/// price points, discount percent, and installment summary.
pub fn parse_card_text(card_text: &str) -> ParsedCardText {
    let mut parsed = ParsedCardText::default();
    let lines: Vec<&str> = card_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    for line in &lines {
        let upper = line.to_ascii_uppercase();
        if parsed.card_badge.is_none() {
            if upper.contains("MAIS VENDIDO") {
                parsed.card_badge = Some("Mais vendido".into());
            } else if upper.contains("OFERTA DO DIA") {
                parsed.card_badge = Some("Oferta do dia".into());
            } else if upper.contains("MAIS PROCURADO") {
                parsed.card_badge = Some("Mais procurado".into());
            }
        }
        if parsed.marketplace_rating.is_none()
            && line.contains('|')
            && line.to_lowercase().contains("vendid")
        {
            let (left, right) = line.split_once('|').unwrap_or(("", ""));
            let rating_clean = left.trim().replace('.', ",");
            if rating_clean.chars().any(|c| c.is_ascii_digit()) {
                parsed.marketplace_rating = Some(format!("{rating_clean} / 5"));
            }
            let sales = normalize_sales_label(right.trim());
            if !sales.is_empty() {
                parsed.marketplace_sales_label = Some(sales);
            }
        }
        if parsed.discount_label.is_none() && upper.contains("% OFF") {
            parsed.discount_label = Some(line.trim().to_owned());
        }
    }

    // Reconstruct both split ("R$\n699\n,\n90") and inline ("R$ 699,90") price tokens.
    let mut amounts_cents: Vec<i64> = Vec::new();
    let mut idx = 0;
    while idx < lines.len() {
        let line = lines[idx];
        let prev_is_installment = idx > 0
            && (lines[idx - 1].to_ascii_lowercase().ends_with('x')
                || lines[idx - 1].to_ascii_lowercase().ends_with('×'));
        if line == "R$" && idx + 1 < lines.len() {
            let mut amount = lines[idx + 1].to_owned();
            let mut consumed = 2;
            if idx + 3 < lines.len() && lines[idx + 2] == "," {
                amount.push(',');
                amount.push_str(lines[idx + 3]);
                consumed = 4;
            } else if idx + 2 < lines.len() && lines[idx + 2].starts_with(',') {
                amount.push_str(lines[idx + 2]);
                consumed = 3;
            }
            if let Some(cents) = parse_brl_amount_cents(&amount) {
                if !prev_is_installment {
                    amounts_cents.push(cents);
                } else if parsed.installment_summary.is_none() {
                    let count_tok = lines[idx - 1].replace('x', "×");
                    let has_sem_juros = lines[idx + consumed..lines.len().min(idx + consumed + 3)]
                        .iter()
                        .any(|l| l.to_ascii_lowercase().contains("sem juros"));
                    let suffix = if has_sem_juros { " sem juros" } else { "" };
                    parsed.installment_summary = Some(format!(
                        "{count_tok} de {}{suffix}",
                        format_brl_cents(cents)
                    ));
                }
            }
            idx += consumed;
            continue;
        }
        if let Some(pos) = line.find("R$") {
            let after = line[pos + 2..].trim();
            let num_token: String = after
                .chars()
                .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | ','))
                .collect();
            if let Some(cents) = parse_brl_amount_cents(&num_token) {
                let lower = line.to_ascii_lowercase();
                if let Some((prefix, _)) = lower.split_once("r$")
                    && (prefix.contains("x ") || prefix.contains("× "))
                {
                    if parsed.installment_summary.is_none() {
                        parsed.installment_summary = Some(line.trim().to_owned());
                    }
                } else {
                    amounts_cents.push(cents);
                }
            }
        }
        idx += 1;
    }

    if amounts_cents.len() >= 2 && amounts_cents[0] > amounts_cents[1] {
        parsed.reference_price_cents = Some(amounts_cents[0]);
        parsed.card_price_cents = Some(amounts_cents[1]);
    } else if let Some(first) = amounts_cents.first() {
        parsed.card_price_cents = Some(*first);
    }

    parsed
}

fn normalize_sales_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let with_space = trimmed
        .replace("mil ", " mil ")
        .replace("milvend", " mil vend");
    if with_space.to_ascii_lowercase().contains("no ml") {
        with_space
    } else {
        format!("{with_space} no ML")
    }
}

pub fn parse_jsonld_variant(raw: &serde_json::Value) -> Option<MercadoLivreVariant> {
    let obj = raw.as_object()?;
    let name = obj.get("name")?.as_str()?.trim().to_owned();
    let sku = obj
        .get("sku")
        .or_else(|| obj.get("productID"))
        .and_then(|v| v.as_str())?
        .trim()
        .to_owned();
    let variant_product_id = obj
        .get("productID")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| sku.clone());
    let color = obj
        .get("color")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());

    let offers = obj.get("offers").and_then(|v| v.as_object());
    let price_brl_cents = offers
        .and_then(|o| o.get("price"))
        .and_then(parse_price_value_cents)
        .filter(|c| *c > 0)?;
    let currency = offers
        .and_then(|o| o.get("priceCurrency"))
        .and_then(|v| v.as_str())
        .unwrap_or("BRL")
        .to_owned();
    let availability = offers
        .and_then(|o| o.get("availability"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().rsplit('/').next().unwrap_or("InStock").to_owned())
        .unwrap_or_else(|| "InStock".into());
    let policy = offers
        .and_then(|o| o.get("hasMerchantReturnPolicy"))
        .and_then(|v| v.as_object());
    let return_days = policy
        .and_then(|p| p.get("merchantReturnDays"))
        .and_then(|v| v.as_u64())
        .and_then(|d| u16::try_from(d).ok());
    let return_fees = policy
        .and_then(|p| p.get("returnFees"))
        .and_then(|v| v.as_str())
        .map(|s| {
            if s.contains("FreeReturn") {
                "Grátis".into()
            } else {
                s.rsplit('/')
                    .next()
                    .unwrap_or("Conforme anúncio")
                    .to_owned()
            }
        });
    let canonical_url = offers
        .and_then(|o| o.get("url"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .filter(|u| validate_ml_canonical_url(u));

    let mut voltage: Option<String> = None;
    let mut highlights = Vec::new();
    if let Some(props) = obj.get("additionalProperty").and_then(|v| v.as_array()) {
        for prop in props {
            let Some(p) = prop.as_object() else { continue };
            let pname = p.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
            let pval = p.get("value").and_then(|v| v.as_str()).unwrap_or("").trim();
            if pname.is_empty() && pval.is_empty() {
                continue;
            }
            if pname.eq_ignore_ascii_case("Voltagem") && !pval.is_empty() {
                voltage = Some(pval.to_owned());
            }
            if !pval.is_empty() && highlights.len() < 4 {
                highlights.push(pval.to_owned());
            }
        }
    }

    Some(MercadoLivreVariant {
        variant_product_id,
        sku,
        name,
        color,
        voltage,
        price_brl_cents,
        currency,
        availability,
        return_days,
        return_fees,
        canonical_url,
        highlights,
    })
}
