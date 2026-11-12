# PROJECT_MAP.md - NOAAsArk

What each top-level path is, what is authoritative, what is data-only.

- `Cargo.toml` - workspace root, members + shared deps. Authoritative.
- `crates/core/` - shared types (`WeatherMarket`, `Signal`, `TradeIntent`), config loader, errors. Imported by all crates.
- `crates/nwp/` - NWP ingestion only. GFS fetcher, ECMWF stub, HRRR stub, GRIB2 via wgrib2, ensemble-to-probability, station registry. No trading logic.
- `crates/market/` - Polymarket CLOB REST + WebSocket, auth signing, local orderbook state, market/order/trade types. No strategy logic.
- `crates/engine/` - signal generation (edge), calibration, strategy. Pure functions over model probs + orderbooks.
- `crates/execution/` - order lifecycle, position tracking, risk limits. Gates every intent.
- `crates/backtest/` - simulator, orderbook replay, PnL/calibration/Sharpe metrics. Never touches live API.
- `crates/bot/` - thin binary: wires the crates, runs the model-run loop, honors `--dry-run`.
- `config/` - `default.toml` (safe defaults), `production.toml` (template, no secrets). Real secrets live in `config/production.local.toml` or env, never committed.
- `data/` - DATA ONLY, never edited by hand, never committed (except `.gitkeep`):
  - `data/grib/` - downloaded GRIB2 cache
  - `data/orderbooks/` - stored orderbook snapshots for replay
  - `data/calibration/` - learned bias params (`bias_params.json`)

Credentials: names only - `clob.api_key`, `clob.api_secret`, `clob.private_key`
via `config/production.local.toml` or env `POLYMARKET_*`. No values in repo.
