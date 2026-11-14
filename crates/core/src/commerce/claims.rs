//! Conservative checks for the bounded listing template, not a general truth classifier.
use super::{AffiliateProgram, Offer};
use chrono::{DateTime, Utc};
use chrono_tz::America::Cuiaba;
use serde::{Deserialize, Serialize};

pub const MAX_CONTENT_BYTES: usize = 1500;
pub const DEFAULT_DISCLOSURE_TEXT: &str =
    "Publicidade: este conteúdo contém link de afiliado; podemos receber comissão.";

#[derive(Debug, Clone, Copy)]
pub struct ClaimContext<'a> {
    pub now_millis: i64,
    pub max_price_age_millis: i64,
    pub channel_program: AffiliateProgram,
    pub disclosure_text: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimViolation {
    InvalidOffer,
    MissingDisclosure,
    MissingBenefit,
    FirstPersonTestClaim,
    UnsupportedSuperlative,
    UnsupportedDiscount,
    StalePriceClaim,
    ComparativeClaim,
    UnverifiedNumber,
    ForeignUrl,
    ProgramMismatch,
    UnsafeMarkup,
    UnsafeText,
    OversizedText,
}

/// Lists every applicable rule once, in a stable order. Facts must already have provenance.
pub fn lint_claims(text: &str, offer: &Offer, context: ClaimContext<'_>) -> Vec<ClaimViolation> {
    // Bound admission before allocating semantic-check buffers for untrusted content.
    if text.len() > MAX_CONTENT_BYTES {
        return vec![ClaimViolation::OversizedText];
    }
    if offer.validate(context.now_millis).is_err() {
        return vec![ClaimViolation::InvalidOffer];
    }
    let mut violations = Vec::new();
    let age = context.now_millis.checked_sub(offer.price_observed_at);
    let valid_context = context.max_price_age_millis > 0
        && age.is_some_and(|age| age >= 0)
        && valid_disclosure(context.disclosure_text);
    if !valid_context {
        violations.push(ClaimViolation::InvalidOffer);
    }
    let decoded = decode_html(text);
    let link_position = decoded.find(&offer.affiliate_url);
    let disclosure_position = decoded.find(context.disclosure_text);
    if text.lines().next() != Some("#publi")
        || !valid_disclosure(context.disclosure_text)
        || !decoded.lines().any(|line| line == context.disclosure_text)
        || !disclosure_position
            .zip(link_position)
            .is_some_and(|(disclosure, link)| disclosure < link)
    {
        violations.push(ClaimViolation::MissingDisclosure);
    }
    if !decoded.lines().any(|line| {
        line.strip_prefix("- ")
            .is_some_and(|line| offer.benefits.iter().any(|benefit| benefit == line))
    }) {
        violations.push(ClaimViolation::MissingBenefit);
    }
    let urls = url_like_tokens(&decoded);
    let without_urls = urls
        .iter()
        .fold(decoded.clone(), |body, url| body.replace(url, " "));
    let normalized = normalize(&without_urls);
    if contains_phrase(
        &normalized,
        &[
            "testei",
            "testamos",
            "usei",
            "usamos",
            "na minha experiencia",
            "testado por mim",
            "na nossa experiencia",
            "minha experiencia",
            "nossa experiencia",
        ],
    ) {
        violations.push(ClaimViolation::FirstPersonTestClaim);
    }
    if contains_phrase(
        &normalized,
        &[
            "o melhor",
            "a melhor",
            "os melhores",
            "as melhores",
            "menor preco da historia",
            "melhor do mercado",
            "o menor preco",
            "o mais barato",
            "a mais barata",
            "imbativel",
            "garantido",
        ],
    ) {
        violations.push(ClaimViolation::UnsupportedSuperlative);
    }
    // No reference-price contract exists in this slice, so no discount is supportable.
    if without_urls.contains(['%', '％', '﹪', '٪'])
        || contains_phrase(
            &normalized,
            &[
                "desconto",
                "descontos",
                "off",
                "economize",
                "economia de",
                "por cento",
            ],
        )
    {
        violations.push(ClaimViolation::UnsupportedDiscount);
    }
    if offer.program == AffiliateProgram::MercadoLivre
        && contains_phrase(
            &normalized,
            &[
                "mais barato que",
                "mais barata que",
                "mais barato do que",
                "mais barata do que",
                "melhor que",
                "melhor do que",
                "superior a",
                "inferior a",
                "em comparacao",
                "comparado a",
                "comparada a",
                "versus",
                "vs",
                "concorrente",
                "concorrentes",
            ],
        )
    {
        violations.push(ClaimViolation::ComparativeClaim);
    }
    let mut fact_text = format!("{}\n{}", offer.title, offer.benefits.join("\n"));
    let trusted_price_line =
        if valid_context && age.is_some_and(|age| age <= context.max_price_age_millis) {
            fact_text.push_str(&format!("\n{}", offer.price.format_decimal()));
            fresh_price_line(offer)
        } else {
            None
        };
    // Only this exact renderer-owned line may contain observation-date numbers.
    // Date tokens are never evidence for a warranty, count or other product claim.
    let numeric_body = without_urls
        .lines()
        .filter(|line| Some(*line) != trusted_price_line.as_deref())
        .collect::<Vec<_>>()
        .join("\n");
    let allowed_numbers = number_tokens(&fact_text);
    if valid_context
        && age.is_some_and(|age| age > context.max_price_age_millis)
        && has_price_claim(&without_urls.replace("confira o preço atual", ""), offer)
    {
        violations.push(ClaimViolation::StalePriceClaim);
    }
    if number_tokens(&numeric_body)
        .iter()
        .any(|number| !allowed_numbers.contains(number))
    {
        violations.push(ClaimViolation::UnverifiedNumber);
    }
    if urls.is_empty() || urls.iter().any(|url| *url != offer.affiliate_url) {
        violations.push(ClaimViolation::ForeignUrl);
    }
    if context.channel_program != offer.program {
        violations.push(ClaimViolation::ProgramMismatch);
    }
    if text.contains(['<', '>']) {
        violations.push(ClaimViolation::UnsafeMarkup);
    }
    if has_unsafe_text(text, true) {
        violations.push(ClaimViolation::UnsafeText);
    }
    violations
}

pub(crate) fn observation_timestamp(millis: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(millis).map(|time| {
        time.with_timezone(&Cuiaba)
            .format("%Y-%m-%d %H:%M America/Cuiaba")
            .to_string()
    })
}

pub(crate) fn fresh_price_line(offer: &Offer) -> Option<String> {
    Some(format!(
        "R$ {} — observado em {}",
        offer.price.format_decimal(),
        observation_timestamp(offer.price_observed_at)?
    ))
}

pub(crate) fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn decode_html(text: &str) -> String {
    // Decode only entities emitted by our renderer, once. Nested escapes stay literal.
    let mut decoded = String::with_capacity(text.len());
    let mut remaining = text;
    while !remaining.is_empty() {
        let entity = [
            ("&amp;", '&'),
            ("&lt;", '<'),
            ("&gt;", '>'),
            ("&quot;", '"'),
            ("&#39;", '\''),
        ]
        .into_iter()
        .find(|(entity, _)| remaining.starts_with(entity));
        if let Some((entity, character)) = entity {
            decoded.push(character);
            remaining = &remaining[entity.len()..];
        } else if let Some(character) = remaining.chars().next() {
            decoded.push(character);
            remaining = &remaining[character.len_utf8()..];
        }
    }
    decoded
}

pub(crate) fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|character| match character {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c if c.is_alphanumeric() || c == '%' => c,
            _ => ' ',
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn contains_phrase(text: &str, phrases: &[&str]) -> bool {
    let bounded = format!(" {text} ");
    phrases
        .iter()
        .any(|phrase| bounded.contains(&format!(" {phrase} ")))
}

pub(crate) fn valid_disclosure(text: &str) -> bool {
    if text.is_empty() || text.len() > 240 {
        return false;
    }
    // F1 accepts reviewed affirmative wording only. This checks presentation, not
    // the legal truth of an account's affiliation or its production permission.
    text == DEFAULT_DISCLOSURE_TEXT
        || text == "Link de afiliado: posso receber comissão se você comprar."
}

fn has_price_claim(text: &str, offer: &Offer) -> bool {
    if text.to_lowercase().contains("r$")
        || contains_phrase(
            &normalize(text),
            &[
                "brl", "reais", "real", "preco", "precos", "custa", "custando", "valor",
            ],
        )
    {
        return true;
    }
    // Check bare decimal representations of the old price as well as currency
    // wording. Do not classify integer specifications or USB 3.0 as money merely
    // because a price happens to be three reais.
    let decimal = offer.price.format_decimal();
    let mut amounts = vec![decimal.clone(), decimal.replace(',', ".")];
    let shorter = decimal.trim_end_matches('0');
    if shorter.contains(',') && !shorter.ends_with(',') {
        amounts.push(shorter.to_owned());
        amounts.push(shorter.replace(',', "."));
    }
    number_tokens(text)
        .iter()
        .any(|token| amounts.contains(token))
}

pub(crate) fn has_unsafe_text(text: &str, allow_newline: bool) -> bool {
    text.chars().any(|character| {
        (character.is_control() && !(allow_newline && character == '\n'))
            || matches!(character, '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{206f}' | '\u{feff}' | '\u{00ad}' | '\u{0300}'..='\u{036f}')
    })
}

pub(crate) fn number_tokens(text: &str) -> Vec<String> {
    let mut numbers = Vec::new();
    let mut current = String::new();
    let characters: Vec<_> = text.chars().collect();
    for (index, character) in characters.iter().copied().enumerate() {
        if character.is_numeric()
            || (!current.is_empty()
                && matches!(character, '.' | ',')
                && characters
                    .get(index + 1)
                    .is_some_and(|next| next.is_numeric()))
        {
            current.push(character);
        } else if !current.is_empty() {
            numbers.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        numbers.push(current);
    }
    numbers
}

pub(crate) fn url_like_tokens(text: &str) -> Vec<&str> {
    text.split(|character: char| {
        character.is_whitespace() || matches!(character, '<' | '>' | '"' | '\'')
    })
    .filter(|token| {
        let lower = token.to_ascii_lowercase();
        lower.contains("://")
            || lower.contains("www.")
            || lower.starts_with("mailto:")
            || lower.starts_with("javascript:")
            || lower.starts_with("data:")
            || token.rsplit_once('.').is_some_and(|(prefix, suffix)| {
                prefix.chars().any(char::is_alphabetic)
                    && suffix.len() >= 2
                    && suffix.chars().all(char::is_alphabetic)
            })
    })
    .collect()
}
