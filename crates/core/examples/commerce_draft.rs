//! Local synthetic F1 demonstration: no account, provider, persistence or send.
use zaturo_core::commerce::{
    AffiliateProgram, BrandText, Currency, DEFAULT_DISCLOSURE_TEXT, OfferInput, TemplateId, render,
};

fn main() {
    let now = 1_794_658_800_000;
    let offer = OfferInput {
        program: AffiliateProgram::Shopee,
        item_id: "fixture-usb-hub".into(),
        title: "Hub USB de exemplo".into(),
        category: "accessories".into(),
        price: "79,90".into(),
        currency: Currency::Brl,
        observed_at: Some(now - 60_000),
        valid_until: Some(now + 3_600_000),
        affiliate_url: "https://s.shopee.com.br/fixture-offer".into(),
        commission_bp: None,
        sub_id: None,
        evidence_sha256: "ab".repeat(32),
        benefits: vec!["Conector USB-C descrito na ficha fictícia".into()],
    }
    .validate(now)
    .expect("the bounded synthetic offer must validate");
    let brand = BrandText {
        name: "Exemplo local".into(),
        revision: "fixture-v1".into(),
        disclosure_text: DEFAULT_DISCLOSURE_TEXT.into(),
    };
    let draft = render(TemplateId::ListingV1, &offer, &brand, now, 21_600_000)
        .expect("the synthetic listing template must pass claim validation");
    println!("Demonstração local: dados fictícios, sem publicação.\n");
    println!("{}", draft.text);
}
