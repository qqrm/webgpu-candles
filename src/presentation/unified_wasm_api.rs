use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use js_sys::Promise;
use std::cell::RefCell;
use gloo::console;

use crate::{
    domain::{
        market_data::entities::Candle,
        logging::{LogComponent, get_logger},
    },
    infrastructure::{
        rendering::WebGpuRenderer,
        websocket::BinanceWebSocketClient,
    },
    application::coordinator::{
        self, initialize_global_coordinator, with_global_coordinator, with_global_coordinator_mut,
    },
    domain::{
        chart::Chart,
        market_data::{entities::CandleSeries, value_objects::Symbol, TimeInterval},
    },
};

// Глобальное состояние для простого графика с WebSocket поддержкой
thread_local! {
    static SIMPLE_CHART_DATA: RefCell<Option<Vec<Candle>>> = RefCell::new(None);
    static CHART_SYMBOL: RefCell<String> = RefCell::new("BTCUSDT".to_string());
    static CHART_INTERVAL: RefCell<String> = RefCell::new("1s".to_string());
    static WEBSOCKET_CLIENT: RefCell<Option<BinanceWebSocketClient>> = RefCell::new(None);
    static IS_STREAMING: RefCell<bool> = RefCell::new(false);
    static LAST_CANDLE_COUNT: RefCell<usize> = RefCell::new(0);
    static GLOBAL_RENDERER: RefCell<Option<WebGpuRenderer>> = RefCell::new(None);
}

/// WebGPU WASM API для рендеринга графиков с WebSocket поддержкой
#[wasm_bindgen]
pub struct UnifiedPriceChartApi {
    canvas_id: String,
    chart_width: u32,
    chart_height: u32,
    renderer: Option<WebGpuRenderer>,
}

#[wasm_bindgen]
impl UnifiedPriceChartApi {
    /// Создать новый WebGPU API
    #[wasm_bindgen(constructor)]
    pub fn new(canvas_id: String) -> Self {
        Self {
            canvas_id,
            chart_width: 800,
            chart_height: 500,
            renderer: None,
        }
    }

    #[wasm_bindgen(js_name = initialize)]
    pub async fn initialize(&mut self) -> Result<(), JsValue> {
        log_simple("🚀 Initializing WebGPU renderer with WebSocket support...");
        self.renderer = Some(WebGpuRenderer::new(&self.canvas_id, self.chart_width, self.chart_height).await?);
        log_simple("✅ WebGPU renderer created successfully");
        
        // Тестовый рендер треугольника сразу после инициализации
        if let Some(ref renderer) = self.renderer {
            log_simple("🔴 Running basic triangle test...");
            renderer.test_basic_triangle()?;
            log_simple("✅ Basic triangle test completed");
        }
        
        Ok(())
    }

    /// Инициализировать chart с WebSocket stream от Binance
    #[wasm_bindgen(js_name = initializeUnifiedChart)]
    pub fn initialize_unified_chart(
        &mut self,
        symbol: String,
        interval: String,
        historical_limit: Option<usize>,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Promise {
        // Обновляем размеры если переданы
        if let Some(w) = width { self.chart_width = w; }
        if let Some(h) = height { self.chart_height = h; }
        
        let limit = historical_limit.unwrap_or(200);

        future_to_promise(async move {
            use crate::infrastructure::http::BinanceHttpClient;
            use crate::domain::market_data::{Symbol, TimeInterval};
            
            log_simple(&format!("🌐 WebSocket: Loading initial {} data and starting stream", symbol));

            // Сохраняем параметры
            CHART_SYMBOL.with(|s| *s.borrow_mut() = symbol.clone());
            CHART_INTERVAL.with(|i| *i.borrow_mut() = interval.clone());

            let btc_symbol = Symbol::from(symbol.as_str());
            let time_interval = match interval.as_str() {
                "1s" => TimeInterval::OneSecond,
                "1m" => TimeInterval::OneMinute,
                "5m" => TimeInterval::FiveMinutes,
                "15m" => TimeInterval::FifteenMinutes,
                "1h" => TimeInterval::OneHour,
                _ => TimeInterval::OneSecond,
            };

            // 1. Загружаем исторические данные через HTTP
            let http_client = BinanceHttpClient::new();
            match http_client.get_recent_candles(&btc_symbol, time_interval, limit).await {
                Ok(historical_candles) => {
                    SIMPLE_CHART_DATA.with(|data| {
                        *data.borrow_mut() = Some(historical_candles.clone());
                    });
                    
                    LAST_CANDLE_COUNT.with(|count| {
                        *count.borrow_mut() = historical_candles.len();
                    });

                    log_simple(&format!("✅ Loaded {} historical candles", historical_candles.len()));

                    // 2. Запускаем WebSocket stream  
                    log_simple("🔍 DEBUG: About to call start_websocket_stream...");
                    Self::start_websocket_stream(symbol.clone(), interval.clone()).await;
                    log_simple("🔍 DEBUG: start_websocket_stream call completed");

                    Ok(JsValue::from_str(&format!(
                        "websocket_chart_ready:{}:streaming",
                        historical_candles.len()
                    )))
                },
                Err(e) => {
                    log_simple(&format!("❌ Failed to load historical data: {:?}", e));
                    Err(JsValue::from_str(&format!("Failed to load historical data: {:?}", e)))
                }
            }
        })
    }

    /// Запуск WebSocket stream
    async fn start_websocket_stream(symbol: String, interval: String) {
        log_simple(&format!("🔌 Starting WebSocket stream for {}@{}", symbol, interval));
        log_simple("🔍 DEBUG: WebSocket function called");

        let btc_symbol = Symbol::from(symbol.as_str());
        let time_interval = match interval.as_str() {
            "1s" => TimeInterval::OneSecond,  // Binance не поддерживает 1s, но попробуем
            "1m" => TimeInterval::OneMinute,
            "5m" => TimeInterval::FiveMinutes,
            "15m" => TimeInterval::FifteenMinutes,
            "1h" => TimeInterval::OneHour,
            _ => TimeInterval::OneMinute, // Fallback to 1m
        };

        // Создаем WebSocket клиент
        log_simple("🔍 DEBUG: Creating WebSocket client...");
        let mut ws_client = BinanceWebSocketClient::new(btc_symbol, time_interval);
        log_simple("🔍 DEBUG: WebSocket client created");
        
        // Сохраняем клиент в глобальном состоянии
        WEBSOCKET_CLIENT.with(|client| {
            *client.borrow_mut() = Some(ws_client.clone());
        });
        log_simple("🔍 DEBUG: WebSocket client saved to global state");

        IS_STREAMING.with(|streaming| {
            *streaming.borrow_mut() = true;
        });
        log_simple("🔍 DEBUG: Streaming flag set to true");

        // Запускаем обработчик в фоне
        log_simple("🔍 DEBUG: Starting spawn_local for WebSocket handler...");
        wasm_bindgen_futures::spawn_local(async move {
            log_simple("🔍 DEBUG: Inside spawn_local - handler starting...");
            let handler = |candle: Candle| {
                log_simple(&format!("📊 WebSocket: Received candle ${:.2}", candle.ohlcv.close.value()));
                
                // Добавляем новую свечу в данные
                let should_render = SIMPLE_CHART_DATA.with(|data| {
                    if let Some(candles) = data.borrow_mut().as_mut() {
                        // Проверяем, новая ли это свеча или обновление существующей
                        let new_timestamp = candle.timestamp.value();
                        let mut data_changed = false;
                        
                        if let Some(last_candle) = candles.last_mut() {
                            if last_candle.timestamp.value() == new_timestamp {
                                // Обновляем последнюю свечу
                                *last_candle = candle;
                                log_simple("🔄 Updated existing candle");
                                data_changed = true;
                            } else if new_timestamp > last_candle.timestamp.value() {
                                // Добавляем новую свечу
                                candles.push(candle);
                                log_simple("✅ Added new candle to stream");
                                data_changed = true;
                                
                                // Ограничиваем до 300 свечей
                                while candles.len() > 300 {
                                    candles.remove(0);
                                }
                            }
                        } else {
                            // Первая свеча
                            candles.push(candle);
                            log_simple("🎉 Added first WebSocket candle");
                            data_changed = true;
                        }
                        
                        // Обновляем счетчик
                        LAST_CANDLE_COUNT.with(|count| {
                            *count.borrow_mut() = candles.len();
                        });
                        
                        data_changed
                    } else {
                        false
                    }
                });
                
                // 🚀 МГНОВЕННАЯ ПЕРЕРИСОВКА прямо в Rust по каждому тику!
                if should_render {
                    log_simple("🚀 WebSocket: Data updated, will render on next cycle");
                }
            };

            // Запуск stream с обработчиком
            log_simple("🔍 DEBUG: About to call ws_client.start_stream()...");
            match ws_client.start_stream(handler).await {
                Ok(_) => {
                    log_simple("✅ WebSocket stream completed successfully");
                },
                Err(e) => {
                    log_simple(&format!("❌ WebSocket stream error: {}", e));
                    IS_STREAMING.with(|streaming| {
                        *streaming.borrow_mut() = false;
                    });
                }
            }
            log_simple("🔍 DEBUG: spawn_local task ending");
        });

        log_simple("✅ WebSocket stream started successfully (spawn_local launched)");
    }

    /// Рендерить график через WebGPU с WebSocket данными
    #[wasm_bindgen(js_name = renderUnifiedChart)]
    pub fn render_unified_chart(&mut self) -> Result<JsValue, JsValue> {
        self.render_chart_internal()
    }

    /// Внутренняя функция рендеринга
    fn render_chart_internal(&mut self) -> Result<JsValue, JsValue> {
        SIMPLE_CHART_DATA.with(|data| {
            if let Some(candles) = data.borrow().as_ref() {
                let current_count = candles.len();
                let is_streaming = IS_STREAMING.with(|s| *s.borrow());
                
                log_simple(&format!("🎨 WebSocket Render: {} candles (streaming: {})", current_count, is_streaming));
                
                if candles.is_empty() {
                    log_simple("⚠️ No candles to render from WebSocket!");
                    return Err(JsValue::from_str("No WebSocket candles to render"));
                }

                // Проверяем WebGPU рендерер
                if let Some(ref mut renderer) = self.renderer {
                    renderer.resize(self.chart_width, self.chart_height);
                    
                    // Создаем Chart объект для рендеринга
                    let symbol = Symbol::from("BTCUSDT");
                    let mut candle_series = CandleSeries::new(1000);
                    
                    // Добавляем данные
                    for candle in candles {
                        candle_series.add_candle(candle.clone());
                    }
                    
                    let mut chart = Chart::new(
                        format!("websocket-chart-{}", symbol.value()),
                        crate::domain::chart::ChartType::Candlestick,
                        1000
                    );
                    chart.data = candle_series;
                    
                    // Показываем последнюю цену
                    if let Some(last_candle) = candles.last() {
                        log_simple(&format!("💰 Current price: ${:.2}", last_candle.ohlcv.close.value()));
                    }
                    
                    // Рендерим через WebGPU
                    match renderer.render(&chart) {
                        Ok(_) => {
                            Ok(JsValue::from_str("websocket_chart_rendered"))
                        },
                        Err(e) => {
                            log_simple(&format!("❌ WebSocket rendering failed: {:?}", e));
                            Err(e)
                        }
                    }
                } else {
                    let error_msg = "❌ WebGPU renderer not initialized!";
                    log_simple(error_msg);
                    Err(JsValue::from_str(error_msg))
                }
                
            } else {
                Err(JsValue::from_str("No WebSocket data available"))
            }
        })
    }

    /// Получить статистику WebSocket данных
    #[wasm_bindgen(js_name = getUnifiedStats)]
    pub fn get_unified_stats(&self) -> String {
        let is_streaming = IS_STREAMING.with(|s| *s.borrow());
        let candle_count = LAST_CANDLE_COUNT.with(|c| *c.borrow());
        
        SIMPLE_CHART_DATA.with(|data| {
            if let Some(candles) = data.borrow().as_ref() {
                let last_timestamp = candles.last().map(|c| c.timestamp.value()).unwrap_or(0);
                let last_price = candles.last().map(|c| c.ohlcv.close.value()).unwrap_or(0.0);
                
                format!(
                    "{{\"totalCandles\":{},\"hasData\":true,\"isStreaming\":{},\"width\":{},\"height\":{},\"backend\":\"WebSocket+WebGPU\",\"lastTimestamp\":{},\"lastPrice\":{:.2},\"streamActive\":{}}}",
                    candles.len(),
                    is_streaming,
                    self.chart_width,
                    self.chart_height,
                    last_timestamp,
                    last_price,
                    is_streaming
                )
            } else {
                format!(
                    "{{\"totalCandles\":{},\"hasData\":false,\"isStreaming\":{},\"width\":{},\"height\":{},\"backend\":\"WebSocket+WebGPU\",\"lastTimestamp\":0,\"lastPrice\":0,\"streamActive\":{}}}",
                    candle_count,
                    is_streaming,
                    self.chart_width,
                    self.chart_height,
                    is_streaming
                )
            }
        })
    }

    /// Остановить WebSocket поток
    #[wasm_bindgen(js_name = stopUnifiedStream)]
    pub fn stop_unified_stream(&self) -> Promise {
        future_to_promise(async move {
            log_simple("🛑 Stopping WebSocket stream...");
            
            IS_STREAMING.with(|streaming| {
                *streaming.borrow_mut() = false;
            });
            
            WEBSOCKET_CLIENT.with(|client| {
                *client.borrow_mut() = None;
            });
            
            log_simple("✅ WebSocket stream stopped");
            Ok(JsValue::from_str("websocket_stream_stopped"))
        })
    }

    /// Проверить статус WebSocket соединения
    #[wasm_bindgen(js_name = getStreamStatus)]
    pub fn get_stream_status(&self) -> String {
        let is_streaming = IS_STREAMING.with(|s| *s.borrow());
        let candle_count = LAST_CANDLE_COUNT.with(|c| *c.borrow());
        let symbol = CHART_SYMBOL.with(|s| s.borrow().clone());
        let interval = CHART_INTERVAL.with(|i| i.borrow().clone());
        
        format!(
            "{{\"streaming\":{},\"candles\":{},\"symbol\":\"{}\",\"interval\":\"{}\"}}",
            is_streaming, candle_count, symbol, interval
        )
    }

    /// Принудительно переподключить WebSocket
    #[wasm_bindgen(js_name = reconnectWebSocket)]
    pub fn reconnect_websocket(&self) -> Promise {
        let symbol = CHART_SYMBOL.with(|s| s.borrow().clone());
        let interval = CHART_INTERVAL.with(|i| i.borrow().clone());
        
        future_to_promise(async move {
            log_simple("🔄 Reconnecting WebSocket stream...");
            
            // Остановить текущий stream
            IS_STREAMING.with(|streaming| {
                *streaming.borrow_mut() = false;
            });
            
            // Запустить заново
            Self::start_websocket_stream(symbol, interval).await;
            
            Ok(JsValue::from_str("websocket_reconnected"))
        })
    }

    /// Инициализировать глобальный renderer (вызывается после создания)
    #[wasm_bindgen(js_name = initGlobalRenderer)]
    pub fn init_global_renderer(&mut self) {
        if let Some(renderer) = self.renderer.take() {
            GLOBAL_RENDERER.with(|global| {
                *global.borrow_mut() = Some(renderer);
            });
            log_simple("✅ Global renderer initialized for immediate WebSocket rendering");
        }
    }

    /// Обработка зума через WebGPU
    #[wasm_bindgen(js_name = handleUnifiedZoom)]
    pub fn handle_unified_zoom(&self, delta: f32, center_x: f32, _center_y: f32) -> Result<(), JsValue> {
        log_simple(&format!("🔍 WebSocket Zoom: delta={:.1} at x={:.1}", delta, center_x));
        Ok(())
    }
}

/// Логирование через gloo
fn log_simple(message: &str) {
    get_logger().info(LogComponent::Presentation("WebGPU_API"), message);
}

/// Экспортируемые функции для совместимости
#[wasm_bindgen(js_name = createUnifiedChart)]
pub fn create_unified_chart(canvas_id: String) -> UnifiedPriceChartApi {
    UnifiedPriceChartApi::new(canvas_id)
}

#[wasm_bindgen(js_name = getUnifiedCanvasStats)]
pub fn get_unified_canvas_stats() -> String {
    SIMPLE_CHART_DATA.with(|data| {
        if let Some(candles) = data.borrow().as_ref() {
            format!("WebGPU Chart: {} candles generated", candles.len())
        } else {
            "WebGPU Chart: No data".to_string()
        }
    })
}

#[wasm_bindgen]
pub struct PriceChartApi;

#[wasm_bindgen]
impl PriceChartApi {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Логирование инициализации
        get_logger().info(
            LogComponent::Presentation("WASM_API"),
            "PriceChartApi created",
        );
        Self
    }

    #[wasm_bindgen(js_name = initialize)]
    pub fn initialize(canvas_id: String, width: u32, height: u32) -> Promise {
        future_to_promise(async move {
            // Передаем владение canvas_id в координатор
            initialize_global_coordinator(canvas_id.clone(), width, height);
            
            match WebGpuRenderer::new(&canvas_id, width, height).await {
                Ok(renderer) => {
                    with_global_coordinator_mut(|coord| coord.initialize_renderer(renderer));
                    Ok(JsValue::from_str("initialized"))
                }
                Err(e) => {
                    get_logger().error(
                        LogComponent::Application("ChartCoordinator"),
                        &format!("⚠️ Failed to initialize WebGPU renderer from API: {:?}", e)
                    );
                    Err(e)
                }
            }
        })
    }

    #[wasm_bindgen(js_name = render)]
    pub fn render() -> Result<(), JsValue> {
        with_global_coordinator(|coord| coord.render_chart())
            .unwrap_or_else(|| Err(JsValue::from_str("Coordinator not found")))
    }

    #[wasm_bindgen(js_name = setCandles)]
    pub fn set_candles(candles: JsValue) -> Result<(), JsValue> {
        let candles: Vec<Candle> = serde_wasm_bindgen::from_value(candles)?;
        let mut chart = Chart::new("main".to_string(), crate::domain::chart::ChartType::Candlestick, 1000);
        chart.set_historical_data(candles);
        
        with_global_coordinator_mut(|coord| coord.set_chart(chart));
        Ok(())
    }
} 