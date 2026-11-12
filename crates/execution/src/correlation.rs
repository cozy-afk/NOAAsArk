// crates/execution/src/correlation.rs
//
// Correlation-aware risk: positions in the same region are highly correlated,
// so effective exposure must account for grouping.

use std::collections::HashMap;

pub struct CorrelationManager {
    pub market_regions: HashMap<String, String>,
    pub max_regional_exposure: f64,
    pub max_correlated_exposure: f64,
}

impl CorrelationManager {
    pub fn new(max_regional_exposure: f64, max_correlated_exposure: f64) -> Self {
        Self {
            market_regions: HashMap::new(),
            max_regional_exposure,
            max_correlated_exposure,
        }
    }

    pub fn register_region(&mut self, market_id: String, region: String) {
        self.market_regions.insert(market_id, region);
    }

    /// Effective exposure accounting for correlation: sqrt(sum regional_exp^2).
    /// Assumes perfect correlation within region, zero across regions (conservative).
    pub fn effective_exposure(&self, positions: &[crate::risk::Position]) -> f64 {
        let mut regional: HashMap<String, f64> = HashMap::new();
        for pos in positions {
            let region = self
                .market_regions
                .get(&pos.market_id)
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            *regional.entry(region).or_default() += pos.size * pos.entry_price;
        }
        regional.values().map(|e| e * e).sum::<f64>().sqrt()
    }

    pub fn can_add(
        &self,
        positions: &[crate::risk::Position],
        new_market: &str,
        new_exposure: f64,
    ) -> bool {
        let mut test = positions.to_vec();
        test.push(crate::risk::Position {
            market_id: new_market.to_string(),
            side: crate::Side::Buy,
            entry_price: 1.0,
            size: new_exposure,
            entry_time: chrono::Utc::now(),
        });
        let eff = self.effective_exposure(&test);
        eff <= self.max_correlated_exposure
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::Position;

    #[test]
    fn effective_exposure_same_region() {
        let mut cm = CorrelationManager::new(100.0, 100.0);
        cm.register_region("m1".into(), "NE".into());
        cm.register_region("m2".into(), "NE".into());
        let positions = vec![
            Position {
                market_id: "m1".into(),
                side: crate::Side::Buy,
                entry_price: 1.0,
                size: 30.0,
                entry_time: chrono::Utc::now(),
            },
            Position {
                market_id: "m2".into(),
                side: crate::Side::Buy,
                entry_price: 1.0,
                size: 40.0,
                entry_time: chrono::Utc::now(),
            },
        ];
        let eff = cm.effective_exposure(&positions);
        // same region: sqrt(70^2) = 70
        assert!((eff - 70.0).abs() < 1e-9);
    }

    #[test]
    fn effective_exposure_diff_region() {
        let mut cm = CorrelationManager::new(100.0, 100.0);
        cm.register_region("m1".into(), "NE".into());
        cm.register_region("m2".into(), "SW".into());
        let positions = vec![
            Position {
                market_id: "m1".into(),
                side: crate::Side::Buy,
                entry_price: 1.0,
                size: 30.0,
                entry_time: chrono::Utc::now(),
            },
            Position {
                market_id: "m2".into(),
                side: crate::Side::Buy,
                entry_price: 1.0,
                size: 40.0,
                entry_time: chrono::Utc::now(),
            },
        ];
        let eff = cm.effective_exposure(&positions);
        // diff region: sqrt(30^2+40^2)=50
        assert!((eff - 50.0).abs() < 1e-9);
    }
}
