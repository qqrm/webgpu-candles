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
    infrastructure::rendering::WebGpuRenderer,
};

// Глобальное состояние для простого графика
thread_local! {
    static SIMPLE_CHART_DATA: RefCell<Option<Vec<Candle>>> = RefCell::new(None);
}

/// WebGPU WASM API для рендеринга графиков
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
            canvas_id: canvas_id.clone(),
            chart_width: 800,
            chart_height: 500,
            renderer: Some(WebGpuRenderer::new(canvas_id, 800, 500)),
        }
    }

    /// Инициализировать chart с тестовыми данными
    #[wasm_bindgen(js_name = initializeUnifiedChart)]
    pub fn initialize_unified_chart(
        &mut self,
        _symbol: String,
        _interval: String,
        historical_limit: Option<usize>,
        width: Option<u32>,
        height: Option<u32>,
    ) -> Promise {
        // Обновляем размеры если переданы
        if let Some(w) = width { self.chart_width = w; }
        if let Some(h) = height { self.chart_height = h; }
        
        let limit = historical_limit.unwrap_or(100);

        future_to_promise(async move {
            log_simple(&format!("🚀 WebGPU: Generating {} test candles", limit));

            // Генерируем тестовые данные
            let test_candles = generate_test_candles(limit);
            
            SIMPLE_CHART_DATA.with(|data| {
                *data.borrow_mut() = Some(test_candles);
            });

            log_simple(&format!("✅ WebGPU: Generated {} test candles successfully", limit));

            Ok(JsValue::from_str(&format!(
                "webgpu_chart_ready:{}:true",
                limit
            )))
        })
    }

    /// Рендерить график через WebGPU
    #[wasm_bindgen(js_name = renderUnifiedChart)]
    pub fn render_unified_chart(&mut self) -> Result<JsValue, JsValue> {
        SIMPLE_CHART_DATA.with(|data| {
            if let Some(candles) = data.borrow().as_ref() {
                log_simple(&format!("🎨 WebGPU: Rendering {} candles", candles.len()));

                // Инициализируем WebGPU рендерер если нужно
                if let Some(ref mut renderer) = self.renderer {
                    renderer.set_dimensions(self.chart_width, self.chart_height);
                    
                    // Создаем простой Chart объект для рендеринга
                    use crate::domain::{
                        chart::{Chart, value_objects::ChartType},
                        market_data::{entities::CandleSeries, value_objects::Symbol},
                    };
                    
                    let symbol = Symbol::from("BTCUSDT");
                    let mut candle_series = CandleSeries::new(1000); // Max 1000 candles
                    
                    // Добавляем данные
                    for candle in candles {
                        candle_series.add_candle(candle.clone());
                    }
                    
                    let mut chart = Chart::new(
                        format!("webgpu-chart-{}", symbol.value()),
                        ChartType::Candlestick,
                        1000
                    );
                    chart.data = candle_series;
                    
                    // Рендерим через WebGPU
                    match renderer.render_chart_parallel(&chart) {
                        Ok(_) => {
                            log_simple("✅ WebGPU rendering successful");
                            Ok(JsValue::from_str("webgpu_chart_rendered"))
                        },
                        Err(e) => {
                            log_simple(&format!("❌ WebGPU rendering failed: {:?}", e));
                            Err(e)
                        }
                    }
                } else {
                    let error_msg = "WebGPU renderer not initialized";
                    log_simple(error_msg);
                    Err(JsValue::from_str(error_msg))
                }
                
            } else {
                let error_msg = "No chart data available";
                log_simple(error_msg);
                Err(JsValue::from_str(error_msg))
            }
        })
    }

    /// Получить статистику данных
    #[wasm_bindgen(js_name = getUnifiedStats)]
    pub fn get_unified_stats(&self) -> String {
        SIMPLE_CHART_DATA.with(|data| {
            if let Some(candles) = data.borrow().as_ref() {
                format!(
                    "{{\"totalCandles\":{},\"hasData\":true,\"isStreaming\":false,\"width\":{},\"height\":{},\"backend\":\"WebGPU\"}}",
                    candles.len(),
                    self.chart_width,
                    self.chart_height
                )
            } else {
                format!(
                    "{{\"totalCandles\":0,\"hasData\":false,\"isStreaming\":false,\"width\":{},\"height\":{},\"backend\":\"WebGPU\"}}",
                    self.chart_width,
                    self.chart_height
                )
            }
        })
    }

    /// Остановить поток данных
    #[wasm_bindgen(js_name = stopUnifiedStream)]
    pub fn stop_unified_stream(&self) -> Promise {
        future_to_promise(async move {
            log_simple("🛑 WebGPU: Stopping stream");
            Ok(JsValue::from_str("webgpu_stream_stopped"))
        })
    }

    /// Обработка зума через WebGPU
    #[wasm_bindgen(js_name = handleUnifiedZoom)]
    pub fn handle_unified_zoom(&self, delta: f32, center_x: f32, _center_y: f32) -> Result<(), JsValue> {
        log_simple(&format!("🔍 WebGPU: Zoom event delta={:.1} at x={:.1}", delta, center_x));
        Ok(())
    }

    /// Инициализация WebGPU асинхронно
    #[wasm_bindgen(js_name = initializeWebGPU)]
    pub fn initialize_webgpu(&mut self) -> Promise {
        let _canvas_id = self.canvas_id.clone();
        let _width = self.chart_width;
        let _height = self.chart_height;
        
        future_to_promise(async move {
            log_simple("🚀 Initializing WebGPU...");
            
            // Проверяем поддержку WebGPU
            let supported = WebGpuRenderer::is_webgpu_supported().await;
            if !supported {
                let error_msg = "WebGPU not supported in this browser";
                log_simple(error_msg);
                return Err(JsValue::from_str(error_msg));
            }
            
            log_simple("✅ WebGPU supported, initialization complete");
            Ok(JsValue::from_str("webgpu_initialized"))
        })
    }
}

/// Генерация тестовых данных свечей
fn generate_test_candles(count: usize) -> Vec<Candle> {
    use crate::domain::market_data::{
        entities::OHLCV,
        value_objects::{Price, Volume, Timestamp},
    };

    let mut candles = Vec::new();
    let mut current_price = 100000.0; // Начальная цена BTC
    let base_time = 1700000000; // Базовое время

    for i in 0..count {
        let open = current_price;
        let change = (rand() - 0.5) * 2000.0; // Случайное изменение ±1000
        let close = open + change;
        
        let high = open.max(close) + rand() * 500.0;
        let low = open.min(close) - rand() * 500.0;
        let volume = 50.0 + rand() * 100.0;

        let ohlcv = OHLCV::new(
            Price::new(open as f32),
            Price::new(high as f32),
            Price::new(low as f32),
            Price::new(close as f32),
            Volume::new(volume as f32),
        );

        let candle = Candle::new(
            Timestamp::new((base_time + i as i64 * 60) as u64),
            ohlcv,
        );

        candles.push(candle);
        current_price = close;
    }

    candles
}

/// Простая функция для генерации случайных чисел
fn rand() -> f64 {
    use js_sys::Math;
    #[allow(unused_unsafe)]
    unsafe { Math::random() }
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