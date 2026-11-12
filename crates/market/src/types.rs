use serde::{Deserialize, Serialize};

/// Local order side (mirror of core::types::Side for market-internal use).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    pub fn as_str(&self) -> &'static str {
        match self {
            Side::Buy => "BUY",
            Side::Sell => "SELL",
        }
    }
}

impl From<noaas_ark_core::types::Side> for Side {
    fn from(s: noaas_ark_core::types::Side) -> Self {
        match s {
            noaas_ark_core::types::Side::Buy => Side::Buy,
            noaas_ark_core::types::Side::Sell => Side::Sell,
        }
    }
}

impl From<Side> for noaas_ark_core::types::Side {
    fn from(s: Side) -> Self {
        match s {
            Side::Buy => noaas_ark_core::types::Side::Buy,
            Side::Sell => noaas_ark_core::types::Side::Sell,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketStatus {
    Open,
    Closed,
    Resolved { outcome_yes: bool },
}
