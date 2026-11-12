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

// ---- Conditional calibration (Problem 8) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Season {
    Winter,
    Spring,
    Summer,
    Fall,
}

impl Season {
    pub fn from_month(m: u32) -> Self {
        match m {
            12 | 1 | 2 => Season::Winter,
            3..=5 => Season::Spring,
            6..=8 => Season::Summer,
            _ => Season::Fall,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinConditions {
    pub range_hours: (u32, u32),
    pub season: Season,
    pub regime: crate::regime::ThresholdRegime,
    pub region: String,
}

#[derive(Debug, Clone)]
pub struct CalibrationBin {
    pub conditions: BinConditions,
    pub calibration: IsotonicCalibration,
    pub n_samples: usize,
}

pub struct ConditionalCalibrator {
    pub bins: Vec<CalibrationBin>,
    pub global_calibration: IsotonicCalibration,
}

impl ConditionalCalibrator {
    pub fn new() -> Self {
        Self {
            bins: Vec::new(),
            global_calibration: IsotonicCalibration::identity(),
        }
    }

    pub fn find_bin(&self, conditions: &BinConditions) -> Option<&CalibrationBin> {
        self.bins.iter().find(|b| {
            b.conditions.range_hours == conditions.range_hours
                && b.conditions.season == conditions.season
                && b.conditions.regime == conditions.regime
                && b.conditions.region == conditions.region
        })
    }

    pub fn calibrate(&self, raw_prob: f64, conditions: &BinConditions) -> f64 {
        if let Some(bin) = self.find_bin(conditions) {
            if bin.n_samples > 50 {
                return bin.calibration.apply(raw_prob);
            }
        }
        self.global_calibration.apply(raw_prob)
    }
}

impl Default for ConditionalCalibrator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct IsotonicCalibration {
    pub mapping: Vec<(f64, f64)>,
}

impl IsotonicCalibration {
    pub fn identity() -> Self {
        Self {
            mapping: vec![(0.0, 0.0), (1.0, 1.0)],
        }
    }

    pub fn apply(&self, raw_prob: f64) -> f64 {
        if self.mapping.is_empty() {
            return raw_prob;
        }
        if raw_prob <= self.mapping[0].0 {
            return self.mapping[0].1;
        }
        if raw_prob >= self.mapping[self.mapping.len() - 1].0 {
            return self.mapping[self.mapping.len() - 1].1;
        }
        for w in self.mapping.windows(2) {
            let (x0, y0) = w[0];
            let (x1, y1) = w[1];
            if raw_prob >= x0 && raw_prob <= x1 {
                let t = if (x1 - x0).abs() < 1e-12 {
                    0.0
                } else {
                    (raw_prob - x0) / (x1 - x0)
                };
                return y0 + t * (y1 - y0);
            }
        }
        raw_prob
    }

    pub fn fit(predictions: &[f64], outcomes: &[bool]) -> Self {
        if predictions.is_empty() || predictions.len() != outcomes.len() {
            return Self::identity();
        }
        let mut pairs: Vec<(f64, f64)> = predictions
            .iter()
            .zip(outcomes.iter())
            .map(|(&p, &o)| (p, if o { 1.0 } else { 0.0 }))
            .collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        #[derive(Clone)]
        struct Block {
            sum_y: f64,
            n: usize,
            min_x: f64,
            max_x: f64,
        }
        let mut blocks: Vec<Block> = Vec::new();
        for (x, y) in &pairs {
            let mut cur = Block {
                sum_y: *y,
                n: 1,
                min_x: *x,
                max_x: *x,
            };
            while let Some(last) = blocks.last() {
                let last_avg = last.sum_y / last.n as f64;
                let cur_avg = cur.sum_y / cur.n as f64;
                if last_avg <= cur_avg + 1e-12 {
                    break;
                }
                let popped = blocks.pop().unwrap();
                cur = Block {
                    sum_y: popped.sum_y + cur.sum_y,
                    n: popped.n + cur.n,
                    min_x: popped.min_x,
                    max_x: cur.max_x,
                };
            }
            blocks.push(cur);
        }

        let mut mapping: Vec<(f64, f64)> = blocks
            .iter()
            .map(|b| ((b.min_x + b.max_x) / 2.0, b.sum_y / b.n as f64))
            .collect();
        if !mapping.is_empty() {
            mapping.insert(0, (0.0, mapping[0].1));
            mapping.push((1.0, mapping[mapping.len() - 1].1));
        } else {
            mapping = vec![(0.0, 0.0), (1.0, 1.0)];
        }
        Self { mapping }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_passthrough() {
        let c = IsotonicCalibration::identity();
        assert!((c.apply(0.3) - 0.3).abs() < 1e-9);
    }

    #[test]
    fn fit_monotonic() {
        let preds = vec![0.1, 0.2, 0.3, 0.8, 0.9];
        let outs = vec![false, false, true, true, true];
        let c = IsotonicCalibration::fit(&preds, &outs);
        assert!(c.apply(0.8) >= c.apply(0.3));
    }

    #[test]
    fn conditional_fallback() {
        let cc = ConditionalCalibrator::new();
        let cond = BinConditions {
            range_hours: (0, 24),
            season: Season::Summer,
            regime: crate::regime::ThresholdRegime::NearThreshold { z_score: 0.0 },
            region: "NE".into(),
        };
        assert!((cc.calibrate(0.4, &cond) - 0.4).abs() < 1e-9);
    }
}
