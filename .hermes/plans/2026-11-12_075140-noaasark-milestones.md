# NOAAsArk Next Milestones Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Take the v0.1 Rust scaffold from "compiles + empty dry-run" to a credible dry-run that discovers real Polymarket weather markets, reads real CLOB orderbooks, generates signals, and logs them - with a backtest/calibration path behind it. Live orders remain gated until M7.

**Architecture:** Keep the existing crate boundaries (core/nwp/market/engine/execution/backtest/bot). Milestones are ordered so each one is independently verifiable and leaves the workspace compiling + `cargo check --workspace` green. No live orders until M7; `--dry-run` is the default operational mode through M6.

**Tech Stack:** Rust 1.8x (cargo at ~/.cargo/bin), tokio, reqwest (rustls), serde/toml, chrono, tracing, polars (available but unused yet), wgrib2 (external).

**Primary constraint:** user's stated priority = credible dry-run with real market data first. So M1-M3 before M4-M6.

---

## Milestone 0 - Baseline hygiene (prerequisite for everything)

**Objective:** Make the repo safe to iterate: tests runnable, formatting, branch discipline, no silent regressions.

### Task M0.1: Install git hooks + verify branch
- **Files:** `.githooks/` (if present), `BRANCHING.md`
- **Step 1:** `git branch --show-current` must NOT be master/main/prod for edits. Create `dev/milestones`.
- **Step 2:** `bash scripts/install_hooks.sh` if exists.
- **Verification:** `git branch --show-current` prints `dev/milestones`.

### Task M0.2: Add a minimal test harness
- **Files:** Create `crates/core/tests/smoke.rs`, `crates/engine/tests/smoke.rs`.
- **Step 1:** Add `[[test]]` entries or rely on `tests/` auto-discovery; add `dev-dependencies` (none needed for smoke).
- **Step 2:** Write a smoke test that loads `config/default.toml` and constructs `BotConfig::load`.
- **Verification:** `cargo test -p noaas-ark-core` passes.

### Task M0.3: cargo fmt + clippy baseline
- **Files:** all crates.
- **Step 1:** `cargo fmt --all`.
- **Step 2:** `cargo clippy --workspace -- -D warnings` (record any pre-existing warnings; fix or allow).
- **Verification:** `cargo check --workspace` still green; `cargo fmt --all --check` clean.

### Task M0.4: README quickstart accuracy
- **Files:** `README.md`.
- **Step 1:** Document cargo path (`export PATH="$HOME/.cargo/bin:$PATH"`), wgrib2 install, dry-run command.
- **Verification:** manual read-through.

---

## Milestone 1 - Market discovery (Gamma API) [BLOCKS dry-run]

**Objective:** Replace `fetch_active_weather_markets()` stub with real Gamma API discovery of open weather markets, parsed into `WeatherMarket`.

### Task M1.1: Add Gamma client module
- **Files:** Create `crates/market/src/gamma.rs`; register in `crates/market/src/lib.rs`.
- **Step 1:** Define `GammaClient { http }` with `fetch_active_weather_markets(&self) -> Result<Vec<WeatherMarket>>`.
- **Step 2:** GET `https://gamma-api.polymarket.com/markets?active=true&closed=false&limit=...` with tag filter for weather.
- **Step 3:** Parse JSON into intermediate structs; map to `WeatherMarket` (condition_id, question, token_id_yes/no, station, metric, event_time, resolution_time).
- **Verification:** unit test with a saved JSON fixture in `crates/market/tests/fixtures/gamma_sample.json`.

### Task M1.2: Station/metric parsing from market question
- **Files:** Modify `crates/nwp/src/station.rs` (add lookup by city), new `crates/market/src/parse.rs` or in gamma.rs.
- **Step 1:** Parse question text for city + metric + threshold + date (e.g. "Highest temperature in NYC on Nov 13?").
- **Step 2:** Map city -> Station (extend registry to cover Polymarket weather cities: NYC, Chicago, LA, Houston, Phoenix, Miami, Dallas, Seattle, Denver, Atlanta).
- **Step 3:** Return `None` (skip market) if unparseable; log count.
- **Verification:** unit tests for 5 representative questions.

### Task M1.3: Wire into bot
- **Files:** `crates/bot/src/main.rs`.
- **Step 1:** Replace `fetch_active_weather_markets()` stub body to call `GammaClient`.
- **Step 2:** Pass client into `run_cycle`.
- **Verification:** `cargo run -p noaas-ark-bot -- --dry-run` logs non-zero market count (network dependent; allow 0 with warning).

### Task M1.4: Market discovery tests
- **Files:** `crates/market/tests/gamma_parse.rs`.
- **Step 1:** Offline parse tests against fixture.
- **Verification:** `cargo test -p noaas-ark-market`.

---

## Milestone 2 - Real CLOB reads + correct auth signing

**Objective:** Make orderbook fetches robust and replace placeholder signing with the real Polymarket CLOB scheme (or at least a documented, testable seam).

### Task M2.1: Fix CLOB orderbook deserialization
- **Files:** `crates/market/src/clob.rs`.
- **Step 1:** Confirm Gamma/CLOB JSON shape (`bids`/`asks` arrays of `{price,size}`); add `#[serde(rename_all="camelCase")]` if needed; handle missing fields.
- **Step 2:** Add `timestamp` parsing (RFC3339) with fallback to now.
- **Verification:** unit test parsing a real captured snapshot.

### Task M2.2: Real CLOB auth signature scheme
- **Files:** `crates/market/src/auth.rs`.
- **Step 1:** Research Polymarket CLOB signing (EIP-712 order struct, HMAC for L2). Document in a comment with source link.
- **Step 2:** Implement HMAC-SHA256 over canonical body using `hmac` + `sha2` crates; add to workspace deps.
- **Step 3:** Keep `is_configured()` gate; add `#[cfg(test)]` mock signer.
- **Verification:** `cargo test -p noaas-ark-market` (signature determinism test).

### Task M2.3: Orderbook fetch retry + rate limit
- **Files:** `crates/market/src/clob.rs`.
- **Step 1:** Add retry with backoff for 429/5xx; timeout 10s.
- **Step 2:** Log failures, don't abort whole cycle.
- **Verification:** compile + manual.

---

## Milestone 3 - End-to-end credible dry-run

**Objective:** A dry-run that, given real markets, fetches ensemble + orderbook, generates signals, sizes trades, runs risk gates, and logs a structured record per candidate. No orders placed.

### Task M3.1: Structured dry-run logging
- **Files:** `crates/bot/src/main.rs`, `crates/engine/src/signal.rs`.
- **Step 1:** Emit JSON-ish tracing fields: market, model_prob, market_price, edge, confidence, size, risk verdict.
- **Step 2:** Add a `--markets-limit` arg for controlled runs.
- **Verification:** run dry-run for 1 cycle, inspect logs.

### Task M3.2: Orderbook snapshot persistence (for replay)
- **Files:** `crates/market/src/clob.rs`, `crates/backtest/src/replay.rs`.
- **Step 1:** Add `save_snapshot(path, &OrderbookSnapshot)` helper.
- **Step 2:** Bot in dry-run optionally writes `data/orderbooks/<date>.jsonl`.
- **Verification:** file written after run.

### Task M3.3: Risk manager wiring correctness
- **Files:** `crates/execution/src/risk.rs`, `crates/bot/src/main.rs`.
- **Step 1:** Fix `add_position` to accumulate (currently overwrites per market) - track size accumulation.
- **Step 2:** Add `update_pnl` call path (stub ok for now).
- **Verification:** unit test in risk.rs.

### Task M3.4: Smoke integration test
- **Files:** `crates/bot/tests/dry_run.rs` (offline, using fixtures).
- **Step 1:** Construct a market + orderbook fixture + ensemble; run engine.evaluate; assert signal or None deterministically.
- **Verification:** `cargo test -p noaas-ark-bot`.

---

## Milestone 4 - Backtest harness

**Objective:** Replay stored snapshots + model probs through the engine; report PnL, win rate, Sharpe, calibration.

### Task M4.1: Simulator loop
- **Files:** `crates/backtest/src/simulator.rs`.
- **Step 1:** Implement `run_backtest(snapshots, model_probs, config) -> BacktestResult` applying taker fee.
- **Step 2:** Resolve outcome from `WeatherMarket.resolved_outcome`.
- **Verification:** unit test with synthetic data.

### Task M4.2: Metrics + reporting
- **Files:** `crates/backtest/src/metrics.rs`.
- **Step 1:** Add calibration error (Brier), max drawdown, trade count by market.
- **Step 2:** CLI binary or `cargo run -p noaas-ark-backtest` (new bin) printing report.
- **Verification:** run on sample data.

### Task M4.3: Data hygiene
- **Files:** `data/.gitkeep`, `.gitignore`.
- **Step 1:** Ensure `data/orderbooks/`, `data/grib/`, `data/calibration/` gitignored except `.gitkeep`.
- **Verification:** `git status` clean after generating data.

---

## Milestone 5 - Calibration study

**Objective:** Learn bias params from historical model-vs-observed data; persist to `data/calibration/bias_params.json`.

### Task M5.1: Bias fitting routine
- **Files:** `crates/engine/src/calibration.rs` (extend) or new `crates/nwp/src/calibration.rs`.
- **Step 1:** Given (model_ensemble, observed_outcome) pairs, fit mean_bias + spread_factor minimizing Brier.
- **Step 2:** Save JSON.
- **Verification:** unit test on synthetic.

### Task M5.2: Wire bias into forecaster
- **Files:** `crates/bot/src/main.rs` (already loads).
- **Step 1:** Ensure `load_bias_or_default` reads fitted file; log which bias used.
- **Verification:** dry-run logs bias params.

---

## Milestone 6 - Model parity (ECMWF + HRRR) + blending

**Objective:** Implement ECMWF and HRRR fetchers; blend in strategy layer per config weights.

### Task M6.1: ECMWF fetcher
- **Files:** `crates/nwp/src/ecmwf.rs`.
- **Step 1:** Fetch ECMWF open data GRIB2, extract T2m ensemble, parse.
- **Verification:** integration test (network, ignored by default).

### Task M6.2: HRRR fetcher
- **Files:** `crates/nwp/src/hrrr.rs`.
- **Step 1:** Fetch HRRR prod GRIB2, extract T2m (deterministic, not ensemble).
- **Verification:** integration test.

### Task M6.3: Multi-model blending in strategy
- **Files:** `crates/engine/src/strategy.rs`.
- **Step 1:** `filter_signal` becomes `blend_signals(gfs_prob, ecmwf_prob, weights) -> prob`; apply min-edge gates.
- **Verification:** unit tests.

---

## Milestone 7 - Live execution hardening (NOT before M1-M6 green)

**Objective:** Make live mode actually safe: real signing, order lifecycle, fill tracking, kill switch.

### Task M7.1: Real order signing end-to-end
- **Files:** `crates/market/src/auth.rs`, `crates/market/src/clob.rs`.
- **Verification:** place a $1 test order on a low-stakes market in a sandbox account; cancel.

### Task M7.2: Order lifecycle + fill tracking
- **Files:** `crates/execution/src/manager.rs`, `position.rs`.
- **Step 1:** Track pending/open/filled/cancelled; reconcile via get_open_orders.
- **Verification:** unit tests.

### Task M7.3: Kill switch + daily halt
- **Files:** `crates/execution/src/risk.rs`, bot.
- **Step 1:** File-flag kill switch (`artifacts/state/kill_switch.flag`) checked each cycle; daily loss halt already in risk.rs.
- **Verification:** integration test.

---

## Milestone 8 - Shadow trial

**Objective:** Run dry-run for weeks, collect signals + hypothetical PnL, decide go/no-go for live.

### Task M8.1: Shadow telemetry
- **Files:** new `crates/bot/src/shadow.rs` or logging.
- **Step 1:** Persist each signal + hypothetical fill + resolution outcome.
- **Verification:** data accumulates.

### Task M8.2: Go/no-go review
- **Files:** doc.
- **Step 1:** Review win rate, calibration, drawdown vs thresholds; document decision.

---

## Risks & tradeoffs

- **Network dependency in CI:** integration tests that hit Gamma/CLOB/NOMADS must be `#[ignore]` by default; CI runs unit tests only.
- **wgrib2 external binary:** GRIB2 parsing depends on it; provide `wgrib2_available()` check at startup with clear error.
- **Auth correctness is the #1 live-safety risk:** do not enable live mode until M7.1 verified with a real test order.
- **Market discovery parsing fragility:** Gamma question text formats change; keep parser tolerant and skip unparseable markets rather than crash.
- **No tests today:** M0.2 is non-optional; every subsequent task must add at least one test.

## Open questions

1. Which additional weather cities beyond the 5 in StationRegistry should be supported? (Polymarket lists vary by season.)
2. Is there a budget/size cap for the shadow trial (max hypothetical USDC)?
3. Preferred storage format for snapshots: JSONL (current) vs parquet (polars available)?

---

## Technical critique integration (Problems 1-10)

The plan now incorporates the ten problems raised in the review, mapped to
revised priorities:

Priority 1 (done in this pass):
- P1 Distribution fitting -> `crates/nwp/src/distribution.rs` (EnsembleDistribution, erfc-based normal CDF, delta-method uncertainty). `EnsembleForecaster::probability` now uses it instead of n/N counting.
- P2 Station bias -> `crates/nwp/src/station_bias.rs` (StationBias + learn_station_bias with seasonal + range components).
- P7 Resolution criteria -> `crates/core/src/resolution.rs` (ResolutionCriteria, DataSource, TimeWindow, RoundingRule, MissingDataRule).

Priority 2 (done in this pass):
- P4 Threshold regime -> `crates/engine/src/regime.rs` (ThresholdRegime classify + min_edge + size_multiplier), wired into SignalEngine::evaluate.
- P3 Market reaction timing -> `crates/engine/src/timing.rs` (MarketReactionModel).
- P8 Conditional calibration -> `crates/engine/src/calibration.rs` (ConditionalCalibrator + IsotonicCalibration).

Priority 3 (done in this pass):
- P5 Multi-model blending -> `crates/engine/src/multi_model.rs` (ModelBlender).
- P6 Correlation-aware risk -> `crates/execution/src/correlation.rs` (CorrelationManager).
- P9 Competition detection -> deferred (needs orderbook history); milestone M3+.
- P10 Extreme events -> `crates/engine/src/extreme.rs` (ExtremeEventDetector).

Remaining follow-ups: P9 competition detector, market microstructure analysis,
continuous recalibration, and wiring station_bias into the forecaster (currently
available but not auto-applied).

- `cargo check --workspace` green.
- `cargo test --workspace` green (unit tests; integration ignored).
- `cargo fmt --all --check` clean.
- `cargo clippy --workspace -- -D warnings` clean (or documented allows).
- Dry-run command runs one full cycle without panic.

---

**Estimated order:** M0 (1-2 days) -> M1 (3-5 days) -> M2 (2-4 days) -> M3 (2-3 days) -> M4 (3-5 days) -> M5 (2-3 days) -> M6 (4-6 days) -> M7 (3-5 days, gated) -> M8 (weeks of data).