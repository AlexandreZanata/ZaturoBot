use zaturo_core::commerce::{
    attribution::{AttributionError, AttributionMap},
    offer::AffiliateProgram,
};

#[test]
fn sub_id_attribution_maps_known_ids_only() {
    let mut map = AttributionMap::new(AffiliateProgram::Shopee, 64, 2).unwrap();
    let first = map.generate("content-one").unwrap();
    assert_eq!(first.len(), 64);
    assert!(first.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(map.lookup(&first), Some("content-one"));
    assert_eq!(map.sub_id_for("content-one"), Some(first.as_str()));
    assert_eq!(map.generate("content-one").unwrap(), first);
    assert_eq!(map.lookup("unknown-official-sub-id"), None);
    let second = map.generate("content-two").unwrap();
    assert_ne!(first, second);
    assert_eq!(
        map.generate("content-three"),
        Err(AttributionError::Capacity)
    );
}

#[test]
fn attribution_refuses_conflicts_invalid_bounds_and_hash_truncation() {
    let mut map = AttributionMap::new(AffiliateProgram::MercadoLivre, 12, 2).unwrap();
    assert_eq!(
        map.generate("content-one"),
        Err(AttributionError::SubIdLimit)
    );
    map.register("content-one", "official-one").unwrap();
    map.register("content-one", "official-one").unwrap();
    assert_eq!(
        map.register("content-two", "official-one"),
        Err(AttributionError::Conflict)
    );
    assert_eq!(
        map.register("content-one", "official-two"),
        Err(AttributionError::Conflict)
    );
    assert!(map.register("content-two", "not/url-safe").is_err());
    assert!(AttributionMap::new(AffiliateProgram::Shopee, 0, 1).is_err());
    assert!(AttributionMap::new(AffiliateProgram::Shopee, 64, 1001).is_err());
    let mut other = AttributionMap::new(AffiliateProgram::Shopee, 64, 1).unwrap();
    let mut mercado = AttributionMap::new(AffiliateProgram::MercadoLivre, 64, 1).unwrap();
    assert_ne!(
        other.generate("same-content").unwrap(),
        mercado.generate("same-content").unwrap()
    );
}
