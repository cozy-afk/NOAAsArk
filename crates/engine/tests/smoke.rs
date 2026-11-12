use noaas_ark_engine::signal::{EngineConfig, SignalEngine};

#[test]
fn smoke_engine_with_defaults() {
    let engine = SignalEngine::with_defaults();
    let _ = engine;
}

#[test]
fn smoke_engine_config_default() {
    let cfg = EngineConfig::default();
    assert!(cfg.min_edge > 0.0);
    assert!(cfg.kelly_fraction > 0.0);
    assert!(cfg.max_position_usdc > 0.0);
}
