// crates/engine/src/extreme.rs

pub struct ExtremeEventDetector {
    pub model_disagreement_threshold: f64,
    pub spread_percentile_threshold: f64,
}

impl ExtremeEventDetector {
    pub fn new(model_disagreement_threshold: f64, spread_percentile_threshold: f64) -> Self {
        Self {
            model_disagreement_threshold,
            spread_percentile_threshold,
        }
    }

    pub fn is_extreme_situation(
        &self,
        gfs_prob: f64,
        ecmwf_prob: f64,
        ensemble_spread: f64,
        historical_spread_distribution: &[f64],
    ) -> bool {
        let disagreement = (gfs_prob - ecmwf_prob).abs();
        let spread_percentile = if historical_spread_distribution.is_empty() {
            0.0
        } else {
            historical_spread_distribution
                .iter()
                .filter(|&&s| s < ensemble_spread)
                .count() as f64
                / historical_spread_distribution.len() as f64
        };
        disagreement > self.model_disagreement_threshold
            || spread_percentile > self.spread_percentile_threshold
    }
}

impl Default for ExtremeEventDetector {
    fn default() -> Self {
        Self {
            model_disagreement_threshold: 0.20,
            spread_percentile_threshold: 0.90,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_disagreement() {
        let d = ExtremeEventDetector::default();
        assert!(d.is_extreme_situation(0.8, 0.5, 1.0, &[0.5, 0.6, 0.7]));
    }

    #[test]
    fn normal_not_extreme() {
        let d = ExtremeEventDetector::default();
        assert!(!d.is_extreme_situation(0.6, 0.62, 0.5, &[0.4, 0.5, 0.6]));
    }
}
