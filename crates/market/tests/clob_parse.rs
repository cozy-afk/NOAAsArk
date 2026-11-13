use noaas_ark_market::parse_book;

#[test]
fn parses_real_book_fixture() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/book_sample.json"
    );
    let json = std::fs::read_to_string(path).expect("fixture");
    let snap = parse_book(&json, "fallback").expect("parse");
    assert!(snap.asks.len() > 0);
    assert_eq!(
        snap.token_id,
        "110644309893984406649365829538264362245275741848653731547886452059326929902957"
    );
    assert!(snap.timestamp.timestamp() > 1_577_836_800);
    assert!(snap.best_ask().unwrap() > 0.0);
    // bids empty in fixture -> best_bid None
    assert!(snap.best_bid().is_none());
}
