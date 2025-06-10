# 🦀 Bitcoin Chart WASM - Актуальная архитектура v4.0

## 📊 Что у нас есть сейчас

**Real-time Bitcoin торговый график с WebGPU + Leptos + WebSocket**

- ✅ Живые данные от Binance WebSocket
- ✅ WebGPU рендеринг (60 FPS)
- ✅ Скользящие средние: SMA20, EMA12
- ✅ Сплошная линия текущей цены
- ✅ Интерактивный tooltip
- ✅ Профессиональный вид (как TradingView)
Подробнее об оптимизациях смотрите в [PERFORMANCE.md](./PERFORMANCE.md).

## 🗂️ Файловая структура

```
src/
├── app.rs                  # Leptos UI компоненты + реактивность
├── lib.rs                  # WASM exports (hydrate, main)
├── candle_shader.wgsl      # WebGPU шейдеры для свечей
├── domain/
│   ├── chart/
│   │   ├── entities.rs     # Chart, ChartData
│   │   └── value_objects.rs # ChartType, Viewport
│   ├── market_data/
│   │   ├── entities.rs     # Candle, CandleSeries
│   │   ├── value_objects.rs # OHLCV, Price, Volume, Symbol
│   │   └── services.rs     # Validation, data operations
│   ├── logging.rs          # Logger abstractions
│   └── errors.rs           # AppError (simplified)
└── infrastructure/
    ├── websocket/
    │   ├── binance_client.rs # WebSocket клиент Binance
    │   └── dto.rs           # JSON DTO structures
    ├── rendering/
    │   ├── renderer/          # WebGPU рендерер по частям
    │   └── gpu_structures.rs  # GPU vertex structures
    └── mod.rs               # Infrastructure services
```

## ⚡ Поток данных

```
Binance WebSocket → BinanceClient → Leptos Signals → WebGPU → Canvas
                                          ↓
                                    Tooltip + UI Updates
```

## 🧩 Ключевые компоненты

### **app.rs - Leptos Frontend**
- `App()` - главный компонент с CSS
- `Header()` - цена, количество свечей, статус
- `ChartContainer()` - WebGPU рендеринг + mouse events
- `ChartTooltip()` - интерактивный tooltip

### **renderer** - GPU рендеринг
- Рендеринг свечей (зеленые/красные)
- Скользящие средние (SMA20, EMA12)
- Сплошная линия цены (желтая)
- 300-свечной скроллинг буфер

### **binance_client.rs - WebSocket**
- Подключение к `wss://stream.binance.com`
- Парсинг kline events
- Обновление Leptos сигналов

## 📡 Глобальные сигналы

```rust
GLOBAL_CURRENT_PRICE: f64    // Текущая цена BTC
GLOBAL_CANDLE_COUNT: usize   // Количество свечей
GLOBAL_IS_STREAMING: bool    // WebSocket статус
TOOLTIP_DATA: TooltipData    // Данные tooltip
GLOBAL_LOGS: Vec<String>     // Debug логи
```

## 🎨 Визуальные элементы

- **Свечи**: Зеленые (рост) / Красные (падение)
- **SMA20**: Красная линия (простое среднее 20 периодов) 
- **EMA12**: Фиолетовая линия (экспоненциальное среднее 12 периодов)
- **Цена**: Сплошная желтая линия + оранжевый лейбл
- **Tooltip**: Черный с OHLC + Volume + % change

## 🔧 Технические детали

**WebGPU Pipeline:**
- Вершинный буфер: 100k вершин
- Шейдеры: `candle_shader.wgsl`
- Координаты: NDC [-1, 1]
- Цвета: через uniform buffer

**WebSocket:**
- Interval: 1m candles
- Symbol: BTCUSDT
- Auto-reconnect с экспоненциальной задержкой (см. [реализацию](src/infrastructure/websocket/binance_client.rs#L146-L223))

**Leptos:**
- SSR отключен (client-only)
- Реактивные updates
- CSS встроенный

## 📦 Сборка

```bash
# Установите wasm32 таргет один раз
rustup target add wasm32-unknown-unknown

# Development
cargo build --target wasm32-unknown-unknown

# Release
wasm-pack build --target web --release

# Serve
python -m http.server 8080
```

## 🎯 Статус проекта

**Готово:**
- Real-time торговый график ✅
- Технические индикаторы ✅  
- Профессиональный UI ✅
- WebGPU производительность ✅

**Архитектура:** Простая, чистая, работающая 🚀 