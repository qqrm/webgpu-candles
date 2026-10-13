# 🦀 Bitcoin Chart WASM - Current Architecture v4.0

## 📊 Current Status

**Real-time Bitcoin chart using WebGPU, Leptos and WebSocket**

- ✅ Live data from Binance WebSocket
- ✅ WebGPU rendering (60 FPS)
- ✅ Technical indicators: SMA20, SMA50, SMA200, EMA12, EMA26, Ichimoku
- ✅ Solid line for the current price
- ✅ Interactive tooltip
- ✅ Professional look similar to TradingView
- ✅ Historical data backfill via REST when panning left
See [PIPELINES.md](../.github/workflows/PIPELINES.md) for optimization details.

## 🗂️ File Structure

```
src/
├── app.rs                  # Leptos UI components and reactivity
├── lib.rs                  # WASM exports (hydrate, main)
├── simple_shader.wgsl      # WebGPU shaders for candles
├── domain/
│   ├── chart/
│   │   ├── entities.rs     # Chart, ChartData
│   │   └── value_objects.rs # ChartType, Viewport
│   ├── market_data/
│   │   ├── entities.rs     # Candle, CandleSeries
│   │   ├── value_objects.rs # OHLCV, Price, Volume, Symbol
│   │   └── services.rs     # Validation and operations
│   ├── logging.rs          # Logger abstractions
│   └── errors.rs           # Simplified AppError
└── infrastructure/
    ├── websocket/
    │   ├── binance_client.rs # Binance WebSocket client
    │   └── dto.rs           # JSON DTO structures
    ├── rendering/
    │   ├── renderer/          # WebGPU renderer pieces
    │   └── gpu_structures.rs  # GPU vertex structures
    └── mod.rs               # Infrastructure services
```

## ⚡ Data Flow

```
Binance WebSocket → BinanceClient → Leptos Signals → WebGPU → Canvas
                                          ↓
                                    Tooltip + UI Updates
```

## 🧩 Key Components

### **app.rs - Leptos Frontend**
- `App()` - main component with CSS
- `Header()` - price, candle count, status
- `ChartContainer()` - WebGPU rendering + mouse events
- `ChartTooltip()` - interactive tooltip

### **renderer** - GPU rendering
- Candle rendering (green/red)
- Technical indicators (SMA20, SMA50, SMA200, EMA12, EMA26, Ichimoku)
- Solid price line (yellow)
- 300-candle scrolling buffer

### **binance_client.rs - WebSocket**
- Connects to `wss://stream.binance.com`
- Parses kline events
- Updates Leptos signals

## 📡 Global Signals

```rust
GLOBAL_CURRENT_PRICE: f64    // Current BTC price
GLOBAL_CANDLE_COUNT: usize   // Number of candles
GLOBAL_IS_STREAMING: bool    // WebSocket status
TOOLTIP_DATA: TooltipData    // Tooltip info
```

## 🎨 Visual Elements

- **Candles**: green (up) / red (down)
- **SMA20**: red line (simple 20-period average)
- **SMA50**: yellow line (simple 50-period average)
- **SMA200**: blue line (simple 200-period average)
- **EMA12**: purple line (12-period exponential average)
- **EMA26**: cyan line (26-period exponential average)
- **Ichimoku**: cloud with Tenkan, Kijun, Senkou and Chikou
- **Price**: yellow solid line + orange label
- **Tooltip**: black with OHLC + Volume + % change

## 🔧 Technical Details

**WebGPU Pipeline:**
- Vertex buffer: 100k vertices
- Shaders: `simple_shader.wgsl`
- Coordinates: NDC [-1, 1]
- Colors: via uniform buffer

**WebSocket:**
- Interval: 2s candles
- Symbol: BTCUSDT
- Auto-reconnect with exponential backoff (see [implementation](src/infrastructure/websocket/binance_client.rs#L146-L223))

**Leptos:**
- SSR disabled (client only)
- Reactive updates
- Inline CSS

## 📦 Build

```bash
# Install the wasm32 target once
rustup target add wasm32-unknown-unknown

# Development
cargo build --target wasm32-unknown-unknown

# Release
wasm-pack build --target web --release

# Serve
python -m http.server 8080
```

## 🎯 Project Status

**Done:**
- Real-time trading chart ✅
- Technical indicators ✅
- Professional UI ✅
- WebGPU performance ✅

**Architecture:** simple, clean, working 🚀

## Application state ownership

`App` creates a dedicated reactive owner and provides `AppContext`. The context owns UI signals and its own ECS world; owner cleanup aborts streams and clears chart entities before signals are disposed. Charts first requested by transient effects are allocated under the application owner. DOM callbacks and queued render tasks retain their originating owner and ignore work after disposal.

`global_state` remains a compatibility facade over the current context. New consumers can use `AppContext` directly. `ui_state` holds plain `TooltipData` and `LineVisibility`; tooltip formatting receives its symbol explicitly. Domain data remains ordinary Rust data. ECS chart components still adapt Leptos signals; removing that adapter and the single renderer service is outside this first migration step.

Lifecycle tests dispose and remount independent reactive owners. In Leptos CSR, `create_runtime` returns the shared runtime, so owner disposal is the meaningful browser lifecycle boundary.
