use std::collections::HashMap;

/// Local orderbook state built from WS deltas / REST snapshots.
#[derive(Debug, Default)]
pub struct LocalOrderbook {
    pub token_id: String,
    pub bids: HashMap<String, f64>,
    pub asks: HashMap<String, f64>,
}

impl LocalOrderbook {
    pub fn new(token_id: String) -> Self {
        Self {
            token_id,
            bids: HashMap::new(),
            asks: HashMap::new(),
        }
    }

    pub fn apply_snapshot(&mut self, bids: &[(f64, f64)], asks: &[(f64, f64)]) {
        self.bids.clear();
        self.asks.clear();
        for (p, s) in bids {
            self.bids.insert(price_key(*p), *s);
        }
        for (p, s) in asks {
            self.asks.insert(price_key(*p), *s);
        }
    }

    pub fn best_bid(&self) -> Option<f64> {
        self.bids
            .iter()
            .filter(|(_, s)| **s > 0.0)
            .map(|(p, _)| p.parse::<f64>().unwrap_or(0.0))
            .fold(None, |a: Option<f64>, b| Some(a.map_or(b, |x| x.max(b))))
    }

    pub fn best_ask(&self) -> Option<f64> {
        self.asks
            .iter()
            .filter(|(_, s)| **s > 0.0)
            .map(|(p, _)| p.parse::<f64>().unwrap_or(1.0))
            .fold(None, |a: Option<f64>, b| Some(a.map_or(b, |x| x.min(b))))
    }
}

fn price_key(p: f64) -> String {
    format!("{:.4}", p)
}
