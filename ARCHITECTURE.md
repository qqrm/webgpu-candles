# 🚀 Simplified Rust WASM Architecture - Real-time Bitcoin Chart

## 📋 Текущая структура - Leptos + WebGPU + WebSocket

```
src/
├── app.rs                   # Leptos App с реактивными компонентами
├── lib.rs                   # Leptos инициализация
├── candle_shader.wgsl       # WebGPU шейдеры
├── domain/                  # Упрощенный домен
│   ├── chart/              
│   │   ├── entities.rs     # Chart, ChartData
│   │   └── value_objects.rs # Viewport, Color
│   ├── market_data/        
│   │   ├── entities.rs     # Candle, CandleSeries
│   │   ├── value_objects.rs # OHLCV, Price, Volume
│   │   └── services.rs     # CandleDataService, ValidationService
│   ├── logging.rs          # Logger trait
│   └── errors.rs           # DomainError
├── infrastructure/         
│   ├── websocket/          # WebSocket для реального времени
│   │   ├── binance_client.rs
│   │   ├── binance_http_client.rs
│   │   └── dto.rs
│   ├── rendering/          # WebGPU рендеринг
│   │   ├── webgpu_renderer.rs
│   │   └── gpu_structures.rs
│   ├── mod.rs              # ConsoleLogger, LeptosLogger
│   └── http.rs
└── presentation/           
    └── mod.rs              # Экспорты
```

## 🗑️ Упрощение (55% меньше кода)

**Удалили:**
- `repositories.rs` - Repository Pattern без реализаций
- `events.rs` - Event System без использования  
- `chart/services.rs` - Неиспользуемые Domain Services
- `application/use_cases/` - Сложные Use Cases
- `unified_wasm_api.rs` - Заменен на Leptos

**Результат:** 34 → 25 файлов, domain код 18KB → 8KB

## 🆕 Leptos - Pure Rust Frontend

**Реактивные глобальные сигналы:**
```rust
GLOBAL_CURRENT_PRICE   // Текущая цена BTC
GLOBAL_CANDLE_COUNT    // Количество свечей
GLOBAL_IS_STREAMING    // Статус WebSocket
GLOBAL_LOGS           // Логи для debug консоли
```

**Компоненты:**
- `Header` - статистика в реальном времени
- `ChartContainer` - WebGPU рендеринг
- `DebugConsole` - логи с паузой

## 🌊 WebSocket Integration

WebSocket клиент получает данные от Binance и обновляет Leptos сигналы → UI автоматически обновляется

```
Binance WebSocket → BinanceClient → GLOBAL_SIGNALS → Leptos UI → WebGPU
```

## 🏛️ Принципы архитектуры

1. **Простота** - убрали все лишнее, только нужные абстракции
2. **Реактивность** - Leptos сигналы для автообновлений  
3. **Производительность** - WebGPU на GPU, WebSocket в реальном времени
4. **Pure Rust** - никакого JavaScript, все на Rust

## ⚡ Текущие возможности

- [x] WebSocket подключение к Binance (реальное время)
- [x] WebGPU рендеринг свечей (GPU ускорение) 
- [x] Leptos UI с автообновлениями
- [x] Debug консоль с логами
- [x] Статистика: цена, количество свечей, статус WebSocket

---

**Упрощенная архитектура: реальное время + WebGPU + Pure Rust frontend** 🔥 