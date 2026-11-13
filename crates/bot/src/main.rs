use anyhow::Result;
use chrono::{Timelike, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use noaas_ark_core::config::BotConfig;
use noaas_ark_core::types::{ModelRunId, NwpModel, WeatherMarket};
use noaas_ark_engine::signal::{EngineConfig, SignalEngine};
use noaas_ark_execution::risk::{RiskConfig, RiskManager};
use noaas_ark_market::auth::ClobAuth;
use noaas_ark_market::clob::{ClobClient, OrderRequest};
use noaas_ark_market::gamma::GammaClient;
use noaas_ark_nwp::ensemble::{BiasCorrection, EnsembleForecaster};
use noaas_ark_nwp::gfs::GfsEnsembleClient;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("noaas_ark_bot=debug,info")
        .init();

    let args: Vec<String> = std::env::args().collect();
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let markets_limit = args
        .iter()
        .position(|a| a == "--markets-limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse::<usize>().ok());
    let config_path = args
        .iter()
        .position(|a| a == "--config")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "config/default.toml".to_string());

    let cfg = BotConfig::load_or_default(&config_path);
    info!(config = %config_path, dry_run = dry_run, "Weather bot starting");

    if dry_run {
        info!("DRY RUN - no orders will be placed");
    }

    let gfs_client = Arc::new(GfsEnsembleClient::new());
    let auth = ClobAuth::from_config(&cfg.clob);
    if !dry_run && !auth.is_configured() {
        anyhow::bail!("Live mode needs [clob] credentials - refusing to start");
    }
    let clob_client = Arc::new(ClobClient::new(auth));

    let bias = load_bias_or_default(&cfg.calibration.bias_file);
    let forecaster = EnsembleForecaster::new(bias);
    let signal_engine = Arc::new(SignalEngine::new(
        EngineConfig::from(&cfg.engine),
        forecaster,
    ));
    let risk_manager = Arc::new(RwLock::new(RiskManager::new(RiskConfig::from(&cfg.risk))));

    let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));

    let gamma_client = Arc::new(GammaClient::new());

    loop {
        interval.tick().await;
        if let Err(e) = run_cycle(
            &gamma_client,
            &gfs_client,
            &clob_client,
            &signal_engine,
            &risk_manager,
            dry_run,
            markets_limit,
        )
        .await
        {
            error!(error = %e, "Cycle failed");
        }
    }
}

fn load_bias_or_default(path: &str) -> BiasCorrection {
    noaas_ark_engine::calibration::load_or_default(path)
}

async fn run_cycle(
    gamma: &GammaClient,
    gfs: &GfsEnsembleClient,
    clob: &ClobClient,
    engine: &SignalEngine,
    risk: &RwLock<RiskManager>,
    dry_run: bool,
    markets_limit: Option<usize>,
) -> Result<()> {
    let mut markets = fetch_active_weather_markets(gamma).await?;
    info!(count = markets.len(), "Found active weather markets");
    if let Some(n) = markets_limit {
        markets.truncate(n);
    }

    let latest_run = determine_latest_model_run();
    info!(init_time = %latest_run.init_time, "Using model run");

    for market in &markets {
        let forecast_hour = compute_forecast_hour(&latest_run.init_time, &market.event_time);

        let ensemble_values = match gfs
            .fetch_t2m_ensemble(latest_run.init_time, forecast_hour, &market.station)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                warn!(market = %market.condition_id, error = %e, "Failed to fetch ensemble");
                continue;
            }
        };

        let orderbook = match clob.get_orderbook(&market.token_id_yes).await {
            Ok(ob) => ob,
            Err(e) => {
                warn!(market = %market.condition_id, error = %e, "Orderbook fetch failed");
                continue;
            }
        };

        if dry_run {
            let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
            let path = std::path::Path::new("data/orderbooks").join(format!("{}.jsonl", date));
            if let Err(e) = noaas_ark_market::save_snapshot(&path, &orderbook) {
                warn!(error = %e, "snapshot save failed");
            }
        }

        let signal = engine.evaluate(market, &ensemble_values, &orderbook, latest_run.clone());
        let signal = noaas_ark_engine::strategy::filter_signal(signal);

        if let Some(signal) = signal {
            info!(
                market = %market.question,
                model_prob = %signal.model_prob,
                model_uncertainty = %signal.model_uncertainty,
                market_price = %signal.market_price,
                edge = %signal.edge,
                confidence = %signal.confidence,
                "Signal generated"
            );

            let intent = engine.size_trade(&signal);

            let risk_verdict = {
                let risk_mgr = risk.read().await;
                match risk_mgr.can_trade(&intent) {
                    Ok(()) => "allowed",
                    Err(v) => {
                        warn!(violation = %v, "Trade blocked by risk manager");
                        "blocked"
                    }
                }
            };
            if risk_verdict == "blocked" {
                continue;
            }

            if dry_run {
                info!(
                    price = %intent.price,
                    size = %intent.size,
                    risk = risk_verdict,
                    "DRY RUN - would place order"
                );
                continue;
            }

            let order = OrderRequest {
                token_id: market.token_id_yes.clone(),
                side: "BUY".to_string(),
                price: intent.price,
                size: intent.size,
                order_type: "GTC".to_string(),
            };

            match clob.place_order(&order).await {
                Ok(resp) => {
                    info!(order_id = %resp.order_id, "Order placed successfully");
                    let mut risk_mgr = risk.write().await;
                    risk_mgr.add_position(&intent, resp.avg_price, resp.filled_size);
                }
                Err(e) => {
                    error!(error = %e, "Order placement failed");
                }
            }
        }
    }

    Ok(())
}

/// Live market discovery via Gamma (M1). Network failures propagate so the
/// cycle logs an error instead of silently trading nothing.
async fn fetch_active_weather_markets(gamma: &GammaClient) -> Result<Vec<WeatherMarket>> {
    gamma.fetch_active_weather_markets().await
}

/// Latest GFS run fully available (~4h after init).
fn determine_latest_model_run() -> ModelRunId {
    let now = Utc::now();
    let hour = now.hour();
    let latest_cycle = match hour {
        0..=3 => 18,
        4..=9 => 0,
        10..=15 => 6,
        16..=21 => 12,
        _ => 18,
    };

    let init_time = now
        .date_naive()
        .and_hms_opt(latest_cycle, 0, 0)
        .unwrap()
        .and_utc();

    // If we picked a cycle later than now (e.g. 18Z yesterday logic), roll back a day.
    let init_time = if init_time > now {
        init_time - chrono::Duration::days(1)
    } else {
        init_time
    };

    ModelRunId {
        model: NwpModel::Gfs,
        init_time,
        forecast_hour: 0,
    }
}

fn compute_forecast_hour(
    init_time: &chrono::DateTime<Utc>,
    event_time: &chrono::DateTime<Utc>,
) -> u32 {
    let duration = *event_time - *init_time;
    duration.num_hours().max(0) as u32
}
