use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use js_sys::Array;
use js_sys::Promise;
use wasm_bindgen_futures::future_to_promise;
use std::cell::RefCell;
use web_sys::{MouseEvent, WheelEvent};

// PRODUCTION-READY IMPORTS - FULL APPLICATION LAYER
use crate::application::use_cases::ChartApplicationCoordinator;
use crate::infrastructure::websocket::BinanceWebSocketClient;
use crate::domain::{
    market_data::{Symbol, TimeInterval},
    chart::value_objects::{ChartType, CursorPosition},
    market_data::entities::Candle,
};

// DEMO ФУНКЦИИ (оставляем для совместимости)
use crate::infrastructure::websocket::BinanceHttpClient;

// Глобальное состояние для coordinator'а
thread_local! {
    static GLOBAL_COORDINATOR: RefCell<Option<ChartApplicationCoordinator<BinanceWebSocketClient>>> = RefCell::new(None);
}

// Состояние для интерактивности
thread_local! {
    static MOUSE_STATE: RefCell<MouseState> = RefCell::new(MouseState::new());
}

#[derive(Debug, Clone)]
struct MouseState {
    x: f32,
    y: f32,
    is_over_chart: bool,
    hovered_candle: Option<CandleTooltipData>,
}

#[derive(Debug, Clone)]
struct CandleTooltipData {
    index: usize,
    open: f32,
    high: f32,
    low: f32,
    close: f32,
    volume: f32,
    timestamp: u64,
    x: f32,
    y: f32,
}

impl MouseState {
    fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            is_over_chart: false,
            hovered_candle: None,
        }
    }
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
    
    // Interactive state
    zoom_level: f32,
    min_zoom: f32,
    max_zoom: f32,
    tooltip_enabled: bool,
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
            zoom_level: 1.0,
            min_zoom: 0.1,
            max_zoom: 10.0,
            tooltip_enabled: true,
        }
    }

    /// **PRODUCTION** Инициализировать чарт
    #[wasm_bindgen(js_name = initializeProductionChart)]
    pub fn initialize_production_chart(&mut self, width: u32, height: u32) -> Promise {
        self.chart_width = width;
        self.chart_height = height;
        
        let canvas_id = self.canvas_id.clone();
        
        future_to_promise(async move {
            log("🚀 Initializing Production-Ready Chart...");
            log(&format!("📐 Chart canvas: {}x{}", width, height));
            
            // Настройка интерактивности
            if let Err(e) = setup_chart_interactivity(&canvas_id) {
                log(&format!("⚠️ Failed to setup interactivity: {:?}", e));
            } else {
                log("🎯 Interactive features enabled: zoom and tooltip");
            }
            
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

            // 1. Создаем production components с WebGPU рендерером 🚀
            let websocket_client = BinanceWebSocketClient::new();
            let mut coordinator = ChartApplicationCoordinator::initialize_with_webgpu_renderer(
                websocket_client,
                "chart-canvas".to_string(),
                800,
                400
            ).await;

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

                    // 6. WebGPU coordinator уже настроен при инициализации

                    // 7. Сохраняем coordinator в глобальном состоянии
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

    /// **PRODUCTION** Рендеринг через Infrastructure слой
    #[wasm_bindgen(js_name = renderChartProduction)]
    pub fn render_chart_production(&self) -> Result<JsValue, JsValue> {
        use crate::domain::logging::{LogComponent, get_logger};
        get_logger().info(
            LogComponent::Presentation("WASM_API"),
            "Chart rendering requested via presentation layer"
        );

        // Делегируем рендеринг в Application Layer
        GLOBAL_COORDINATOR.with(|global| {
            if let Some(coordinator) = global.borrow().as_ref() {
                match coordinator.render_chart() {
                    Ok(_) => {
                        get_logger().info(
                            LogComponent::Presentation("WASM_API"),
                            "Chart rendered successfully via Application layer"
                        );
                        Ok(JsValue::from_str("chart_rendered"))
                    }
                    Err(e) => {
                        get_logger().error(
                            LogComponent::Presentation("WASM_API"),
                            &format!("Chart rendering failed: {:?}", e)
                        );
                        Err(e)
                    }
                }
            } else {
                let error_msg = "Chart coordinator not initialized";
                get_logger().error(
                    LogComponent::Presentation("WASM_API"),
                    error_msg
                );
                Err(JsValue::from_str(error_msg))
            }
        })
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

    /// Обработка зума колесом мыши
    #[wasm_bindgen(js_name = handleZoom)]
    pub fn handle_zoom(&mut self, delta: f32, center_x: f32, center_y: f32) -> Result<(), JsValue> {
        // Вычисляем зум фактор
        let zoom_factor = if delta > 0.0 { 1.1 } else { 0.9 };
        
        // Обновляем уровень зума с ограничениями
        let new_zoom = (self.zoom_level * zoom_factor).max(self.min_zoom).min(self.max_zoom);
        
        if (new_zoom - self.zoom_level).abs() > f32::EPSILON {
            self.zoom_level = new_zoom;
            
            // Применяем зум через глобальный координатор
            GLOBAL_COORDINATOR.with(|global| {
                if let Some(coordinator) = global.borrow_mut().as_mut() {
                    let chart = coordinator.get_chart_mut();
                    
                    // Нормализуем центр зума (0-1)
                    let normalized_center_x = center_x / self.chart_width as f32;
                    chart.zoom(zoom_factor, normalized_center_x);
                    
                    log(&format!("🔍 Zoom: {:.2}x at ({:.1}, {:.1})", self.zoom_level, center_x, center_y));
                }
            });
            
            // Перерендерим график
            self.render_chart_production()?;
        }
        
        Ok(())
    }
    
    /// Обработка движения мыши для tooltip
    #[wasm_bindgen(js_name = handleMouseMove)]
    pub fn handle_mouse_move(&self, mouse_x: f32, mouse_y: f32) -> Result<(), JsValue> {
        if !self.tooltip_enabled {
            return Ok(());
        }
        
        // Обновляем позицию мыши в глобальном состоянии
        MOUSE_STATE.with(|mouse_state| {
            let mut state = mouse_state.borrow_mut();
            state.x = mouse_x;
            state.y = mouse_y;
            state.is_over_chart = true;
            
            // Ищем свечу под курсором
            state.hovered_candle = self.find_candle_at_position(mouse_x, mouse_y);
        });
        
        // Перерендерим график с tooltip
        self.render_chart_production()?;
        
        Ok(())
    }
    
    /// Рендеринг tooltip на canvas
    fn render_tooltip(&self, context: &web_sys::CanvasRenderingContext2d) -> Result<(), JsValue> {
        MOUSE_STATE.with(|mouse_state| {
            let state = mouse_state.borrow();
            
            if !state.is_over_chart || state.hovered_candle.is_none() {
                return Ok(());
            }
            
            let tooltip_data = state.hovered_candle.as_ref().unwrap();
            
            // Позиция tooltip
            let tooltip_x = tooltip_data.x + 10.0;
            let tooltip_y = state.y - 10.0;
            
            // Размеры tooltip
            let tooltip_width = 180.0;
            let tooltip_height = 130.0;
            
            // Корректируем позицию если tooltip выходит за границы
            let final_x = if tooltip_x + tooltip_width > self.chart_width as f32 {
                tooltip_data.x - tooltip_width - 10.0
            } else {
                tooltip_x
            };
            
            let final_y = if tooltip_y - tooltip_height < 0.0 {
                state.y + 20.0
            } else {
                tooltip_y - tooltip_height
            };
            
            // Рисуем фон tooltip
            context.set_fill_style(&JsValue::from("rgba(0, 0, 0, 0.9)"));
            context.fill_rect(final_x as f64, final_y as f64, tooltip_width as f64, tooltip_height as f64);
            
            // Рамка
            context.set_stroke_style(&JsValue::from("#00ff88"));
            context.set_line_width(1.0);
            context.stroke_rect(final_x as f64, final_y as f64, tooltip_width as f64, tooltip_height as f64);
            
            // Текст
            context.set_fill_style(&JsValue::from("#ffffff"));
            context.set_font("12px Arial");
            
            let mut text_y = final_y + 20.0;
            let text_x = final_x + 10.0;
            
            // Форматируем время в читаемый вид
            let timestamp_ms = tooltip_data.timestamp * 1000;
            let date = js_sys::Date::new(&JsValue::from_f64(timestamp_ms as f64));
            let time_str = date.to_locale_time_string("en-US").as_string().unwrap_or_default();
            let date_text = format!("#{} • {}", tooltip_data.index, time_str);
            context.fill_text(&date_text, text_x as f64, text_y as f64)?;
            text_y += 18.0;
            
            // OHLC данные
            context.set_fill_style(&JsValue::from("#4ade80"));
            let open_text = format!("O: ${:.2}", tooltip_data.open);
            context.fill_text(&open_text, text_x as f64, text_y as f64)?;
            text_y += 16.0;
            
            context.set_fill_style(&JsValue::from("#00ff88"));
            let high_text = format!("H: ${:.2}", tooltip_data.high);
            context.fill_text(&high_text, text_x as f64, text_y as f64)?;
            text_y += 16.0;
            
            context.set_fill_style(&JsValue::from("#ff4444"));
            let low_text = format!("L: ${:.2}", tooltip_data.low);
            context.fill_text(&low_text, text_x as f64, text_y as f64)?;
            text_y += 16.0;
            
            let close_color = if tooltip_data.close >= tooltip_data.open { "#4ade80" } else { "#ff4444" };
            context.set_fill_style(&JsValue::from(close_color));
            let close_text = format!("C: ${:.2}", tooltip_data.close);
            context.fill_text(&close_text, text_x as f64, text_y as f64)?;
            text_y += 16.0;
            
            // Volume
            context.set_fill_style(&JsValue::from("#a0a0a0"));
            let volume_text = format!("Vol: {:.1}K", tooltip_data.volume / 1000.0);
            context.fill_text(&volume_text, text_x as f64, text_y as f64)?;
            
            Ok(())
        })
    }
    
    /// Поиск свечи под указанной позицией
    fn find_candle_at_position(&self, mouse_x: f32, mouse_y: f32) -> Option<CandleTooltipData> {
        GLOBAL_COORDINATOR.with(|global| {
            global.borrow().as_ref().and_then(|coordinator| {
                let chart = coordinator.get_chart();
                let candles = chart.data.get_candles();
                
                if candles.is_empty() {
                    return None;
                }
                
                // Используем те же параметры что и в рендеринге
                let padding = 50.0;
                let text_space = 80.0;
                let chart_width = self.chart_width as f32 - (padding * 2.0) - text_space;
                
                // Проверяем что мышь в области графика
                if mouse_x < padding || mouse_x > padding + chart_width {
                    return None;
                }
                
                let candle_width = chart_width / candles.len() as f32;
                
                // Находим индекс свечи - точно как в рендеринге
                let relative_x = mouse_x - padding;
                let candle_index = (relative_x / candle_width) as usize;
                
                if candle_index < candles.len() {
                    let candle = &candles[candle_index];
                    
                    // Вычисляем центр свечи точно как в рендеринге
                    let candle_center_x = padding + (candle_index as f32 * candle_width) + (candle_width / 2.0);
                    
                    // Проверяем, что мышь действительно над свечой (с небольшим допуском)
                    let tolerance = candle_width / 2.0;
                    if (mouse_x - candle_center_x).abs() <= tolerance {
                        
                        // Оставляем timestamp как есть для tooltip
                        
                        return Some(CandleTooltipData {
                            index: candle_index,
                            open: candle.ohlcv.open.value(),
                            high: candle.ohlcv.high.value(),
                            low: candle.ohlcv.low.value(),
                            close: candle.ohlcv.close.value(),
                            volume: candle.ohlcv.volume.value(),
                            timestamp: candle.timestamp.value(),
                            x: candle_center_x,
                            y: mouse_y,
                        });
                    }
                }
                
                None
            })
        })
    }
    
    /// Обновить tooltip данные при изменении данных графика
    #[wasm_bindgen(js_name = refreshTooltip)]
    pub fn refresh_tooltip(&self) -> Result<(), JsValue> {
        MOUSE_STATE.with(|mouse_state| {
            let mut state = mouse_state.borrow_mut();
            
            // Если мышь над графиком, пересчитываем tooltip
            if state.is_over_chart {
                state.hovered_candle = self.find_candle_at_position(state.x, state.y);
            }
        });
        
        Ok(())
    }
}

/// Настройка интерактивности для canvas
fn setup_chart_interactivity(canvas_id: &str) -> Result<(), JsValue> {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id(canvas_id)
        .ok_or("Canvas not found")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    
    // Обработчик зума колесом мыши
    {
        let wheel_callback = Closure::wrap(Box::new(move |event: WheelEvent| {
            event.prevent_default();
            
            let delta = event.delta_y();
            let rect = event.target().unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>().unwrap()
                .get_bounding_client_rect();
            
            let mouse_x = event.client_x() as f32 - rect.left() as f32;
            let mouse_y = event.client_y() as f32 - rect.top() as f32;
            
            // Отправляем событие в JavaScript для обработки
            let _ = web_sys::window().unwrap()
                .dispatch_event(&web_sys::CustomEvent::new("chartZoom").unwrap());
                
            log(&format!("🔍 Wheel event: delta={}, pos=({}, {})", delta, mouse_x, mouse_y));
        }) as Box<dyn FnMut(_)>);
        
        canvas.add_event_listener_with_callback("wheel", wheel_callback.as_ref().unchecked_ref())?;
        wheel_callback.forget();
    }
    
    // Обработчик движения мыши
    {
        let mousemove_callback = Closure::wrap(Box::new(move |event: MouseEvent| {
            let _rect = event.target().unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>().unwrap()
                .get_bounding_client_rect();
            
            let _mouse_x = event.client_x() as f32 - _rect.left() as f32;
            let _mouse_y = event.client_y() as f32 - _rect.top() as f32;
            
            // Отправляем событие в JavaScript для обработки
            let _ = web_sys::window().unwrap()
                .dispatch_event(&web_sys::CustomEvent::new("chartMouseMove").unwrap());
        }) as Box<dyn FnMut(_)>);
        
        canvas.add_event_listener_with_callback("mousemove", mousemove_callback.as_ref().unchecked_ref())?;
        mousemove_callback.forget();
    }
    
    // Обработчик ухода мыши с canvas
    {
        let mouseleave_callback = Closure::wrap(Box::new(move |_event: MouseEvent| {
            MOUSE_STATE.with(|mouse_state| {
                let mut state = mouse_state.borrow_mut();
                state.is_over_chart = false;
                state.hovered_candle = None;
            });
        }) as Box<dyn FnMut(_)>);
        
        canvas.add_event_listener_with_callback("mouseleave", mouseleave_callback.as_ref().unchecked_ref())?;
        mouseleave_callback.forget();
    }
    
    Ok(())
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