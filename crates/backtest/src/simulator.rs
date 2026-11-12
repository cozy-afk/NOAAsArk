//! Historical simulator: feed stored orderbooks + model probs, apply fees.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub n_trades: usize,
    pub wins: usize,
    pub total_pnl: f64,
    pub win_rate: f64,
}

impl BacktestResult {
    pub fn empty() -> Self {
        Self {
            n_trades: 0,
            wins: 0,
            total_pnl: 0.0,
            win_rate: 0.0,
        }
    }
}

/// Polymarket taker fee model: 0.07 * price * (1 - price) per share.
pub fn taker_fee_per_share(price: f64) -> f64 {
    0.07 * price * (1.0 - price)
}
