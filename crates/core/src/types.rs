use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A Polymarket weather market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherMarket {
    pub condition_id: String,
    pub question: String,
    pub token_id_yes: String,
    pub token_id_no: String,
    pub station: Station,
    pub metric: WeatherMetric,
    pub event_time: DateTime<Utc>,
    pub resolution_time: DateTime<Utc>,
    pub resolved_outcome: Option<bool>,
}

/// Verification station
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    pub elevation_m: f64,
}

/// What we're measuring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeatherMetric {
    MaxTempF { threshold: f64 },
    MinTempF { threshold: f64 },
    PrecipitationIn { threshold: f64 },
    SnowfallIn { threshold: f64 },
    WindSpeedMph { threshold: f64 },
}

/// A signal from our engine
#[derive(Debug, Clone)]
pub struct Signal {
    pub id: Uuid,
    pub market: WeatherMarket,
    pub model_prob: f64,
    pub model_uncertainty: f64,
    pub market_price: f64,
    pub edge: f64,
    pub confidence: f64,
    pub size_multiplier: f64,
    pub model_run: ModelRunId,
    pub timestamp: DateTime<Utc>,
}

/// Identifies a specific NWP model run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRunId {
    pub model: NwpModel,
    pub init_time: DateTime<Utc>,
    pub forecast_hour: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NwpModel {
    Gfs,
    Ecmwf,
    Hrrr,
    Nam,
}

/// Order side
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

/// A trade we want to execute
#[derive(Debug, Clone)]
pub struct TradeIntent {
    pub signal_id: Uuid,
    pub market: WeatherMarket,
    pub side: Side,
    pub price: f64,
    pub size: f64,
    pub min_edge: f64,
}
