//! Position tracking (fills, exposure).

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct PositionTracker {
    positions: HashMap<String, TrackedPosition>,
}

#[derive(Debug, Clone)]
pub struct TrackedPosition {
    pub market_id: String,
    pub size: f64,
    pub avg_price: f64,
}

impl PositionTracker {
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
        }
    }

    pub fn record_fill(&mut self, market_id: String, size: f64, price: f64) {
        self.positions
            .entry(market_id.clone())
            .and_modify(|p| {
                let total = p.size + size;
                p.avg_price = (p.avg_price * p.size + price * size) / total.max(1e-9);
                p.size = total;
            })
            .or_insert(TrackedPosition {
                market_id,
                size,
                avg_price: price,
            });
    }
}
