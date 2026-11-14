use zaturo_core::commerce::money::{Currency, Money};

#[test]
fn money_parses_brazilian_and_dot_decimals() {
    for raw in ["89,90", "89.90"] {
        let money = Money::parse_decimal(raw, Currency::Brl).unwrap();
        assert_eq!(money.minor, 8990);
        assert_eq!(money.format_decimal(), "89,90");
        assert!(money.validate().is_ok());
    }
    assert_eq!(Money::parse_decimal("7", Currency::Brl).unwrap().minor, 700);
    assert_eq!(
        Money::parse_decimal("0,1", Currency::Brl).unwrap().minor,
        10
    );
    assert_eq!(
        Money::parse_decimal("0.00", Currency::Brl).unwrap().minor,
        0
    );
    let max = Money::parse_decimal("92233720368547758.07", Currency::Brl).unwrap();
    assert_eq!(max.minor, i64::MAX);
    assert_eq!(max.format_decimal(), "92233720368547758,07");
}

#[test]
fn money_rejects_overflow_and_float_inputs() {
    for raw in [
        "",
        "-1",
        "+1",
        "1e2",
        "NaN",
        "inf",
        "1.234",
        "89,901",
        "1,2.3",
        "1_000",
        " 1",
        "1 ",
        ".50",
        "1.",
        "92233720368547758.08",
        "999999999999999999999999999999999999999999999",
    ] {
        assert!(Money::parse_decimal(raw, Currency::Brl).is_err(), "{raw}");
    }
    assert!(serde_json::from_str::<Money>(r#"{"minor":89.9,"currency":"BRL"}"#).is_err());
    assert!(serde_json::from_str::<Money>(r#"{"minor":1,"currency":"USD"}"#).is_err());
    assert!(serde_json::from_str::<Money>(r#"{"minor":1,"currency":"BRL","extra":true}"#).is_err());
    assert!(
        Money {
            minor: -1,
            currency: Currency::Brl
        }
        .validate()
        .is_err()
    );
}
