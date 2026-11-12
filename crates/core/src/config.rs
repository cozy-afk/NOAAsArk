use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    #[serde(default = "default_engine")]
    pub engine: EngineConfig,
    #[serde(default = "default_risk")]
    pub risk: RiskConfig,
    #[serde(default)]
    pub clob: ClobConfig,
    #[serde(default = "default_nwp")]
    pub nwp: NwpConfig,
    #[serde(default = "default_calibration")]
    pub calibration: CalibrationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub min_edge: f64,
    pub min_edge_after_spread: f64,
    pub max_position_usdc: f64,
    pub kelly_fraction: f64,
    pub min_hours_to_resolution: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub max_total_exposure: f64,
    pub max_per_market: f64,
    pub max_daily_loss: f64,
    pub max_concurrent_positions: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClobConfig {
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_secret: String,
    #[serde(default)]
    pub private_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NwpConfig {
    #[serde(default = "default_models")]
    pub models: Vec<String>,
    pub gfs_weight: f64,
    pub ecmwf_weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationConfig {
    pub bias_file: String,
}

fn default_engine() -> EngineConfig {
    EngineConfig {
        min_edge: 0.05,
        min_edge_after_spread: 0.03,
        max_position_usdc: 25.0,
        kelly_fraction: 0.25,
        min_hours_to_resolution: 3.0,
    }
}

fn default_risk() -> RiskConfig {
    RiskConfig {
        max_total_exposure: 200.0,
        max_per_market: 50.0,
        max_daily_loss: 75.0,
        max_concurrent_positions: 8,
    }
}

fn default_models() -> Vec<String> {
    vec!["gfs".to_string(), "ecmwf".to_string()]
}

fn default_nwp() -> NwpConfig {
    NwpConfig {
        models: default_models(),
        gfs_weight: 0.4,
        ecmwf_weight: 0.6,
    }
}

fn default_calibration() -> CalibrationConfig {
    CalibrationConfig {
        bias_file: "data/calibration/bias_params.json".to_string(),
    }
}

impl BotConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading config {:?}", path.as_ref()))?;
        let cfg: BotConfig =
            toml::from_str(&content).with_context(|| "parsing TOML config")?;
        Ok(cfg)
    }

    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        Self::load(path).unwrap_or_else(|_| Self::default_fallback())
    }

    fn default_fallback() -> Self {
        Self {
            engine: default_engine(),
            risk: default_risk(),
            clob: ClobConfig::default(),
            nwp: default_nwp(),
            calibration: default_calibration(),
        }
    }
}
