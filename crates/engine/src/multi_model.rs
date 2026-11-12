// crates/engine/src/multi_model.rs
//
// Regime-dependent multi-model blending (Problem 5).
// Different models excel in different conditions; weights are learned per regime.

use noaas_ark_core::types::NwpModel;

#[derive(Debug, Clone)]
pub struct RegimeBonus {
    pub regime: String,
    pub multiplier: f64,
}

#[derive(Debug, Clone)]
pub struct ModelWeight {
    pub model: NwpModel,
    pub base_weight: f64,
    pub range_decay: f64,
    pub regime_bonus: Vec<RegimeBonus>,
}

pub struct ModelBlender {
    pub weights: Vec<ModelWeight>,
}

impl ModelBlender {
    pub fn new(weights: Vec<ModelWeight>) -> Self {
        Self { weights }
    }

    /// Blend (model, prob, uncertainty) tuples into (blended_prob, blended_uncertainty).
    pub fn blend(
        &self,
        model_probs: &[(NwpModel, f64, f64)],
        forecast_hour: u32,
        regime: &str,
    ) -> (f64, f64) {
        if model_probs.is_empty() {
            return (0.5, 1.0);
        }
        let weights: Vec<f64> = model_probs
            .iter()
            .map(|(model, _, _)| {
                let w = match self.weights.iter().find(|mw| &mw.model == model) {
                    Some(w) => w,
                    None => return 1.0,
                };
                let mut weight = w.base_weight;
                weight *= (-w.range_decay * forecast_hour as f64 / 24.0).exp();
                for bonus in &w.regime_bonus {
                    if bonus.regime == regime {
                        weight *= bonus.multiplier;
                    }
                }
                weight.max(1e-9)
            })
            .collect();

        let total: f64 = weights.iter().sum();
        let normalized: Vec<f64> = weights.iter().map(|w| w / total).collect();

        let blended_prob: f64 = model_probs
            .iter()
            .zip(normalized.iter())
            .map(|((_, prob, _), w)| prob * w)
            .sum();

        let blended_uncertainty: f64 = model_probs
            .iter()
            .zip(normalized.iter())
            .map(|((_, _, unc), w)| (unc * w).powi(2))
            .sum::<f64>()
            .sqrt();

        (blended_prob, blended_uncertainty)
    }
}

impl Default for ModelBlender {
    fn default() -> Self {
        Self {
            weights: vec![
                ModelWeight {
                    model: NwpModel::Gfs,
                    base_weight: 0.4,
                    range_decay: 0.3,
                    regime_bonus: vec![],
                },
                ModelWeight {
                    model: NwpModel::Ecmwf,
                    base_weight: 0.6,
                    range_decay: 0.2,
                    regime_bonus: vec![],
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_single() {
        let b = ModelBlender::default();
        let (p, _) = b.blend(&[(NwpModel::Gfs, 0.7, 0.1)], 24, "normal");
        assert!((p - 0.7).abs() < 1e-9);
    }

    #[test]
    fn blend_weights_sum() {
        let b = ModelBlender::default();
        let (p, u) = b.blend(
            &[(NwpModel::Gfs, 0.6, 0.1), (NwpModel::Ecmwf, 0.8, 0.1)],
            24,
            "normal",
        );
        assert!(p > 0.6 && p < 0.8);
        assert!(u > 0.0);
    }
}
