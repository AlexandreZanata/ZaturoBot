use zaturo_core::commerce::{
    AffiliateProgram, ClaimContext, ClaimViolation, Currency, DEFAULT_DISCLOSURE_TEXT, Money,
    Offer, lint_claims,
};

const NOW: i64 = 1_794_658_800_000;

fn offer() -> Offer {
    Offer {
        program: AffiliateProgram::Shopee,
        item_id: "fixture-17".into(),
        title: "Hub USB 3.0 com 4 portas".into(),
        category: "accessories".into(),
        price: Money {
            minor: 8990,
            currency: Currency::Brl,
        },
        price_observed_at: NOW - 60_000,
        valid_until: NOW + 86_400_000,
        affiliate_url: "https://s.shopee.com.br/fixture17".into(),
        commission_bp: None,
        sub_id: None,
        evidence_sha256: "a".repeat(64),
        benefits: vec!["Conexão USB 3.0".into(), "4 portas".into()],
    }
}
fn context() -> ClaimContext<'static> {
    ClaimContext {
        now_millis: NOW,
        max_price_age_millis: 21_600_000,
        channel_program: AffiliateProgram::Shopee,
        disclosure_text: DEFAULT_DISCLOSURE_TEXT,
    }
}
fn post(extra: &str) -> String {
    format!(
        "#publi\n{DEFAULT_DISCLOSURE_TEXT}\nHub USB 3.0 com 4 portas\n- Conexão USB 3.0\n{extra}\nhttps://s.shopee.com.br/fixture17"
    )
}

#[test]
fn claim_lint_rejects_testing_claims_superlatives_and_unbacked_discounts() {
    for claim in [
        "Eu TESTEI",
        "Usei por meses",
        "Na minha EXPERIÊNCIA",
        "O MELHOR",
        "menor preço da história",
        "20% de desconto",
        "20 por cento de desconto",
    ] {
        assert!(
            !lint_claims(&post(claim), &offer(), context()).is_empty(),
            "{claim}"
        );
    }
    let violations = lint_claims(
        &post("testei; o melhor; 20% de desconto"),
        &offer(),
        context(),
    );
    assert!(violations.contains(&ClaimViolation::FirstPersonTestClaim));
    assert!(violations.contains(&ClaimViolation::UnsupportedSuperlative));
    assert!(violations.contains(&ClaimViolation::UnsupportedDiscount));
    assert_eq!(
        violations
            .iter()
            .filter(|v| **v == ClaimViolation::UnsupportedDiscount)
            .count(),
        1
    );
}

#[test]
fn disclosure_must_precede_link_and_survive_truncation() {
    let text = post("");
    assert!(lint_claims(&text, &offer(), context()).is_empty());
    for text in [
        text.replacen("#publi\n", "", 1),
        format!("{}\n#publi", post("").trim_start_matches("#publi\n")),
        "\n#publi\n- 4 portas".into(),
        post("").replacen("#publi", "#publi publicidade", 1),
    ] {
        assert!(
            lint_claims(&text, &offer(), context()).contains(&ClaimViolation::MissingDisclosure)
        );
    }
    assert!(
        text.chars()
            .take(6)
            .collect::<String>()
            .starts_with("#publi")
    );
}

#[test]
fn post_without_publi_first_line_or_benefit_line_is_rejected() {
    assert!(
        lint_claims(
            "#publi\nhttps://s.shopee.com.br/fixture17",
            &offer(),
            context()
        )
        .contains(&ClaimViolation::MissingBenefit)
    );
    assert!(
        lint_claims(
            &post("").replace("- Conexão USB 3.0", "- Produto incrível"),
            &offer(),
            context()
        )
        .contains(&ClaimViolation::MissingBenefit)
    );
}

#[test]
fn foreign_program_link_is_rejected() {
    let text = format!("{}\nhttps://mercadolivre.com.br/item", post(""));
    assert!(lint_claims(&text, &offer(), context()).contains(&ClaimViolation::ForeignUrl));
    let mut wrong_channel = context();
    wrong_channel.channel_program = AffiliateProgram::MercadoLivre;
    assert!(
        lint_claims(&post(""), &offer(), wrong_channel).contains(&ClaimViolation::ProgramMismatch)
    );
}

#[test]
fn claims_reject_unverified_numbers_comparisons_hidden_text_and_markup() {
    assert!(
        lint_claims(&post("Suporta 999 portas"), &offer(), context())
            .contains(&ClaimViolation::UnverifiedNumber)
    );
    for malicious in [
        "<a href=\"https://evil.invalid\">4 portas</a>",
        "<b>bom</b>",
        "te\u{200b}stei",
        "\u{202e}preço",
    ] {
        assert!(!lint_claims(&post(malicious), &offer(), context()).is_empty());
    }
    let mut mercado = offer();
    mercado.program = AffiliateProgram::MercadoLivre;
    mercado.affiliate_url = "https://mercadolivre.com/sec/fixture17".into();
    let ctx = ClaimContext {
        channel_program: AffiliateProgram::MercadoLivre,
        ..context()
    };
    let text = format!(
        "#publi\n{DEFAULT_DISCLOSURE_TEXT}\n- 4 portas\nMais barato que o concorrente\n{}",
        mercado.affiliate_url
    );
    assert!(lint_claims(&text, &mercado, ctx).contains(&ClaimViolation::ComparativeClaim));
    assert!(
        lint_claims(
            &post(""),
            &offer(),
            ClaimContext {
                now_millis: NOW - 120_000,
                ..context()
            }
        )
        .contains(&ClaimViolation::InvalidOffer)
    );
}

#[test]
fn generated_timestamp_numbers_cannot_be_reused_as_product_claims() {
    for claim in ["14 meses de garantia", "Garantia de 2026 meses"] {
        assert!(
            lint_claims(&post(claim), &offer(), context())
                .contains(&ClaimViolation::UnverifiedNumber),
            "{claim}"
        );
    }
}

#[test]
fn every_url_must_be_the_exact_approved_offer_link() {
    for extra in [
        "http://s.shopee.com.br/fixture17",
        "https://s.shopee.com.br/fixture17/evil",
        "https://s.shopee.com.br/fixture17?tag=other",
        "https://evil.invalid/https://s.shopee.com.br/fixture17",
        "www.evil.invalid",
    ] {
        assert!(
            lint_claims(&post(extra), &offer(), context()).contains(&ClaimViolation::ForeignUrl),
            "{extra}"
        );
    }
}

#[test]
fn unsupported_percent_is_rejected_even_when_number_is_an_offer_fact() {
    for expression in [
        "4% hoje",
        "4 % hoje",
        "4 por cento hoje",
        "4％ hoje",
        "4﹪ hoje",
        "4٪ hoje",
    ] {
        assert!(
            lint_claims(&post(expression), &offer(), context())
                .contains(&ClaimViolation::UnsupportedDiscount),
            "{expression}"
        );
    }
}

#[test]
fn approved_url_slugs_and_percent_encoding_do_not_become_product_claims() {
    let mut offer = offer();
    offer.affiliate_url = "https://s.shopee.com.br/off/testei?ref=4%25".into();
    let text = format!(
        "#publi\n{DEFAULT_DISCLOSURE_TEXT}\n- 4 portas\n{}",
        offer.affiliate_url
    );
    assert!(lint_claims(&text, &offer, context()).is_empty());
}

#[test]
fn affiliate_disclosure_does_not_need_to_repeat_publicity_hashtag() {
    let disclosure = "Link de afiliado: posso receber comissão se você comprar.";
    let text = post("").replace(DEFAULT_DISCLOSURE_TEXT, disclosure);
    assert!(
        lint_claims(
            &text,
            &offer(),
            ClaimContext {
                disclosure_text: disclosure,
                ..context()
            }
        )
        .is_empty()
    );
    assert!(
        lint_claims(
            &text.replace("#publi\n", ""),
            &offer(),
            ClaimContext {
                disclosure_text: disclosure,
                ..context()
            }
        )
        .contains(&ClaimViolation::MissingDisclosure)
    );
}

#[test]
fn stale_price_claims_are_rejected_even_when_imported_facts_whitelist_the_number() {
    let mut stale = offer();
    stale.price_observed_at = NOW - 21_600_001;
    stale.title = "Hub USB por 89.90".into();
    assert!(
        lint_claims(&post("Hub USB por 89.90"), &stale, context())
            .contains(&ClaimViolation::StalePriceClaim)
    );
    let mut stale = offer();
    stale.price_observed_at = NOW - 21_600_001;
    assert!(
        lint_claims(&post("R$ 4"), &stale, context()).contains(&ClaimViolation::StalePriceClaim)
    );
}

#[test]
fn negative_or_misleading_affiliation_cannot_replace_a_disclosure() {
    for disclosure in [
        "Não somos afiliados e não recebemos comissão.",
        "Publicidade: afiliados oficiais garantem comissão para quem comprar.",
    ] {
        let text = post("").replace(DEFAULT_DISCLOSURE_TEXT, disclosure);
        assert!(
            lint_claims(
                &text,
                &offer(),
                ClaimContext {
                    disclosure_text: disclosure,
                    ..context()
                }
            )
            .contains(&ClaimViolation::MissingDisclosure)
        );
    }
}
