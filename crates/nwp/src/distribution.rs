// crates/engine/src/distribution.rs
//
// Continuous distribution fitting for ensemble members.
// Replaces raw member-counting (n/N) with a parametric probability that
// carries an explicit uncertainty estimate (delta method).

/// Fitted distribution over ensemble values.
#[derive(Debug, Clone)]
pub struct EnsembleDistribution {
    pub mean: f64,
    pub std_dev: f64,
    pub n_members: usize,
    /// Multiplier applied to the sample std (ensemble underdispersion correction).
    pub spread_inflation: f64,
}

impl EnsembleDistribution {
    pub fn fit(ensemble_values: &[f64], spread_inflation: f64) -> Self {
        let n = ensemble_values.len() as f64;
        if ensemble_values.is_empty() {
            return Self {
                mean: 0.0,
                std_dev: 1.0,
                n_members: 0,
                spread_inflation: spread_inflation.max(1e-9),
            };
        }
        let mean = ensemble_values.iter().sum::<f64>() / n;
        let variance = if n > 1.0 {
            ensemble_values
                .iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>()
                / (n - 1.0)
        } else {
            0.0
        };
        let std_dev = variance.sqrt() * spread_inflation.max(1e-9);
        Self {
            mean,
            std_dev: std_dev.max(1e-9),
            n_members: ensemble_values.len(),
            spread_inflation: spread_inflation.max(1e-9),
        }
    }

    /// P(X >= threshold) under the fitted normal.
    pub fn prob_exceed(&self, threshold: f64) -> f64 {
        // survival function: 1 - Phi((threshold-mean)/std)
        let z = (threshold - self.mean) / self.std_dev;
        1.0 - standard_normal_cdf(z)
    }

    /// P(low <= X <= high) under the fitted normal. A `None` side is unbounded.
    pub fn prob_in_range(&self, low: Option<f64>, high: Option<f64>) -> f64 {
        let cdf_high = match high {
            Some(h) => standard_normal_cdf((h - self.mean) / self.std_dev),
            None => 1.0,
        };
        let cdf_low = match low {
            Some(l) => standard_normal_cdf((l - self.mean) / self.std_dev),
            None => 0.0,
        };
        (cdf_high - cdf_low).clamp(0.0, 1.0)
    }

    /// Standard error of the probability estimate (delta method).
    pub fn probability_uncertainty(&self, threshold: f64) -> f64 {
        if self.n_members == 0 {
            return 1.0;
        }
        let z = (threshold - self.mean) / self.std_dev;
        // standard normal pdf at z
        let pdf_at_z = (-z * z / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt();
        // derivatives
        let d_prob_d_mean = pdf_at_z / self.std_dev;
        let mean_se = self.std_dev / (self.n_members as f64).sqrt();
        let var_from_mean = (d_prob_d_mean * mean_se).powi(2);

        let d_prob_d_std = pdf_at_z * z / self.std_dev;
        let std_se = self.std_dev / (2.0 * self.n_members as f64).sqrt();
        let var_from_std = (d_prob_d_std * std_se).powi(2);

        (var_from_mean + var_from_std).sqrt()
    }

    /// Standard error of the P(low <= X <= high) estimate (delta method).
    /// Same structure as `probability_uncertainty` with phi/z differences.
    pub fn probability_uncertainty_range(&self, low: Option<f64>, high: Option<f64>) -> f64 {
        if self.n_members == 0 {
            return 1.0;
        }
        let phi = |z: f64| (-z * z / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt();
        let (phi_h, z_h) = match high {
            Some(h) => {
                let z = (h - self.mean) / self.std_dev;
                (phi(z), z)
            }
            None => (0.0, 0.0),
        };
        let (phi_l, z_l) = match low {
            Some(l) => {
                let z = (l - self.mean) / self.std_dev;
                (phi(z), z)
            }
            None => (0.0, 0.0),
        };
        let d_prob_d_mean = (phi_h - phi_l) / self.std_dev;
        let mean_se = self.std_dev / (self.n_members as f64).sqrt();
        let d_prob_d_std = (z_h * phi_h - z_l * phi_l) / self.std_dev;
        let std_se = self.std_dev / (2.0 * self.n_members as f64).sqrt();
        ((d_prob_d_mean * mean_se).powi(2) + (d_prob_d_std * std_se).powi(2)).sqrt()
    }

    pub fn z_score(&self, threshold: f64) -> f64 {
        (self.mean - threshold) / self.std_dev
    }
}

/// Standard normal CDF via Abramowitz & Stegun erfc approximation.
fn standard_normal_cdf(z: f64) -> f64 {
    0.5 * erfc(-z / std::f64::consts::SQRT_2)
}

fn erfc(x: f64) -> f64 {
    if x >= 0.0 {
        erfc_pos(x)
    } else {
        2.0 - erfc_pos(-x)
    }
}

fn erfc_pos(x: f64) -> f64 {
    let p = 0.3275911_f64;
    let a1 = 0.254829592_f64;
    let a2 = -0.284496736_f64;
    let a3 = 1.421413741_f64;
    let a4 = -1.453152027_f64;
    let a5 = 1.061405429_f64;
    let t = 1.0 / (1.0 + p * x);
    1.0 - (((((a5 * t + a4) * t + a3) * t + a2) * t + a1) * t) * (-x * x).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prob_exceed_monotonic() {
        let vals: Vec<f64> = (0..31).map(|i| 88.0 + i as f64 * 0.3).collect();
        let d = EnsembleDistribution::fit(&vals, 1.0);
        let p90 = d.prob_exceed(90.0);
        let p95 = d.prob_exceed(95.0);
        assert!(p90 > p95, "prob should decrease with higher threshold");
        assert!((0.0..=1.0).contains(&p90));
    }

    #[test]
    fn uncertainty_positive_and_finite() {
        let vals: Vec<f64> = vec![88.0, 89.0, 90.0, 91.0, 92.0];
        let d = EnsembleDistribution::fit(&vals, 1.2);
        let u = d.probability_uncertainty(90.0);
        assert!(u.is_finite() && u >= 0.0);
    }

    #[test]
    fn empty_fit_safe() {
        let d = EnsembleDistribution::fit(&[], 1.0);
        assert!(d.prob_exceed(90.0).is_finite());
        assert!((0.0..=1.0).contains(&d.prob_exceed(90.0)));
    }

    #[test]
    fn range_covers_bracket_tiling() {
        // Brackets "41 or below" / "42-43" tile at 41.5; probs must sum to ~1.
        let vals: Vec<f64> = (0..31).map(|i| 40.0 + i as f64 * 0.2).collect();
        let d = EnsembleDistribution::fit(&vals, 1.0);
        let p_below = d.prob_in_range(None, Some(41.5));
        let p_mid = d.prob_in_range(Some(41.5), Some(43.5));
        let p_above = d.prob_in_range(Some(43.5), None);
        let total = p_below + p_mid + p_above;
        assert!(
            (total - 1.0).abs() < 1e-9,
            "tiled brackets sum to 1, got {total}"
        );
        assert!((0.0..=1.0).contains(&p_mid));
    }

    #[test]
    fn range_uncertainty_finite() {
        let vals: Vec<f64> = vec![40.0, 41.0, 42.0, 43.0, 44.0];
        let d = EnsembleDistribution::fit(&vals, 1.0);
        let u = d.probability_uncertainty_range(Some(41.5), Some(43.5));
        assert!(u.is_finite() && u >= 0.0);
        let u_open = d.probability_uncertainty_range(None, Some(41.5));
        assert!(u_open.is_finite() && u_open >= 0.0);
    }
}
