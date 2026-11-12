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

    /// P(metric exceeds threshold) from ensemble members.
    /// Uses continuous distribution fitting (normal) instead of raw member counts,
    /// which removes the severe sampling noise of n/N estimates.
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
            WeatherMetric::MaxTempRangeF { low, high }
            | WeatherMetric::MinTempRangeF { low, high } => {
                let dist = crate::distribution::EnsembleDistribution::fit(
                    &converted,
                    self.bias_correction.spread_factor,
                );
                return dist.prob_in_range(*low, *high);
            }
        };

        let dist = crate::distribution::EnsembleDistribution::fit(
            &converted,
            self.bias_correction.spread_factor,
        );
        dist.prob_exceed(threshold)
    }

    /// Fit (and return) the distribution used for probability, for regime/uncertainty callers.
    pub fn fit_distribution(
        &self,
        ensemble_values: &[f64],
        metric: &WeatherMetric,
    ) -> crate::distribution::EnsembleDistribution {
        if ensemble_values.is_empty() {
            return crate::distribution::EnsembleDistribution::fit(
                &[],
                self.bias_correction.spread_factor,
            );
        }
        let converted: Vec<f64> = ensemble_values
            .iter()
            .map(|&v| self.convert_and_correct(v, metric))
            .collect();
        crate::distribution::EnsembleDistribution::fit(
            &converted,
            self.bias_correction.spread_factor,
        )
    }
    pub fn probability_with_uncertainty(
        &self,
        ensemble_values: &[f64],
        metric: &WeatherMetric,
    ) -> (f64, f64) {
        if ensemble_values.is_empty() {
            return (0.5, 1.0);
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
            WeatherMetric::MaxTempRangeF { low, high }
            | WeatherMetric::MinTempRangeF { low, high } => {
                let dist = crate::distribution::EnsembleDistribution::fit(
                    &converted,
                    self.bias_correction.spread_factor,
                );
                return (
                    dist.prob_in_range(*low, *high),
                    dist.probability_uncertainty_range(*low, *high),
                );
            }
        };
        let dist = crate::distribution::EnsembleDistribution::fit(
            &converted,
            self.bias_correction.spread_factor,
        );
        (
            dist.prob_exceed(threshold),
            dist.probability_uncertainty(threshold),
        )
    }

    fn convert_and_correct(&self, value: f64, metric: &WeatherMetric) -> f64 {
        match metric {
            WeatherMetric::MaxTempF { .. }
            | WeatherMetric::MinTempF { .. }
            | WeatherMetric::MaxTempRangeF { .. }
            | WeatherMetric::MinTempRangeF { .. } => {
                let fahrenheit = (value - 273.15) * 9.0 / 5.0 + 32.0;
                fahrenheit - self.bias_correction.mean_bias
            }
            _ => value - self.bias_correction.mean_bias,
        }
    }
}
