use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::types::Side;

const CLOB_BASE: &str = "https://clob.polymarket.com";

pub struct ClobClient {
    http: reqwest::Client,
    auth: super::auth::ClobAuth,
}

#[derive(Debug, Deserialize)]
struct RawPriceLevel {
    #[serde(deserialize_with = "de_f64_from_str_or_num")]
    price: f64,
    #[serde(deserialize_with = "de_f64_from_str_or_num")]
    size: f64,
}

#[derive(Debug, Deserialize)]
struct RawBook {
    #[serde(default)]
    market: String,
    #[serde(default, rename = "asset_id")]
    asset_id: String,
    #[serde(default)]
    bids: Vec<RawPriceLevel>,
    #[serde(default)]
    asks: Vec<RawPriceLevel>,
    #[serde(default, deserialize_with = "de_timestamp_ms_or_rfc3339")]
    timestamp: chrono::DateTime<chrono::Utc>,
}

fn de_f64_from_str_or_num<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = f64;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a number or numeric string")
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<f64, E> {
            v.parse().map_err(de::Error::custom)
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<f64, E> {
            Ok(v)
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<f64, E> {
            Ok(v as f64)
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<f64, E> {
            Ok(v as f64)
        }
    }
    deserializer.deserialize_any(V)
}

fn de_timestamp_ms_or_rfc3339<'de, D>(
    deserializer: D,
) -> Result<chrono::DateTime<chrono::Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};
    struct V;
    impl<'de> Visitor<'de> for V {
        type Value = chrono::DateTime<chrono::Utc>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "a timestamp (ms epoch string/number or RFC3339)")
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            // Try ms epoch first.
            if let Ok(ms) = v.parse::<i64>() {
                if ms > 1_000_000_000_000 {
                    let secs = ms / 1000;
                    let nanos = ((ms % 1000) * 1_000_000) as u32;
                    return chrono::DateTime::from_timestamp(secs, nanos)
                        .map(Ok)
                        .unwrap_or_else(|| Err(de::Error::custom("bad epoch")));
                }
            }
            v.parse().map_err(de::Error::custom)
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            let secs = v / 1000;
            let nanos = ((v % 1000) * 1_000_000) as u32;
            chrono::DateTime::from_timestamp(secs, nanos)
                .ok_or_else(|| de::Error::custom("bad epoch"))
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
            let ms = v as i64;
            let secs = ms / 1000;
            let nanos = ((ms % 1000) * 1_000_000) as u32;
            chrono::DateTime::from_timestamp(secs, nanos)
                .ok_or_else(|| de::Error::custom("bad epoch"))
        }
    }
    deserializer.deserialize_any(V)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Parse a CLOB `/book` JSON payload into an OrderbookSnapshot.
/// Public so offline tests can exercise the deserialization path.
pub fn parse_book(json: &str, token_id: &str) -> Result<OrderbookSnapshot> {
    let raw: RawBook =
        serde_json::from_str(json).map_err(|e| anyhow::anyhow!("parsing orderbook JSON: {e}"))?;
    let token = if raw.asset_id.is_empty() {
        token_id.to_string()
    } else {
        raw.asset_id
    };
    let ts = if raw.timestamp.timestamp() == 0 {
        chrono::Utc::now()
    } else {
        raw.timestamp
    };
    Ok(OrderbookSnapshot {
        market: raw.market,
        token_id: token,
        bids: raw
            .bids
            .into_iter()
            .map(|l| PriceLevel {
                price: l.price,
                size: l.size,
            })
            .collect(),
        asks: raw
            .asks
            .into_iter()
            .map(|l| PriceLevel {
                price: l.price,
                size: l.size,
            })
            .collect(),
        timestamp: ts,
    })
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
        // Retry with backoff on 429/5xx; per M2.3 never abort the whole cycle.
        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 0..4 {
            if attempt > 0 {
                let backoff = std::time::Duration::from_millis(250 * (1 << (attempt - 1)));
                tokio::time::sleep(backoff).await;
            }
            let resp = self
                .http
                .get(&url)
                .header("User-Agent", "NOAAsArk/0.1")
                .header("Accept-Encoding", "gzip, deflate")
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await;
            match resp {
                Ok(r) => {
                    let status = r.status();
                    if status.is_success() {
                        let raw_text = r.text().await.unwrap_or_default();
                        let raw: RawBook = serde_json::from_str(&raw_text)
                            .map_err(|e| anyhow::anyhow!("parsing orderbook JSON: {e}"))?;
                        let token = if raw.asset_id.is_empty() {
                            token_id.to_string()
                        } else {
                            raw.asset_id
                        };
                        let ts = if raw.timestamp.timestamp() == 0 {
                            chrono::Utc::now()
                        } else {
                            raw.timestamp
                        };
                        return Ok(OrderbookSnapshot {
                            market: raw.market,
                            token_id: token,
                            bids: raw
                                .bids
                                .into_iter()
                                .map(|l| PriceLevel {
                                    price: l.price,
                                    size: l.size,
                                })
                                .collect(),
                            asks: raw
                                .asks
                                .into_iter()
                                .map(|l| PriceLevel {
                                    price: l.price,
                                    size: l.size,
                                })
                                .collect(),
                            timestamp: ts,
                        });
                    }
                    if status.as_u16() == 429 || status.is_server_error() {
                        last_err = Some(anyhow::anyhow!("orderbook HTTP {}", status));
                        continue;
                    }
                    // Other 4xx: don't retry.
                    let text = r.text().await.unwrap_or_default();
                    return Err(anyhow::anyhow!("orderbook HTTP {}: {}", status, text));
                }
                Err(e) => {
                    last_err = Some(e.into());
                    continue;
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("orderbook fetch failed")))
    }

    pub async fn place_order(&self, order: &OrderRequest) -> Result<OrderResponse> {
        let url = format!("{}/order", CLOB_BASE);
        let body = serde_json::to_string(order)?;
        let ts = chrono::Utc::now().timestamp();
        let signature = self.auth.sign_l2("POST", "/order", &body, ts)?;
        let resp = self
            .http
            .post(&url)
            .header("User-Agent", "NOAAsArk/0.1")
            .header("POLY_ADDRESS", self.auth.wallet_address())
            .header("POLY_API_KEY", &self.auth.api_key)
            .header("POLY_PASSPHRASE", &self.auth.api_passphrase)
            .header("POLY_SIGNATURE", &signature)
            .header("POLY_TIMESTAMP", ts.to_string())
            .header("Content-Type", "application/json")
            .body(body)
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
