# NOAAsArk - Polymarket Weather Bot (Rust)

NWP-driven Polymarket weather market bot. Compares ensemble model probabilities
(GFS / ECMWF / HRRR) against Polymarket CLOB prices and trades the edge with
fractional-Kelly sizing and hard risk limits.

## Layout

- `crates/core` - shared types, config, errors
- `crates/nwp` - weather model ingestion (GFS ensemble via NOMADS / AWS S3, GRIB2 via wgrib2)
- `crates/market` - Polymarket CLOB REST + WebSocket client
- `crates/engine` - signal generation, calibration, strategy
- `crates/execution` - order lifecycle, positions, risk
- `crates/backtest` - historical simulator, orderbook replay, metrics
- `crates/bot` - main binary (model-run loop)
- `config/` - `default.toml`, `production.toml`
- `data/` - local cache (gitignored except `.gitkeep`)

## Quickstart

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo check --workspace
cargo run -p noaas-ark-bot -- --dry-run
```

Copy `config/default.toml` to `config/production.local.toml` and fill in
`[clob]` credentials. Never commit credentials.

## GRIB2 notes

v1 shells out to `wgrib2` for extraction (battle-tested). Install it and make
sure `wgrib2` is on PATH. NOMADS is rate-limited; prefer the AWS S3 mirror
`noaa-gefs-pds` (same data, no rate limits).

## Dry run first

Always run with `--dry-run` for weeks before live. The bot also enforces
`min_hours_to_resolution`, max exposure, per-market caps, and daily loss halt.

## Status

v0.1 scaffold - compiles, dry-run loop, no live orders placed without explicit
credentialed config.
