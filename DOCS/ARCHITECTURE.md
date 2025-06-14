# 🦀 Bitcoin Chart WASM - Current Architecture v4.0

## 📊 Current Status

**Real-time Bitcoin chart using WebGPU, Leptos and WebSocket**

- ✅ Live data from Binance WebSocket
- ✅ WebGPU rendering (60 FPS)
- ✅ Moving averages: SMA20, EMA12
- ✅ Solid line for the current price
- ✅ Interactive tooltip
- ✅ Professional look similar to TradingView
See [PERFORMANCE.md](./PERFORMANCE.md) for optimization details.

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
- Moving averages (SMA20, EMA12)
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
- **EMA12**: purple line (12-period exponential average)
- **Price**: yellow solid line + orange label
- **Tooltip**: black with OHLC + Volume + % change

## 🔧 Technical Details

**WebGPU Pipeline:**
- Vertex buffer: 100k vertices
- Shaders: `simple_shader.wgsl`
- Coordinates: NDC [-1, 1]
- Colors: via uniform buffer

**WebSocket:**
- Interval: 1m candles
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
