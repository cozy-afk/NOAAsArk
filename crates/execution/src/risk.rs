use chrono::{DateTime, Utc};
use noaas_ark_core::types::Side;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RiskConfig {
    pub max_total_exposure: f64,
    pub max_per_market: f64,
    pub max_daily_loss: f64,
    pub max_concurrent_positions: usize,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            max_total_exposure: 200.0,
            max_per_market: 50.0,
            max_daily_loss: 75.0,
            max_concurrent_positions: 8,
        }
    }
}

impl From<&noaas_ark_core::config::RiskConfig> for RiskConfig {
    fn from(c: &noaas_ark_core::config::RiskConfig) -> Self {
        Self {
            max_total_exposure: c.max_total_exposure,
            max_per_market: c.max_per_market,
            max_daily_loss: c.max_daily_loss,
            max_concurrent_positions: c.max_concurrent_positions,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Position {
    pub market_id: String,
    pub side: Side,
    pub entry_price: f64,
    pub size: f64,
    pub entry_time: DateTime<Utc>,
}

pub struct RiskManager {
    config: RiskConfig,
    positions: HashMap<String, Position>,
    daily_pnl: f64,
    daily_reset: DateTime<Utc>,
}

impl RiskManager {
    pub fn new(config: RiskConfig) -> Self {
        Self {
            config,
            positions: HashMap::new(),
            daily_pnl: 0.0,
            daily_reset: Utc::now(),
        }
    }

    pub fn can_trade(&self, intent: &noaas_ark_core::types::TradeIntent) -> Result<(), RiskViolation> {
        if self.daily_pnl <= -self.config.max_daily_loss {
            return Err(RiskViolation::DailyLossLimit);
        }
        let total_exposure: f64 = self
            .positions
            .values()
            .map(|p| p.size * p.entry_price)
            .sum();
        if total_exposure + intent.size > self.config.max_total_exposure {
            return Err(RiskViolation::MaxExposure);
        }
        if let Some(existing) = self.positions.get(&intent.market.condition_id) {
            let existing_exposure = existing.size * existing.entry_price;
            if existing_exposure + intent.size > self.config.max_per_market {
                return Err(RiskViolation::PerMarketLimit);
            }
        }
        if self.positions.len() >= self.config.max_concurrent_positions
            && !self.positions.contains_key(&intent.market.condition_id)
        {
            return Err(RiskViolation::TooManyPositions);
        }
        Ok(())
    }

    pub fn add_position(
        &mut self,
        intent: &noaas_ark_core::types::TradeIntent,
        fill_price: f64,
        filled_size: f64,
    ) {
        let pos = Position {
            market_id: intent.market.condition_id.clone(),
            side: intent.side,
            entry_price: fill_price,
            size: filled_size,
            entry_time: Utc::now(),
        };
        self.positions.insert(intent.market.condition_id.clone(), pos);
    }

    pub fn record_pnl(&mut self, pnl: f64) {
        let now = Utc::now();
        if (now - self.daily_reset).num_hours() >= 24 {
            self.daily_pnl = 0.0;
            self.daily_reset = now;
        }
        self.daily_pnl += pnl;
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RiskViolation {
    #[error("Daily loss limit reached")]
    DailyLossLimit,
    #[error("Maximum total exposure exceeded")]
    MaxExposure,
    #[error("Per-market exposure limit exceeded")]
    PerMarketLimit,
    #[error("Too many concurrent positions")]
    TooManyPositions,
}
