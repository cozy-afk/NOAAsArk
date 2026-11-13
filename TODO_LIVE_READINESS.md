# NOAAsArk Live-Market Readiness TODO

Persistent work list. Live trading is explicitly LAST. Checked items are done and
verified; unchecked items are open. Work top to bottom within each phase.

Source: live-market audit 2026-11-12. Repo: https://github.com/cozy-afk/NOAAsArk

---

## Phase 0 - Cannot trade correctly (blockers, do these first)

- [ ] **B1. Replace placeholder order signing** (`crates/market/src/auth.rs:28-51`)
  - `sign_order` uses `std::collections::hash_map::DefaultHasher` over the JSON body, in-code marked NOT for production.
  - Polymarket CLOB needs its real signature scheme (EIP-712 / API-key HMAC per docs).
  - Also: `ClobAuth` derives `Debug` and contains `private_key` - any debug print of the client leaks the key into logs. Redact or hand-write Debug.
  - Verify with one far-out-of-the-money order + cancel on a funded account.

- [ ] **B2. Make the NO side expressible** (`crates/engine/src/signal.rs:104-147`, `crates/bot/src/main.rs:146-151`)
  - `evaluate` computes `edge_no` correctly, but `size_trade` always sets `Side::Buy`, and main hardcodes `token_id_yes` + `"BUY"`.
  - When the edge is on NO, `intent.price` is `1 - bid` (a NO price) but the order buys the YES token at that price: wrong token, wrong price.
  - Fix: carry side/token through the intent (BUY `token_id_no` for a NO edge, or SELL YES) and make main use `intent.side` + the matching token.

- [ ] **B3. Add fee accounting, taker minimum, and tick rounding** (`crates/backtest/src/simulator.rs:24-26`, `crates/bot/src/main.rs:146-151`, `crates/market/src/clob.rs:101-140`)
  - Backtest knows `0.07 * price * (1 - price)` but the engine compares `model_prob` to raw best ask/bid with no fee deduction.
  - `size_trade` uses `size.max(1.0)` = 1 share, not 1 USD notional. At 0.90 that is 0.90 USD and the venue rejects it.
  - Prices are passed raw with no `tick_size` rounding.
  - Fix: net fees out of edge, enforce notional >= 1.00 USD, round price to the market tick, extract actual fill price and fee from the response instead of the estimate.

## Phase 1 - Cannot survive fills (do before any funded run)

- [ ] **R1. Make the daily-loss halt actually fire** (`crates/execution/src/risk.rs:44-115`)
  - `daily_pnl` starts at 0 and `record_pnl` is never called anywhere, so `can_trade` always sees 0 loss.
  - Positions are in-memory only, never removed on resolution or sale; `add_position` overwrites the same `condition_id`.
  - A restart wipes all positions and the bot re-trades them.
  - Fix: persist positions, call `record_pnl` on settlement, reconcile open orders at boot (`get_open_orders` exists, never called), add a manual kill flag.

- [ ] **R2. Handle partial fills and order status** (`crates/market/src/clob.rs:91-99`, `crates/bot/src/main.rs:154-159`)
  - `OrderResponse` defaults `filled_size` and `avg_price` to 0.0; `status` is a raw string nobody matches on.
  - `add_position` is called with whatever came back, including zeros.
  - The engine evaluates a taker cross (best ask) but submits `order_type: "GTC"`, which rests instead of crossing.
  - Fix: check status, use `filled_size`/`avg_price` only when real, handle partials, treat GTC vs taker differently.

## Phase 2 - Cannot price the weather (do before trusting a signal)

- [ ] **D1. Stop re-fetching the same GRIB 31x per market** (`crates/nwp/src/gfs.rs:33-79`)
  - Every market triggers 31 sequential NOMADS downloads, each a full 0.25-degree GRIB2 parsed via wgrib2.
  - Eleven brackets on one city refetch the same 31 files 11 times.
  - One slow member (120s timeout) stalls the whole 5-minute loop.
  - NOMADS rate-limits; the S3 mirror function exists but is never called in the loop.
  - `wgrib2_available` exists but main never checks it, so a missing binary fails every member.
  - Fix: fetch each run once per cycle, share across markets, fetch concurrently, prefer S3, cache by `(init_time, hour)`, fail fast with a clear error.

- [ ] **D2. Respect model-run publication lag** (`crates/bot/src/main.rs:177-206`)
  - Picks the latest cycle by wall clock with no availability check. GFS 12Z is not complete until ~16:30Z.
  - Requesting missing files gives per-member 404s, then the market bails at the 50% member gate.
  - Fix: log which run was attempted, fall back to the newest complete run.

- [ ] **D3. Confirm what `endDate` means** (`crates/market/src/gamma.rs:200-210`)
  - `event_time` and `resolution_time` are both set to `endDate`.
  - For daily high/low markets the observation window is the calendar day, not the close timestamp.
  - `forecast_hour = endDate - init_time` can point at the wrong lead time.
  - Fix: confirm against one real market; set `event_time` to the observation window explicitly.

- [ ] **D4. Refuse non-temperature metrics until the right variable is fetched** (`crates/nwp/src/gfs.rs`, `crates/nwp/src/ensemble.rs:130-141`)
  - The fetcher pulls only 2m temperature, but the metric enum includes precipitation, snowfall, and wind.
  - Those paths reuse temperature values and subtract a degrees-based `mean_bias` from inches or mph.
  - Fix: fetch the matching variable per metric, or refuse to trade non-temperature markets.

- [ ] **D5. Verify every city's station mapping** (`crates/nwp/src/station.rs:14-20`)
  - NYC was fixed from Central Park to LaGuardia (KLGA) with a "verified" comment. Apply the same verification to every other city.
  - A 50 km station error on a threshold market is a permanent bias.

## Phase 3 - Cannot stay alive (ops)

- [ ] **O1. Do not let one slow market block all markets** (`crates/bot/src/main.rs:58-72, 79-92`)
  - The loop awaits each market sequentially; `fetch_active_weather_markets` uses `?` so one Gamma failure skips every market that cycle.
  - Fix: per-market timeouts, fetch books concurrently, serve the last good market list on discovery failure.

- [ ] **O2. Remove panic paths on bad book data** (`crates/market/src/clob.rs:62-63`)
  - `sort_by` uses `partial_cmp().unwrap()`, so one NaN price panics the task.
  - Fix: total ordering or filter non-finite levels.

- [ ] **O3. Fail closed on bad config** (`crates/core/src/config.rs:105`, `crates/bot/src/main.rs:32`)
  - `load_or_default` silently substitutes defaults on a bad `--config` path. A typo runs with 25 USD sizing and empty creds instead of refusing.
  - Fix: fail closed on bad config in live mode.

- [ ] **O4. Add timeouts and retries to Gamma discovery** (`crates/market/src/gamma.rs:74-121`)
  - Plain `reqwest::Client::new()` with no timeout, no retry/backoff, `MAX_PAGES 6`.
  - A transient 403 or timeout empties the whole tradable set for that cycle.
  - Fix: timeouts, retries, log the raw counts already collected.

- [ ] **O5. Build the backtest before trusting any edge** (`crates/backtest/src/*`, `crates/bot/src/main.rs`)
  - The simulator returns an empty result, replay reads snapshots, but the bot never writes orderbook snapshots. No calibration history either.
  - Fix: record books + model probs, then backtest with the same fee and sizing code as live.

- [ ] **O6. Wire or delete dead modules** (`crates/engine/src/timing.rs`, `crates/engine/src/multi_model.rs`)
  - `MarketReactionModel.should_trade` and the multi-model blender exist but main never calls them.
  - Fix: wire them or delete them so nobody assumes protection that is not running.

- [ ] **O7. Secrets hygiene** (`.gitignore`, `crates/market/src/auth.rs`)
  - `.gitignore` correctly excludes `production.local.toml` and `.env`; `config/production.toml` is a template. Keep it that way.
  - Never put the real private key or relayer keys in config or logs. Fix the `Debug` derive on `ClobAuth`.

## Phase 4 - Live (only after Phases 0-3 are checked)

- [ ] Paper-trade with snapshot logging for at least 20 model cycles.
- [ ] Reconcile paper PnL against Gamma settlement for every market traded.
- [ ] One live order: report handshake, signal, order status, latency, post-order open-order count.
- [ ] Only then scale size.

---

## Notes

- Repo discipline: this is a separate repo because it is a different product with its own lifecycle (Rust + NWP data), not a variant of `polymarket5m`.
- Branch policy from `polymarket5m` does not apply here yet; add `BRANCHING.md` before the first deploy.
- `data/` stays gitignored except `.gitkeep`. Never commit GRIB2 or orderbook snapshots.
