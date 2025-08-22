# WebGPU Candles - Technical Specification (v1.0)

## 1. Purpose and scope
A high-performance candlestick chart in Rust + WebAssembly that renders with WebGPU and streams real-time market data from Binance over WebSocket. Must feel comparable to Binance or Bybit web charts regarding zoom, pan, history backfill, and common indicators. No custom backend.

## 2. Technology stack
- Language: Rust 1.78+ target wasm32-unknown-unknown
- UI: Leptos reactive components
- Graphics: WebGPU via wgpu, WGSL shaders
- Concurrency: Web platform event loop, optional Rayon for native benchmarks
- Packaging: Trunk or wasm-pack
- Data provider: Binance WebSocket for live klines, Binance REST uiKlines/klines for history
- Repo docs and README. Binance API references.

## 3. Data model
### 3.1 Candle
```
struct Candle {
  open_time_ms: u64
  close_time_ms: u64
  open: f64
  high: f64
  low: f64
  close: f64
  volume: f64
  is_closed: bool
}
```
Key: open_time_ms. Deduplicate by key. Close flag flips on kline close event.

### 3.2 Series
- `Candles: Vec<Candle>` sorted by `open_time` ascending
- Append-only, dedup by `open_time` on merge
- Historical batches merged atomically

### 3.3 Indicators
- SMA periods: 20, 50, 200
- EMA periods: 12, 26
- Computation:
  - SMA rolling window with running sum
  - EMA incremental with `alpha = 2/(N+1)`
- Optional: Ichimoku later

## 4. State separation
- DomainState: immutable time scale
  - `timeframe: Duration` (example 1s per candle)
  - `series: candles`
  - `indicator buffers` (values in candle coordinates)
- ViewState: pure presentation
  - `pixels_per_candle (ppc)` in `[ppc_min, ppc_max]`
  - `pan_offset_px`
  - `cursor_anchor_ratio` in `[0,1]` for zoom anchoring
  - `viewport` width and height in px
- No user interaction mutates timeframe or series, only ViewState.

## 5. Interaction model
### 5.1 Zoom
- Input: mouse wheel or pinch
- Update only `pixels_per_candle`
- Anchor at cursor: keep candle index under cursor invariant
- Clamp ppc range, minimum 1 visible candle
- No data resampling

### 5.2 Pan
- Drag translates `pan_offset_px`
- Left-edge pan triggers history preload threshold

### 5.3 Resize
- Recompute visible range from ViewState and canvas size

## 6. Rendering pipeline
Order per frame:
1. Grid and axes
2. Candles (body + wicks) in a dedicated pipeline
3. Volume bars
4. Indicators polylines (SMA, EMA)
5. Current price line and last trade marker
6. Overlays (selection, crosshair)

Buffers:
- Static grid buffers rebuild only on resize
- Candle instance buffer updated for visible slice only
- Indicator vertex buffers updated incrementally on new data

Performance targets:
- 60 FPS desktop for 10k candles visible
- No GC spikes > 2 ms per frame

## 7. Data ingestion
### 7.1 Live stream
- WebSocket stream name: `<symbol>@kline_<interval>` for Spot
- Parse events, update current forming candle, set `is_closed` on final tick
- Reconnect with jittered backoff and sequence guard
- Ref. Binance WebSocket kline naming.

### 7.2 History backfill
- REST: `/api/v3/uiKlines` preferred for charting or `/api/v3/klines` fallback
- Parameters: symbol, interval, `endTime` or `startTime`, limit up to 1000
- Merge:
  - Append older batch to front
  - Sort by `open_time` and dedup
  - Recompute indicators only for affected window
- Refs. uiKlines and general REST endpoints.

### 7.3 Rate limits and safety
- Single outstanding history request
- Respect weight and per-minute ceilings
- Handle HTTP fallback domains if needed
- Ref. base endpoints.

## 8. Symbol switching
- Unsubscribe and dispose previous WebSocket and tasks
- Optional keep-or-reset ViewState
- Bootstrap with initial REST batch (1000 klines) then attach WebSocket
- Ignore late messages with `connection_id` check

## 9. Testing and QA
- Unit tests for SMA/EMA against fixtures
- Property tests for zoom invariants
- Integration test: history backfill across 3 batches, no gaps
- Performance test: visible 10k, FPS ≥ 60 on reference desktop
- Memory test: switch symbols 20x, no leaks

