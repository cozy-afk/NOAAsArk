// crates/nwp/src/station_bias.rs
//
// Station-specific bias correction. Polymarket resolves on a station
// observation; the model grid point differs by representativeness error.
// Learn per-station bias from paired (model, observation) history.

use std::f64::consts::PI;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StationBias {
    pub station_id: String,
    /// Mean difference: observation - model (in the model's units, e.g. degF).
    /// Positive = model runs cold relative to this station.
    pub mean_bias: f64,
    /// Seasonal amplitude (cosine).
    pub seasonal_amplitude: f64,
    pub seasonal_phase: f64,
    /// Bias growth per 24h of forecast range.
    pub range_dependence: f64,
    /// Residual std after correction.
    pub residual_std: f64,
    pub n_samples: usize,
}

impl StationBias {
    pub fn correction(&self, day_of_year: u32, forecast_hour: u32) -> f64 {
        let seasonal = self.seasonal_amplitude
            * ((2.0 * PI * day_of_year as f64 / 365.0) + self.seasonal_phase).cos();
        let range_effect = self.range_dependence * (forecast_hour as f64 / 24.0);
        self.mean_bias + seasonal + range_effect
    }

    /// Apply correction to a model value: corrected = model + correction.
    pub fn correct(&self, model_value: f64, day_of_year: u32, forecast_hour: u32) -> f64 {
        model_value + self.correction(day_of_year, forecast_hour)
    }
}

impl Default for StationBias {
    fn default() -> Self {
        Self {
            station_id: String::new(),
            mean_bias: 0.0,
            seasonal_amplitude: 0.0,
            seasonal_phase: 0.0,
            range_dependence: 0.0,
            residual_std: 1.0,
            n_samples: 0,
        }
    }
}

/// Learn a station bias from paired (model_value, obs_value, day_of_year, forecast_hour).
pub fn learn_station_bias(station_id: &str, pairs: &[(f64, f64, u32, u32)]) -> StationBias {
    if pairs.is_empty() {
        return StationBias {
            station_id: station_id.to_string(),
            ..Default::default()
        };
    }

    let n = pairs.len() as f64;
    // mean bias (obs - model)
    let mean_bias = pairs.iter().map(|(m, o, _, _)| o - m).sum::<f64>() / n;

    // Seasonal fit: residual = a*cos(theta) + b*sin(theta), theta = 2pi*doy/365
    let mut s_cos = 0.0;
    let mut s_sin = 0.0;
    let mut s_cc = 0.0;
    let mut s_ss = 0.0;
    let mut s_cs = 0.0;
    for (m, o, doy, _) in pairs {
        let r = (o - m) - mean_bias;
        let theta = 2.0 * PI * *doy as f64 / 365.0;
        let c = theta.cos();
        let s = theta.sin();
        s_cos += r * c;
        s_sin += r * s;
        s_cc += c * c;
        s_ss += s * s;
        s_cs += c * s;
    }
    // Solve 2x2 normal equations for a,b
    let det = s_cc * s_ss - s_cs * s_cs;
    let (a, b) = if det.abs() > 1e-12 {
        let a = (s_cos * s_ss - s_cs * s_sin) / det;
        let b = (s_cc * s_sin - s_cos * s_cs) / det;
        (a, b)
    } else {
        (0.0, 0.0)
    };
    let seasonal_amplitude = (a * a + b * b).sqrt();
    let seasonal_phase = b.atan2(a);

    // Range dependence: linear regression slope of (obs-model) vs forecast_hour
    // (scaled so range_dependence is bias change per 24h).
    let mean_fh = pairs.iter().map(|(_, _, _, fh)| *fh as f64).sum::<f64>() / n;
    let mut num = 0.0;
    let mut den = 0.0;
    for (m, o, _, fh) in pairs {
        let x = *fh as f64 - mean_fh;
        let y = (o - m) - mean_bias;
        num += x * y;
        den += x * x;
    }
    let slope_per_hour = if den.abs() > 1e-12 { num / den } else { 0.0 };
    let range_dependence = slope_per_hour * 24.0;

    // Residual std after full correction
    let mut sq = 0.0;
    for (m, o, doy, fh) in pairs {
        let corr = mean_bias
            + seasonal_amplitude * ((2.0 * PI * *doy as f64 / 365.0) + seasonal_phase).cos()
            + (range_dependence * (*fh as f64 / 24.0));
        let resid = (o - m) - corr;
        sq += resid * resid;
    }
    let residual_std = (sq / n).sqrt().max(1e-9);

    StationBias {
        station_id: station_id.to_string(),
        mean_bias,
        seasonal_amplitude,
        seasonal_phase,
        range_dependence,
        residual_std,
        n_samples: pairs.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correction_runs() {
        let sb = StationBias {
            station_id: "KNYC".into(),
            mean_bias: 2.0,
            seasonal_amplitude: 1.0,
            seasonal_phase: 0.0,
            range_dependence: 0.5,
            residual_std: 1.5,
            n_samples: 10,
        };
        let c = sb.correction(180, 24);
        assert!(c.is_finite());
    }

    #[test]
    fn learn_empty_safe() {
        let sb = learn_station_bias("X", &[]);
        assert_eq!(sb.n_samples, 0);
        assert_eq!(sb.mean_bias, 0.0);
    }

    #[test]
    fn learn_recovers_mean_bias() {
        // model consistently 3 deg cold
        let pairs: Vec<(f64, f64, u32, u32)> = (0..100)
            .map(|i| (80.0, 83.0, (i % 365) as u32 + 1, 24))
            .collect();
        let sb = learn_station_bias("KNYC", &pairs);
        assert!((sb.mean_bias - 3.0).abs() < 1e-9);
        assert!(sb.residual_std <= 1e-9);
    }
}
