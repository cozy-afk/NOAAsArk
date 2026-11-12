//! Bias correction learned from a calibration study.
//! File format: {"mean_bias": f64, "spread_factor": f64}

use anyhow::Result;
use noaas_ark_nwp::ensemble::BiasCorrection;

pub fn load_bias_correction(path: &str) -> Result<BiasCorrection> {
    let content = std::fs::read_to_string(path)?;
    let v: serde_json::Value = serde_json::from_str(&content)?;
    Ok(BiasCorrection {
        mean_bias: v.get("mean_bias").and_then(|x| x.as_f64()).unwrap_or(0.0),
        spread_factor: v
            .get("spread_factor")
            .and_then(|x| x.as_f64())
            .unwrap_or(1.0),
    })
}

pub fn load_or_default(path: &str) -> BiasCorrection {
    load_bias_correction(path).unwrap_or(BiasCorrection {
        mean_bias: 0.0,
        spread_factor: 1.0,
    })
}
