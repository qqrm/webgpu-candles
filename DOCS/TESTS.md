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
