//! Polymarket Gamma API discovery for daily-temperature bracket markets.
//!
//! Source: GET /events?tag_slug=daily-temperature&active=true&closed=false.
//! Each event holds ~11 bracket markets (Yes/No on a temperature range).
//! Only markets that are open AND end in the future become WeatherMarkets;
//! everything else is counted in DiscoveryStats and logged.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tracing::info;

use noaas_ark_core::types::{WeatherMarket, WeatherMetric};
use noaas_ark_nwp::station::StationRegistry;

use crate::parse::{parse_bracket_question, BracketKind};

pub const GAMMA_BASE: &str = "https://gamma-api.polymarket.com";
/// Tag that covers the recurring daily high/low temperature series.
/// ("weather" is too broad: earthquakes, measles, etc.)
pub const DISCOVERY_TAG: &str = "daily-temperature";

pub struct GammaClient {
    http: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Default)]
pub struct DiscoveryStats {
    pub events_seen: usize,
    pub markets_seen: usize,
    pub parsed: usize,
    pub skipped_closed: usize,
    pub skipped_ended: usize,
    pub skipped_unparseable: usize,
    pub skipped_unknown_city: usize,
    pub skipped_bad_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct GammaEvent {
    #[serde(default)]
    markets: Vec<GammaMarket>,
}

#[derive(Debug, Deserialize)]
struct GammaMarket {
    #[serde(default, rename = "conditionId")]
    condition_id: String,
    #[serde(default)]
    question: String,
    /// JSON-encoded string: "[\"Yes\", \"No\"]"
    #[serde(default)]
    outcomes: String,
    /// JSON-encoded string: "[\"<yes_token>\", \"<no_token>\"]"
    #[serde(default, rename = "clobTokenIds")]
    clob_token_ids: String,
    #[serde(default, rename = "endDate")]
    end_date: String,
    #[serde(default)]
    active: bool,
    #[serde(default)]
    closed: bool,
}

impl GammaClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: GAMMA_BASE.to_string(),
        }
    }

    pub async fn fetch_active_weather_markets(&self) -> Result<Vec<WeatherMarket>> {
        // Paginate: the tag holds ~250 events but the server caps pages at
        // ~100 rows, and stale events sort first, so a single page misses
        // the current ones. Stop on a short page; cap at 6 as a runaway guard.
        const PAGE_LIMIT: usize = 100;
        const MAX_PAGES: usize = 6;
        let mut all_events: Vec<GammaEvent> = Vec::new();
        for page in 0..MAX_PAGES {
            let url = format!(
                "{}/events?tag_slug={}&active=true&closed=false&limit={}&offset={}",
                self.base_url,
                DISCOVERY_TAG,
                PAGE_LIMIT,
                page * PAGE_LIMIT
            );
            let body = self
                .http
                .get(&url)
                .header("User-Agent", "NOAAsArk/0.1")
                .header("Accept-Encoding", "gzip, deflate")
                .send()
                .await
                .with_context(|| format!("gamma events request failed (page {page})"))?
                .text()
                .await
                .context("reading gamma events body")?;
            let events: Vec<GammaEvent> =
                serde_json::from_str(&body).context("events JSON decode")?;
            let n = events.len();
            all_events.extend(events);
            if n < PAGE_LIMIT {
                break;
            }
        }
        let (markets, stats) = markets_from_events(&all_events, Utc::now());
        info!(
            events = stats.events_seen,
            markets = stats.markets_seen,
            parsed = stats.parsed,
            skipped_closed = stats.skipped_closed,
            skipped_ended = stats.skipped_ended,
            skipped_unparseable = stats.skipped_unparseable,
            skipped_unknown_city = stats.skipped_unknown_city,
            skipped_bad_tokens = stats.skipped_bad_tokens,
            "Gamma discovery complete"
        );
        Ok(markets)
    }
}

impl Default for GammaClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Pure parse path (offline-testable): events JSON -> markets + stats.
pub fn markets_from_events_json(
    json: &str,
    now: DateTime<Utc>,
) -> Result<(Vec<WeatherMarket>, DiscoveryStats)> {
    let events: Vec<GammaEvent> = serde_json::from_str(json).context("events JSON decode")?;
    Ok(markets_from_events(&events, now))
}

/// Pure convert path over already-decoded events.
fn markets_from_events(
    events: &[GammaEvent],
    now: DateTime<Utc>,
) -> (Vec<WeatherMarket>, DiscoveryStats) {
    let mut stats = DiscoveryStats {
        events_seen: events.len(),
        ..Default::default()
    };
    let registry = StationRegistry::new();
    let mut out = Vec::new();

    for event in events {
        for m in &event.markets {
            stats.markets_seen += 1;
            if m.closed || !m.active {
                stats.skipped_closed += 1;
                continue;
            }
            let end_date = match m.end_date.parse::<DateTime<Utc>>() {
                Ok(d) => d,
                Err(_) => {
                    stats.skipped_unparseable += 1;
                    continue;
                }
            };
            if end_date <= now {
                stats.skipped_ended += 1;
                continue;
            }
            let parsed = match parse_bracket_question(&m.question) {
                Some(p) => p,
                None => {
                    stats.skipped_unparseable += 1;
                    continue;
                }
            };
            let station = match registry.lookup_by_city_name(&parsed.city) {
                Some(s) => s.clone(),
                None => {
                    stats.skipped_unknown_city += 1;
                    continue;
                }
            };
            let (yes_token, no_token) = match split_yes_no(&m.outcomes, &m.clob_token_ids) {
                Some(t) => t,
                None => {
                    stats.skipped_bad_tokens += 1;
                    continue;
                }
            };
            let metric = match parsed.kind {
                BracketKind::High => WeatherMetric::MaxTempRangeF {
                    low: parsed.low,
                    high: parsed.high,
                },
                BracketKind::Low => WeatherMetric::MinTempRangeF {
                    low: parsed.low,
                    high: parsed.high,
                },
            };
            out.push(WeatherMarket {
                condition_id: m.condition_id.clone(),
                question: m.question.clone(),
                token_id_yes: yes_token,
                token_id_no: no_token,
                station,
                metric,
                event_time: end_date,
                resolution_time: end_date,
                resolved_outcome: None,
            });
            stats.parsed += 1;
        }
    }
    (out, stats)
}

/// Map JSON-encoded outcomes + token arrays to (yes_token, no_token).
/// Requires a Yes/No outcome pair (in either order).
fn split_yes_no(outcomes_json: &str, tokens_json: &str) -> Option<(String, String)> {
    let outcomes: Vec<String> = serde_json::from_str(outcomes_json).ok()?;
    let tokens: Vec<String> = serde_json::from_str(tokens_json).ok()?;
    if outcomes.len() != 2 || tokens.len() != 2 {
        return None;
    }
    let yes_idx = outcomes
        .iter()
        .position(|o| o.eq_ignore_ascii_case("yes"))?;
    let no_idx = outcomes.iter().position(|o| o.eq_ignore_ascii_case("no"))?;
    Some((tokens[yes_idx].clone(), tokens[no_idx].clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_yes_no_order_independent() {
        let (yes, no) = split_yes_no(r#"["Yes", "No"]"#, r#"["111", "222"]"#).unwrap();
        assert_eq!((yes.as_str(), no.as_str()), ("111", "222"));
        let (yes, no) = split_yes_no(r#"["No", "Yes"]"#, r#"["111", "222"]"#).unwrap();
        assert_eq!((yes.as_str(), no.as_str()), ("222", "111"));
        assert!(split_yes_no(r#"["Up", "Down"]"#, r#"["111", "222"]"#).is_none());
        assert!(split_yes_no(r#"["Yes"]"#, r#"["111"]"#).is_none());
    }
}
