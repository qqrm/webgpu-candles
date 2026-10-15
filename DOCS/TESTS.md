# Test Suite

The `tests/` folder contains unit and integration tests for the main components. Key files include:

- `viewport.rs` — verifies `Viewport` methods (coordinate conversions, panning and zoom)
- `geometry.rs` — generates candle vertices and compares them with a snapshot
- `offset.rs` — checks candle positioning by index and count
- `indicator_vertices.rs` — validates vertices for indicator and current price lines
- `ecs_full_pipeline.rs` — end-to-end test from WebSocket message through ECS to WebGPU

Snapshot fixtures are stored in `tests/fixtures`. The pipeline test does not create snapshots.

Internal module tests for the renderer can be found in `src/infrastructure/rendering/renderer/render_loop.rs`.

## Browser end-to-end tests

Playwright scenarios in `e2e/chart.spec.ts` build and serve the release bundle, mock Binance REST and WebSocket traffic, and verify:

- WebGPU chart startup without browser errors;
- the single connection status, price formatting, indicator toggles, zoom, and pan;
- concurrent BTC, ETH, and SOL streams plus instant market switching without reconnects;
- continued timeframe, pan, and zoom interaction after subsequent live ticks;
- native `1s` source aggregation into `2s` candles and zero-volume bucket removal;
- history backfill only after the viewport reaches the left edge.
- responsive large-chart layout and the one-million-candle LOD stress mode;
- zoom, pan, and live-mode recovery after the stress data set is loaded.

Install Chromium once and run the suite with:

```bash
npx playwright install chromium
npm run test:e2e
```

Failure screenshots, traces, and the HTML report are written to `test-results/` and `playwright-report/`.

## Required CI and coverage boundaries

`ci-gate` requires locked stable compilation, strict Clippy, Python regression and dependency-policy tests, browser Rust tests, Node logic tests, the Playwright WebGPU suite, dependency auditing, and measured completed-frame performance. Browser Rust tests explicitly select the Chrome binary paired with ChromeDriver. Tests run with `--no-fail-fast` so a failing test executable does not hide later failures.

`tests/performance_limit.rs` runs separately in release mode with the `performance` feature. It requires a real WebGPU adapter (software SwiftShader on CI), rejects invalid inputs and reports all 15 completed-frame samples. Ten Python regression tests reject absent/invalid/duplicate samples, zero or mismatched baselines, missing baseline files, and per-workload regressions; medians tolerate individual outliers. See [measurement details](../.github/workflows/PIPELINES.md).

`tests/app_context.rs` checks independent UI/ECS owners, disposal/remount, chart lifetime, and callback ownership. The renderer and render queue remain shared services; these tests do not establish independent GPU renderers for multiple application roots. Browser tests use software WebGPU and do not establish physical-device performance.

The 49 formerly compiled-only cases now use `wasm_bindgen_test` in the browser job. A disposable reactive owner supplies each migrated test with an explicit base-series interval and independent signals. The queue test awaits a trailing completion task and restores the previous renderer/queue. The zoom test calls the actual chart zoom operation and checks that the viewport changes while source candles, data revision and all moving averages remain unchanged.

The zoom QuickCheck property runs at least 256 successful finite-input cases plus explicit edge positions/deltas. Non-finite values are discarded because the tested property concerns finite UI coordinates and zoom deltas; this does not establish NaN/infinity handling by `ViewState`.

One separate native-only case, `tests/indicator_perf.rs::update_under_half_ms`, remains excluded from WASM. It asserts an absolute 0.5 ms indicator-update budget using `std::time::Instant`. The existing relative cached-frame benchmark cannot replace that contract: it measures different work and compares a ratio, not that absolute latency. A dedicated native measurement environment is still needed to validate this case without changing its meaning. No production behavior or performance threshold is changed by the harness migration.
