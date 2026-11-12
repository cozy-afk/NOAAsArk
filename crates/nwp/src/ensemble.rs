use noaas_ark_core::types::WeatherMetric;

/// Converts raw ensemble values into a calibrated probability.
pub struct EnsembleForecaster {
    bias_correction: BiasCorrection,
}

#[derive(Debug, Clone)]
pub struct BiasCorrection {
    /// Additive bias: model tends to be X degrees too warm/cold
    pub mean_bias: f64,
    /// Spread correction: model spread too narrow/wide
    pub spread_factor: f64,
}

impl Default for BiasCorrection {
    fn default() -> Self {
        Self {
            mean_bias: 0.0,
            spread_factor: 1.0,
        }
    }
}

impl EnsembleForecaster {
    pub fn new(bias_correction: BiasCorrection) -> Self {
        Self { bias_correction }
    }

    pub fn with_default_bias() -> Self {
        Self::new(BiasCorrection::default())
    }

    /// P(metric exceeds threshold) from ensemble members (Kelvin for temp).
    pub fn probability(&self, ensemble_values: &[f64], metric: &WeatherMetric) -> f64 {
        if ensemble_values.is_empty() {
            return 0.5;
        }
        let converted: Vec<f64> = ensemble_values
            .iter()
            .map(|&v| self.convert_and_correct(v, metric))
            .collect();

        let threshold = match metric {
            WeatherMetric::MaxTempF { threshold } => *threshold,
            WeatherMetric::MinTempF { threshold } => *threshold,
            WeatherMetric::PrecipitationIn { threshold } => *threshold,
            WeatherMetric::SnowfallIn { threshold } => *threshold,
            WeatherMetric::WindSpeedMph { threshold } => *threshold,
        };

        let exceed_count = converted.iter().filter(|&&v| v >= threshold).count();
        let raw_prob = exceed_count as f64 / converted.len() as f64;
        // Clamp away from 0/1 so the logit is finite.
        let raw_clamped = raw_prob.clamp(0.01, 0.99);
        self.apply_spread_correction(raw_clamped)
    }

    fn convert_and_correct(&self, value: f64, metric: &WeatherMetric) -> f64 {
        match metric {
            WeatherMetric::MaxTempF { .. } | WeatherMetric::MinTempF { .. } => {
                let fahrenheit = (value - 273.15) * 9.0 / 5.0 + 32.0;
                fahrenheit - self.bias_correction.mean_bias
            }
            _ => value - self.bias_correction.mean_bias,
        }
    }

    fn apply_spread_correction(&self, raw_prob: f64) -> f64 {
        if self.bias_correction.spread_factor <= 0.0 {
            return raw_prob;
        }
        let p = raw_prob.clamp(1e-6, 1.0 - 1e-6);
        let logit = (p / (1.0 - p)).ln();
        let corrected_logit = logit / self.bias_correction.spread_factor;
        1.0 / (1.0 + (-corrected_logit).exp())
    }
}
