use noaas_ark_core::BotConfig;
use std::path::PathBuf;

#[test]
fn smoke_load_default_config() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cfg_path = manifest.join("../../config/default.toml");
    let cfg = BotConfig::load(&cfg_path).expect("default config should parse");
    assert!(cfg.engine.min_edge > 0.0);
    assert!(cfg.risk.max_total_exposure > 0.0);
}

#[test]
fn smoke_config_fallback() {
    let cfg = BotConfig::load_or_default("does/not/exist.toml");
    assert!(cfg.engine.min_edge > 0.0);
}
