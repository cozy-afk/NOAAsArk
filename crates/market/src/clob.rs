use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::types::Side;

const CLOB_BASE: &str = "https://clob.polymarket.com";

pub struct ClobClient {
    http: reqwest::Client,
    auth: super::auth::ClobAuth,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderbookSnapshot {
    pub market: String,
    pub token_id: String,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: f64,
    pub size: f64,
}

impl OrderbookSnapshot {
    /// Best bid = max over bids. Asks arrive descending, so never index [0].
    pub fn best_bid(&self) -> Option<f64> {
        self.bids
            .iter()
            .map(|l| l.price)
            .fold(None, |a, b| Some(a.map_or(b, |x: f64| x.max(b))))
    }

    /// Best ask = min over asks.
    pub fn best_ask(&self) -> Option<f64> {
        self.asks
            .iter()
            .map(|l| l.price)
            .fold(None, |a, b| Some(a.map_or(b, |x: f64| x.min(b))))
    }

    pub fn midpoint(&self) -> Option<f64> {
        match (self.best_bid(), self.best_ask()) {
            (Some(b), Some(a)) => Some((b + a) / 2.0),
            _ => None,
        }
    }

    /// Average fill price for a given size, or None if not enough liquidity.
    pub fn estimate_fill_price(&self, side: Side, size: f64) -> Option<f64> {
        let levels = match side {
            Side::Buy => &self.asks,
            Side::Sell => &self.bids,
        };
        // Walk from best outward: asks ascending, bids descending.
        let mut ordered: Vec<&PriceLevel> = levels.iter().collect();
        match side {
            Side::Buy => ordered.sort_by(|a, b| a.price.partial_cmp(&b.price).unwrap()),
            Side::Sell => ordered.sort_by(|a, b| b.price.partial_cmp(&a.price).unwrap()),
        }
        let mut remaining = size;
        let mut total_cost = 0.0;
        for level in ordered {
            let fill = remaining.min(level.size);
            total_cost += fill * level.price;
            remaining -= fill;
            if remaining <= 0.0 {
                break;
            }
        }
        if remaining > 0.0 {
            return None;
        }
        Some(total_cost / size)
    }
}

#[derive(Debug, Serialize)]
pub struct OrderRequest {
    pub token_id: String,
    pub side: String,
    pub price: f64,
    pub size: f64,
    pub order_type: String,
}

#[derive(Debug, Deserialize)]
pub struct OrderResponse {
    pub order_id: String,
    pub status: String,
    #[serde(default)]
    pub filled_size: f64,
    #[serde(default)]
    pub avg_price: f64,
}

impl ClobClient {
    pub fn new(auth: super::auth::ClobAuth) -> Self {
        Self {
            http: reqwest::Client::new(),
            auth,
        }
    }

    pub async fn get_orderbook(&self, token_id: &str) -> Result<OrderbookSnapshot> {
        let url = format!("{}/book?token_id={}", CLOB_BASE, token_id);
        let resp = self
            .http
            .get(&url)
            .header("User-Agent", "NOAAsArk/0.1")
            .header("Accept-Encoding", "gzip, deflate")
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }

    pub async fn place_order(&self, order: &OrderRequest) -> Result<OrderResponse> {
        let url = format!("{}/order", CLOB_BASE);
        let signature = self.auth.sign_order(order)?;
        let resp = self
            .http
            .post(&url)
            .header("User-Agent", "NOAAsArk/0.1")
            .header("Authorization", &self.auth.api_key)
            .header("X-Signature", &signature)
            .json(order)
            .send()
            .await?
            .json()
            .await?;
        let out: OrderResponse = resp;
        info!(order_id = %out.order_id, status = %out.status, "Order placed");
        Ok(out)
    }

    pub async fn cancel_order(&self, order_id: &str) -> Result<()> {
        let url = format!("{}/order/{}", CLOB_BASE, order_id);
        self.http
            .delete(&url)
            .header("User-Agent", "NOAAsArk/0.1")
            .header("Authorization", &self.auth.api_key)
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_open_orders(&self) -> Result<Vec<OrderResponse>> {
        let url = format!("{}/orders?status=OPEN", CLOB_BASE);
        let resp = self
            .http
            .get(&url)
            .header("User-Agent", "NOAAsArk/0.1")
            .header("Authorization", &self.auth.api_key)
            .send()
            .await?
            .json()
            .await?;
        Ok(resp)
    }
}
