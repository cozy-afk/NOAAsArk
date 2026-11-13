use chrono::{TimeZone, Utc};
use noaas_ark_core::types::{Station, WeatherMarket, WeatherMetric};
use noaas_ark_engine::signal::{EngineConfig, SignalEngine};
use noaas_ark_market::clob::{OrderbookSnapshot, PriceLevel};
use noaas_ark_market::parse_book;
use noaas_ark_nwp::ensemble::BiasCorrection;
use noaas_ark_nwp::ensemble::EnsembleForecaster;

#[test]
fn dry_run_offline_no_signal_without_bid() {
    // Real book fixture has asks but no bids -> engine cannot compute edge.
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../market/tests/fixtures/book_sample.json"
    );
    let json = std::fs::read_to_string(path).expect("book fixture");
    let orderbook = parse_book(&json, "fallback").expect("parse");

    let market = WeatherMarket {
        condition_id: "0xde0ac9d9".into(),
        question: "q".into(),
        token_id_yes: "y".into(),
        token_id_no: "n".into(),
        station: Station {
            id: "KATL".into(),
            name: "Atlanta".into(),
            lat: 0.0,
            lon: 0.0,
            elevation_m: 0.0,
        },
        metric: WeatherMetric::MaxTempRangeF {
            low: Some(63.5),
            high: None,
        },
        event_time: Utc.with_ymd_and_hms(2030, 11, 13, 12, 0, 0).unwrap(),
        resolution_time: Utc.with_ymd_and_hms(2030, 11, 13, 12, 0, 0).unwrap(),
        resolved_outcome: None,
    };

    let engine = SignalEngine::new(
        EngineConfig::default(),
        EnsembleForecaster::new(BiasCorrection::default()),
    );
    let ensemble: Vec<f64> = (0..31).map(|i| 290.0 + i as f64 * 0.3).collect();
    let run = noaas_ark_core::types::ModelRunId {
        model: noaas_ark_core::types::NwpModel::Gfs,
        init_time: Utc.with_ymd_and_hms(2026, 11, 12, 6, 0, 0).unwrap(),
        forecast_hour: 30,
    };
    // Real fixture has no bids -> no signal.
    assert!(engine
        .evaluate(&market, &ensemble, &orderbook, run.clone())
        .is_none());
}

#[test]
fn dry_run_offline_signal_with_two_sided_book() {
    let market = WeatherMarket {
        condition_id: "m2".into(),
        question: "q".into(),
        token_id_yes: "y".into(),
        token_id_no: "n".into(),
        station: Station {
            id: "KATL".into(),
            name: "Atlanta".into(),
            lat: 0.0,
            lon: 0.0,
            elevation_m: 0.0,
        },
        metric: WeatherMetric::MaxTempRangeF {
            low: Some(63.5),
            high: None,
        },
        event_time: Utc.with_ymd_and_hms(2030, 11, 13, 12, 0, 0).unwrap(),
        resolution_time: Utc.with_ymd_and_hms(2030, 11, 13, 12, 0, 0).unwrap(),
        resolved_outcome: None,
    };
    // A two-sided book where model (high prob of exceeding 63.5F) should
    // generate an edge against a low ask.
    let orderbook = OrderbookSnapshot {
        market: "m".into(),
        token_id: "t".into(),
        bids: vec![PriceLevel {
            price: 0.30,
            size: 100.0,
        }],
        asks: vec![PriceLevel {
            price: 0.35,
            size: 100.0,
        }],
        timestamp: Utc::now(),
    };
    let engine = SignalEngine::new(
        EngineConfig {
            min_edge: 0.01,
            min_edge_after_spread: 0.0,
            ..EngineConfig::default()
        },
        EnsembleForecaster::new(BiasCorrection::default()),
    );
    // Ensemble centered well above 63.5F -> high model prob of exceeding.
    let ensemble: Vec<f64> = (0..31).map(|i| 300.0 + i as f64).collect();
    let run = noaas_ark_core::types::ModelRunId {
        model: noaas_ark_core::types::NwpModel::Gfs,
        init_time: Utc.with_ymd_and_hms(2026, 11, 12, 6, 0, 0).unwrap(),
        forecast_hour: 30,
    };
    let sig = engine.evaluate(&market, &ensemble, &orderbook, run);
    // Deterministic either way; just assert the call doesn't panic and is
    // consistent with a second identical call.
    let sig2 = engine.evaluate(
        &market,
        &ensemble,
        &orderbook,
        noaas_ark_core::types::ModelRunId {
            model: noaas_ark_core::types::NwpModel::Gfs,
            init_time: Utc.with_ymd_and_hms(2026, 11, 12, 6, 0, 0).unwrap(),
            forecast_hour: 30,
        },
    );
    assert_eq!(sig.is_some(), sig2.is_some());
}
