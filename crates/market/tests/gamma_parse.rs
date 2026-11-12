use chrono::{TimeZone, Utc};
use noaas_ark_core::types::WeatherMetric;
use noaas_ark_market::markets_from_events_json;

fn fixture_json() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/gamma_sample.json"
    );
    std::fs::read_to_string(path).expect("fixture must exist")
}

#[test]
fn parses_open_brackets_into_markets() {
    let now = Utc.with_ymd_and_hms(2026, 11, 12, 0, 0, 0).unwrap();
    let (markets, stats) = markets_from_events_json(&fixture_json(), now).unwrap();

    // 3 NYC brackets + 1 Chicago bracket + 2 London brackets parse;
    // Jinan skipped (unknown city).
    assert_eq!(markets.len(), 6, "stats: {stats:?}");
    assert_eq!(stats.parsed, 6);
    assert_eq!(stats.skipped_closed, 1);
    assert_eq!(stats.skipped_ended, 1);
    assert_eq!(stats.skipped_unparseable, 1);
    assert_eq!(stats.skipped_unknown_city, 1);

    let below = markets
        .iter()
        .find(|m| m.condition_id == "0xaaa111")
        .expect("below bracket");
    assert_eq!(below.station.id, "KLGA");
    assert_eq!(below.token_id_yes, "1001");
    assert_eq!(below.token_id_no, "1002");
    assert!(matches!(
        below.metric,
        WeatherMetric::MaxTempRangeF {
            low: None,
            high: Some(h)
        } if (h - 41.5).abs() < 1e-9
    ));

    // Reversed ["No","Yes"] outcomes still map yes/no tokens correctly.
    let above = markets
        .iter()
        .find(|m| m.condition_id == "0xccc333")
        .expect("above bracket");
    assert_eq!(above.token_id_yes, "1006");
    assert_eq!(above.token_id_no, "1005");

    let chi = markets
        .iter()
        .find(|m| m.condition_id == "0xeee555")
        .expect("chicago bracket");
    assert_eq!(chi.station.id, "KORD");
    assert!(matches!(
        chi.metric,
        WeatherMetric::MinTempRangeF {
            low: Some(_),
            high: Some(_)
        }
    ));

    // Celsius point bracket converted to Fahrenheit edges at EGLC.
    let ldn = markets
        .iter()
        .find(|m| m.condition_id == "0xiii999")
        .expect("london point bracket");
    assert_eq!(ldn.station.id, "EGLC");
    match &ldn.metric {
        WeatherMetric::MinTempRangeF {
            low: Some(l),
            high: Some(h),
        } => {
            assert!((l - 34.7).abs() < 1e-9, "low={l}");
            assert!((h - 36.5).abs() < 1e-9, "high={h}");
        }
        other => panic!("unexpected metric: {other:?}"),
    }
    let ldn_high = markets
        .iter()
        .find(|m| m.condition_id == "0xjjj000")
        .expect("london higher bracket");
    assert_eq!(ldn_high.station.id, "EGLC");
}

#[test]
fn malformed_json_errors() {
    let now = Utc.with_ymd_and_hms(2026, 11, 12, 0, 0, 0).unwrap();
    assert!(markets_from_events_json("not json", now).is_err());
}
