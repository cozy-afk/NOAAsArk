//! Real-time orderbook updates via Polymarket CLOB WebSocket.
//! wss://ws-subscriptions-clob.polymarket.com/ws/market

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tracing::info;

pub const WS_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";

/// Live market feed handle (v0.1: connect + subscribe, full state in market::orderbook).
pub struct MarketFeed {
    pub token_ids: Vec<String>,
}

impl MarketFeed {
    pub async fn connect(token_ids: Vec<String>) -> anyhow::Result<Self> {
        let (ws_stream, _) = connect_async(WS_URL).await?;
        let (mut write, mut read) = ws_stream.split();

        let subscribe = serde_json::json!({
            "type": "subscribe",
            "assets_ids": token_ids,
        });
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                subscribe.to_string(),
            ))
            .await?;

        info!("Connected to Polymarket WebSocket");

        // Drain one message to confirm subscription, then hand off.
        // Full read loop lives with the caller in v0.2.
        tokio::spawn(async move {
            if let Some(msg) = read.next().await {
                match msg {
                    Ok(m) => {
                        info!(?m, "WS msg");
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "WS read error");
                    }
                }
            }
        });

        Ok(Self { token_ids })
    }
}
