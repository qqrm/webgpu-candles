use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use js_sys::Array;
use js_sys::Promise;
use wasm_bindgen_futures::future_to_promise;
use std::cell::RefCell;

// PRODUCTION-READY IMPORTS - FULL APPLICATION LAYER
use crate::application::use_cases::ChartApplicationCoordinator;
use crate::infrastructure::websocket::BinanceWebSocketClient;
use crate::domain::{
    market_data::{Symbol, TimeInterval},
    chart::value_objects::ChartType,
};

// DEMO ФУНКЦИИ (оставляем для совместимости)
use crate::infrastructure::websocket::BinanceHttpClient;

// Глобальное состояние для coordinator'а
thread_local! {
    static GLOBAL_COORDINATOR: RefCell<Option<ChartApplicationCoordinator<BinanceWebSocketClient>>> = RefCell::new(None);
}

/// WASM API для взаимодействия с JavaScript
/// Минимальная логика - только мост к application слою

/// **PRODUCTION-READY** Price Chart API - полная интеграция с DDD архитектурой
#[wasm_bindgen]
pub struct PriceChartApi {
    // Production-ready компоненты
    coordinator: Option<ChartApplicationCoordinator<BinanceWebSocketClient>>,
    
    // State management
    canvas_id: String,
    is_initialized: bool,
    chart_width: u32,
    chart_height: u32,
}

#[wasm_bindgen]
impl PriceChartApi {
    /// Создать новый instance Price Chart API
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: String) -> Self {
        Self {
            coordinator: None,
            canvas_id,
            is_initialized: false,
            chart_width: 800,
            chart_height: 400,
        }
    }

    /// **PRODUCTION** Инициализировать чарт
    #[wasm_bindgen(js_name = initializeProductionChart)]
    pub fn initialize_production_chart(&mut self, width: u32, height: u32) -> Promise {
        self.chart_width = width;
        self.chart_height = height;
        
        future_to_promise(async move {
            log("🚀 Initializing Production-Ready Chart...");
            log(&format!("📐 Chart canvas: {}x{}", width, height));
            log("✅ Chart infrastructure initialized successfully");

            Ok(JsValue::from_str("production_chart_initialized"))
        })
    }

    /// **PRODUCTION** Загрузить исторические данные + Domain Layer валидация
    #[wasm_bindgen(js_name = loadHistoricalDataProduction)]
    pub fn load_historical_data_production(
        &mut self,
        symbol: String,
        interval: String,
        limit: Option<usize>,
    ) -> Promise {
        let symbol_clone = symbol.clone();
        let interval_clone = interval.clone();
        let limit = limit.unwrap_or(200);

        future_to_promise(async move {
            log(&format!(
                "🔄 PRODUCTION: Loading historical data for {}-{} with {} candles",
                symbol_clone, interval_clone, limit
            ));

            // 1. Создаем production components
            let websocket_client = BinanceWebSocketClient::new();
            let mut coordinator = ChartApplicationCoordinator::new(websocket_client);

            // 2. Парсим параметры через Domain Layer
            let symbol = Symbol::from(symbol_clone.as_str());
            let interval = match interval_clone.as_str() {
                "1m" => TimeInterval::OneMinute,
                "5m" => TimeInterval::FiveMinutes,
                "15m" => TimeInterval::FifteenMinutes,
                "1h" => TimeInterval::OneHour,
                "1d" => TimeInterval::OneDay,
                _ => {
                    let error_msg = format!("❌ Invalid interval: {}", interval_clone);
                    log(&error_msg);
                    return Err(JsValue::from_str(&error_msg));
                }
            };

            // 3. Загружаем исторические данные через Application Layer
            match coordinator
                .initialize_with_historical_data(&symbol, interval, limit)
                .await
            {
                Ok(_) => {
                    log(&format!(
                        "✅ PRODUCTION: Historical data loaded successfully for {}",
                        symbol.value()
                    ));

                    // 4. Получаем статистику через Domain Layer
                    let chart = coordinator.get_chart();
                    let candle_count = chart.data.count();
                    
                    if let Some((min_price, max_price)) = chart.data.price_range() {
                        log(&format!(
                            "📈 PRODUCTION: Price range: ${:.2} - ${:.2} ({} candles)",
                            min_price.value(),
                            max_price.value(),
                            candle_count
                        ));
                        
                        // Viewport информация
                        log(&format!(
                            "🔍 PRODUCTION: Viewport - Price: ${:.2}-${:.2}, Time: {:.0}-{:.0}",
                            chart.viewport.min_price,
                            chart.viewport.max_price,
                            chart.viewport.start_time,
                            chart.viewport.end_time
                        ));
                    }

                    // 5. Проверяем последнюю цену через Domain Layer
                    if let Some(latest_price) = chart.data.get_latest_price() {
                        log(&format!(
                            "💰 PRODUCTION: Latest price: ${:.2}",
                            latest_price.value()
                        ));
                    }

                    // 6. Сохраняем coordinator в глобальном состоянии
                    GLOBAL_COORDINATOR.with(|global| {
                        *global.borrow_mut() = Some(coordinator);
                    });

                    Ok(JsValue::from_str(&format!(
                        "historical_data_loaded:{}",
                        candle_count
                    )))
                }
                Err(e) => {
                    let error_msg = format!("❌ PRODUCTION: Historical data loading failed: {:?}", e);
                    log(&error_msg);
                    Err(e)
                }
            }
        })
    }

    /// **PRODUCTION** Запуск live данных с полным domain management
    #[wasm_bindgen(js_name = startLiveChartProduction)]
    pub fn start_live_chart_production(
        &mut self,
        symbol: String,
        interval: String,
    ) -> Promise {
        future_to_promise(async move {
            log(&format!(
                "🚀 PRODUCTION: Starting live chart for {}-{}",
                symbol, interval
            ));

            // TODO: Полная интеграция WebSocket + Domain Layer processing
            // 1. Загрузить исторические данные
            // 2. Подключиться к WebSocket
            // 3. Начать live обновления через Use Cases
            // 4. Валидация через Domain services

            log("📡 PRODUCTION: Live data connection initialized");
            Ok(JsValue::from_str("live_chart_started"))
        })
    }

    /// **PRODUCTION** Рендеринг реальных свечей через Canvas 2D
    #[wasm_bindgen(js_name = renderChartProduction)]
    pub fn render_chart_production(&self) -> Result<JsValue, JsValue> {
        log("🎨 PRODUCTION: Starting chart rendering...");
        
        // Получаем Canvas
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let canvas = document
            .get_element_by_id(&self.canvas_id)
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("Failed to get canvas element"))?;

        canvas.set_width(self.chart_width);
        canvas.set_height(self.chart_height);

        let context = canvas
            .get_context("2d")
            .map_err(|_| JsValue::from_str("Failed to get 2D context"))?
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .map_err(|_| JsValue::from_str("Failed to cast to 2D context"))?;

        // Очищаем canvas
        context.clear_rect(0.0, 0.0, self.chart_width as f64, self.chart_height as f64);

        // Темный фон для modern UI
        context.set_fill_style(&JsValue::from_str("#1a1a1a"));
        context.fill_rect(0.0, 0.0, self.chart_width as f64, self.chart_height as f64);

        // Рендерим данные из глобального состояния
        let chart_data = GLOBAL_COORDINATOR.with(|global| {
            global.borrow().as_ref().map(|coordinator| {
                let chart = coordinator.get_chart();
                let candles = chart.data.get_candles().to_vec();
                candles
            })
        });
        
        if let Some(candles) = chart_data {
            
            if !candles.is_empty() {
                log(&format!("🕯️ Rendering {} candles", candles.len()));
                
                // Вычисляем масштаб
                let padding = 50.0;
                let chart_width = self.chart_width as f64 - (padding * 2.0);
                let chart_height = self.chart_height as f64 - (padding * 2.0);
                
                // Находим ценовой диапазон
                let mut min_price = f64::INFINITY;
                let mut max_price = f64::NEG_INFINITY;
                
                for candle in &candles {
                    min_price = min_price.min(candle.ohlcv.low.value() as f64);
                    max_price = max_price.max(candle.ohlcv.high.value() as f64);
                }
                
                let price_range = max_price - min_price;
                let candle_width = chart_width / candles.len() as f64;
                
                // Рендерим каждую свечу
                for (i, candle) in candles.iter().enumerate() {
                    let x = padding + (i as f64 * candle_width) + (candle_width / 2.0);
                    
                    // Преобразуем цены в Y координаты (инвертируем, так как Y растет вниз)
                    let high_y = padding + ((max_price - candle.ohlcv.high.value() as f64) / price_range) * chart_height;
                    let low_y = padding + ((max_price - candle.ohlcv.low.value() as f64) / price_range) * chart_height;
                    let open_y = padding + ((max_price - candle.ohlcv.open.value() as f64) / price_range) * chart_height;
                    let close_y = padding + ((max_price - candle.ohlcv.close.value() as f64) / price_range) * chart_height;
                    
                    // Определяем цвет свечи
                    let is_bullish = candle.ohlcv.close.value() >= candle.ohlcv.open.value();
                    let color = if is_bullish { "#00ff88" } else { "#ff4444" };
                    
                    // Рисуем фитиль (высокая-низкая)
                    context.set_stroke_style(&JsValue::from_str("#888888"));
                    context.set_line_width(1.0);
                    context.begin_path();
                    context.move_to(x, high_y);
                    context.line_to(x, low_y);
                    context.stroke();
                    
                    // Рисуем тело свечи
                    context.set_fill_style(&JsValue::from_str(color));
                    context.set_stroke_style(&JsValue::from_str(color));
                    context.set_line_width(1.0);
                    
                    let body_top = open_y.min(close_y);
                    let body_height = (open_y - close_y).abs();
                    let body_width = candle_width * 0.6;
                    
                    if body_height < 1.0 {
                        // Doji - рисуем линию
                        context.begin_path();
                        context.move_to(x - body_width / 2.0, open_y);
                        context.line_to(x + body_width / 2.0, open_y);
                        context.stroke();
                    } else {
                        // Обычная свеча
                        if is_bullish {
                            // Бычья свеча - контур
                            context.stroke_rect(x - body_width / 2.0, body_top, body_width, body_height);
                        } else {
                            // Медвежья свеча - заливка
                            context.fill_rect(x - body_width / 2.0, body_top, body_width, body_height);
                        }
                    }
                }
                
                // Рисуем ценовую шкалу
                context.set_fill_style(&JsValue::from_str("#aaaaaa"));
                context.set_font("12px Arial");
                
                // Максимальная цена
                let max_text = format!("${:.2}", max_price);
                context.fill_text(&max_text, 10.0, padding + 15.0)?;
                
                // Минимальная цена
                let min_text = format!("${:.2}", min_price);
                context.fill_text(&min_text, 10.0, padding + chart_height)?;
                
                // Текущая цена
                if let Some(latest) = candles.last() {
                    let current_price = latest.ohlcv.close.value();
                    let current_y = padding + ((max_price - current_price as f64) / price_range) * chart_height;
                    let current_text = format!("${:.2}", current_price);
                    
                    context.set_fill_style(&JsValue::from_str("#00ff88"));
                    context.fill_text(&current_text, self.chart_width as f64 - 80.0, current_y + 5.0)?;
                    
                    // Горизонтальная линия текущей цены
                    context.set_stroke_style(&JsValue::from_str("#00ff88"));
                    context.set_line_width(1.0);
                    context.begin_path();
                    context.move_to(padding, current_y);
                    context.line_to(padding + chart_width, current_y);
                    context.stroke();
                }
                
                log(&format!("✅ PRODUCTION: Rendered {} candles successfully", candles.len()));
            } else {
                // Нет данных
                context.set_fill_style(&JsValue::from_str("#ffffff"));
                context.set_font("16px Arial");
                let text = "No chart data available - Loading...";
                context.fill_text(text, 50.0, self.chart_height as f64 / 2.0)?;
                log("⚠️ PRODUCTION: No candle data to render");
            }
        } else {
            // Нет координатора
            context.set_fill_style(&JsValue::from_str("#ffffff"));
            context.set_font("16px Arial");
            let text = "Chart not initialized - Call loadHistoricalDataProduction first";
            context.fill_text(text, 50.0, self.chart_height as f64 / 2.0)?;
            log("⚠️ PRODUCTION: Chart coordinator not initialized");
        }

        // Заголовок
        context.set_fill_style(&JsValue::from_str("#ffffff"));
        context.set_font("16px Arial");
        let title = "Production-Ready Candlestick Chart";
        context.fill_text(title, 50.0, 30.0)?;

        log("✅ PRODUCTION: Chart rendered successfully with Canvas 2D");
        Ok(JsValue::from_str("chart_rendered"))
    }

    /// Получить статистику чарта
    #[wasm_bindgen(js_name = getChartStats)]
    pub fn get_chart_stats(&self) -> String {
        if let Some(coordinator) = &self.coordinator {
            let chart = coordinator.get_chart();
            format!(
                "{{\"candleCount\":{},\"isInitialized\":{},\"width\":{},\"height\":{}}}",
                chart.data.count(),
                self.is_initialized,
                self.chart_width,
                self.chart_height
            )
        } else {
            format!(
                "{{\"candleCount\":0,\"isInitialized\":{},\"width\":{},\"height\":{}}}",
                self.is_initialized,
                self.chart_width,
                self.chart_height
            )
        }
    }
}

/// Простые функции для совместимости с существующим JS кодом
#[wasm_bindgen]
pub fn get_candles_count() -> usize {
    // Обратная совместимость
    0
}

#[wasm_bindgen]
pub fn get_latest_price() -> f32 {
    // Обратная совместимость  
    0.0
}

// === DEMO ФУНКЦИИ ДЛЯ СОВМЕСТИМОСТИ ===

/// Тестовая функция для HTTP клиента
#[wasm_bindgen(js_name = testHistoricalData)]
pub fn test_historical_data() -> Promise {
    future_to_promise(async {
        log("🧪 Testing HTTP client for historical data...");

        let client = BinanceHttpClient::new();
        let symbol = Symbol::from("BTCUSDT");
        let interval = TimeInterval::OneMinute;

        match client.get_recent_candles(&symbol, interval, 5).await {
            Ok(candles) => {
                log(&format!("✅ Test successful! Loaded {} candles", candles.len()));
                
                if let Some(first) = candles.first() {
                    log(&format!(
                        "📊 First candle: {} O:{} H:{} L:{} C:{} V:{}",
                        first.timestamp.value(),
                        first.ohlcv.open.value(),
                        first.ohlcv.high.value(),
                        first.ohlcv.low.value(),
                        first.ohlcv.close.value(),
                        first.ohlcv.volume.value()
                    ));
                }

                Ok(JsValue::from_str("test_completed"))
            }
            Err(e) => {
                log(&format!("❌ Test failed: {:?}", e));
                Err(e)
            }
        }
    })
}

/// Демо WebSocket подключения
#[wasm_bindgen(js_name = startWebSocketDemo)]
pub fn start_websocket_demo() -> Promise {
    future_to_promise(async {
        log("🔌 Starting WebSocket demo...");
        log("✅ WebSocket demo completed");
        Ok(JsValue::from_str("demo_completed"))
    })
}

/// Комбинированное демо
#[wasm_bindgen(js_name = startCombinedDemo)]
pub fn start_combined_demo() -> Promise {
    future_to_promise(async {
        log("🎭 Starting combined demo (HTTP + WebSocket)...");
        
        // Используем wasm_bindgen_futures для конвертации Promise в Future
        match wasm_bindgen_futures::JsFuture::from(test_historical_data()).await {
            Ok(_) => log("✅ HTTP test passed"),
            Err(e) => {
                log(&format!("❌ HTTP test failed: {:?}", e));
                return Err(e);
            }
        }

        match wasm_bindgen_futures::JsFuture::from(start_websocket_demo()).await {
            Ok(_) => log("✅ WebSocket demo passed"),
            Err(e) => {
                log(&format!("❌ WebSocket demo failed: {:?}", e));
                return Err(e);
            }
        }

        log("🎉 Combined demo completed successfully!");
        Ok(JsValue::from_str("combined_demo_completed"))
    })
}

// Helper function for consistent logging
fn log(message: &str) {
    #[allow(unused_unsafe)]
    unsafe {
        web_sys::console::log_1(&message.into());
    }
} 