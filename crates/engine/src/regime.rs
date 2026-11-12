// crates/engine/src/regime.rs

use noaas_ark_nwp::EnsembleDistribution;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThresholdRegime {
    FarAbove { z_score: f64 },
    NearThreshold { z_score: f64 },
    FarBelow { z_score: f64 },
}

impl ThresholdRegime {
    pub fn classify(dist: &EnsembleDistribution, threshold: f64) -> Self {
        let z = dist.z_score(threshold);
        if z > 2.0 {
            Self::FarAbove { z_score: z }
        } else if z < -2.0 {
            Self::FarBelow { z_score: z }
        } else {
            Self::NearThreshold { z_score: z }
        }
    }

    pub fn min_edge_required(&self) -> f64 {
        match self {
            Self::FarAbove { .. } => 0.08,
            Self::NearThreshold { .. } => 0.04,
            Self::FarBelow { .. } => 0.08,
        }
    }

    pub fn size_multiplier(&self) -> f64 {
        match self {
            Self::FarAbove { .. } => 0.5,
            Self::NearThreshold { .. } => 1.0,
            Self::FarBelow { .. } => 0.5,
        }
    }

    pub fn z_score(&self) -> f64 {
        match self {
            Self::FarAbove { z_score }
            | Self::NearThreshold { z_score }
            | Self::FarBelow { z_score } => *z_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use noaas_ark_nwp::EnsembleDistribution;

    #[test]
    fn classify_far_above() {
        let vals: Vec<f64> = (0..10).map(|_| 95.0).collect();
        let d = EnsembleDistribution::fit(&vals, 1.0);
        let r = ThresholdRegime::classify(&d, 90.0);
        assert!(matches!(r, ThresholdRegime::FarAbove { .. }));
    }
}
