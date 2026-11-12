//! Orderbook replay from stored snapshots (data/orderbooks/).

use anyhow::Result;
use noaas_ark_market::clob::OrderbookSnapshot;

pub fn load_snapshots(path: &str) -> Result<Vec<OrderbookSnapshot>> {
    let content = std::fs::read_to_string(path)?;
    let snaps: Vec<OrderbookSnapshot> = serde_json::from_str(&content)?;
    Ok(snaps)
}
