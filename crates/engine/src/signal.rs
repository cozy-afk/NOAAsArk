use chrono::Utc;
use noaas_ark_core::types::{ModelRunId, NwpModel, Side, Signal, TradeIntent, WeatherMarket};
use noaas_ark_market::clob::OrderbookSnapshot;
use noaas_ark_nwp::ensemble::EnsembleForecaster;
use uuid::Uuid;

pub struct SignalEngine {
    forecaster: EnsembleForecaster,
    config: EngineConfig,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub min_edge: f64,
    pub min_edge_after_spread: f64,
    pub max_position_usdc: f64,
    pub kelly_fraction: f64,
    pub min_hours_to_resolution: f64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            min_edge: 0.05,
            min_edge_after_spread: 0.03,
            max_position_usdc: 25.0,
            kelly_fraction: 0.25,
            min_hours_to_resolution: 3.0,
        }
    }
}

impl From<&noaas_ark_core::config::EngineConfig> for EngineConfig {
    fn from(c: &noaas_ark_core::config::EngineConfig) -> Self {
        Self {
            min_edge: c.min_edge,
            min_edge_after_spread: c.min_edge_after_spread,
            max_position_usdc: c.max_position_usdc,
            kelly_fraction: c.kelly_fraction,
            min_hours_to_resolution: c.min_hours_to_resolution,
        }
    }
}

impl SignalEngine {
    pub fn new(config: EngineConfig, forecaster: EnsembleForecaster) -> Self {
        Self { forecaster, config }
    }

    pub fn with_defaults() -> Self {
        Self::new(
            EngineConfig::default(),
            EnsembleForecaster::with_default_bias(),
        )
    }

    pub fn evaluate(
        &self,
        market: &WeatherMarket,
        ensemble_values: &[f64],
        orderbook: &OrderbookSnapshot,
        model_run: ModelRunId,
    ) -> Option<Signal> {
        let hours_to_resolution = (market.resolution_time - Utc::now()).num_minutes() as f64 / 60.0;
        if hours_to_resolution < self.config.min_hours_to_resolution {
            return None;
        }

        let (model_prob, model_uncertainty) = self
            .forecaster
            .probability_with_uncertainty(ensemble_values, &market.metric);

        let dist = self
            .forecaster
            .fit_distribution(ensemble_values, &market.metric);
        let threshold = match &market.metric {
            noaas_ark_core::types::WeatherMetric::MaxTempF { threshold } => *threshold,
            noaas_ark_core::types::WeatherMetric::MinTempF { threshold } => *threshold,
            noaas_ark_core::types::WeatherMetric::PrecipitationIn { threshold } => *threshold,
            noaas_ark_core::types::WeatherMetric::SnowfallIn { threshold } => *threshold,
            noaas_ark_core::types::WeatherMetric::WindSpeedMph { threshold } => *threshold,
            // Regime gating for brackets uses the bracket midpoint (or the single
            // bounded edge for open-ended brackets). Approximate by design: regime
            // only scales min_edge, the probability itself uses the full interval.
            noaas_ark_core::types::WeatherMetric::MaxTempRangeF { low, high }
            | noaas_ark_core::types::WeatherMetric::MinTempRangeF { low, high } => {
                match (low, high) {
                    (Some(l), Some(h)) => (l + h) / 2.0,
                    (Some(l), None) => *l,
                    (None, Some(h)) => *h,
                    (None, None) => return None,
                }
            }
        };
        let regime = crate::regime::ThresholdRegime::classify(&dist, threshold);
        let min_edge_required = regime.min_edge_required().max(self.config.min_edge);

        let best_ask = orderbook.best_ask()?;
        let best_bid = orderbook.best_bid()?;

        let edge_yes = model_prob - best_ask;
        let edge_no = (1.0 - model_prob) - (1.0 - best_bid);

        let (edge, price) = if edge_yes >= edge_no && edge_yes > 0.0 {
            (edge_yes, best_ask)
        } else if edge_no > 0.0 {
            (edge_no, 1.0 - best_bid)
        } else {
            return None;
        };

        if edge < min_edge_required {
            return None;
        }

        let spread = (best_ask - best_bid).max(0.0);
        if edge - spread / 2.0 < self.config.min_edge_after_spread {
            return None;
        }

        Some(Signal {
            id: Uuid::new_v4(),
            market: market.clone(),
            model_prob,
            model_uncertainty,
            market_price: price,
            edge,
            confidence: (1.0 - model_uncertainty).clamp(0.0, 1.0),
            size_multiplier: regime.size_multiplier(),
            model_run,
            timestamp: Utc::now(),
        })
    }

    pub fn size_trade(&self, signal: &Signal) -> TradeIntent {
        let price = signal.market_price.clamp(0.01, 0.99);
        let kelly = signal.edge / price;
        let fraction = (kelly * self.config.kelly_fraction).clamp(0.0, 1.0);
        let size = (fraction * self.config.max_position_usdc).min(self.config.max_position_usdc)
            * signal.size_multiplier;

        TradeIntent {
            signal_id: signal.id,
            market: signal.market.clone(),
            side: Side::Buy,
            price: signal.market_price,
            size: size.max(1.0),
            min_edge: self.config.min_edge,
        }
    }
}

pub fn fallback_model_run() -> ModelRunId {
    ModelRunId {
        model: NwpModel::Gfs,
        init_time: Utc::now(),
        forecast_hour: 0,
    }
}
