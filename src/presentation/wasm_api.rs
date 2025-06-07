use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use js_sys::Promise;
use wasm_bindgen_futures::future_to_promise;

// CLEAN PRESENTATION LAYER - только мост к application
use crate::application::{
    ChartApplicationService,
    coordinator::{GLOBAL_COORDINATOR, initialize_global_coordinator},
    RenderChartUseCase,
};
use crate::domain::{
    market_data::{Symbol, TimeInterval},
    market_data::entities::Candle,
    logging::{LogComponent, get_logger},
};

/// **CLEAN PRESENTATION LAYER** - Тонкий мост к application слою
/// Минимальная логика согласно DDD принципам
#[wasm_bindgen]
pub struct PriceChartApi {
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
        // Initialize infrastructure services first
        crate::infrastructure::services::initialize_infrastructure_services();
        
        get_logger().info(
            LogComponent::Presentation("WASM_API"),
            "Creating new PriceChartApi instance"
        );

        Self {
            canvas_id,
            is_initialized: false,
            chart_width: 800,
            chart_height: 400,
        }
    }

    /// **CLEAN** Инициализировать чарт через application layer
    #[wasm_bindgen(js_name = initializeChart)]
    pub fn initialize_chart(&mut self, width: u32, height: u32) -> Promise {
        self.chart_width = width;
        self.chart_height = height;
        
        let canvas_id = self.canvas_id.clone();
        
        future_to_promise(async move {
            get_logger().info(
                LogComponent::Presentation("WASM_API"),
                "🚀 Initializing chart via Application Layer..."
            );
            
            // Делегируем инициализацию в Application Layer
            initialize_global_coordinator(canvas_id, width, height);
            
            // Асинхронная инициализация WebGPU через координатор
            GLOBAL_COORDINATOR.with(|global| {
                if let Some(coordinator) = global.borrow_mut().as_mut() {
                    let init_future = coordinator.initialize_webgpu_renderer();
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Err(e) = init_future.await {
                            get_logger().error(
                                LogComponent::Presentation("WASM_API"),
                                &format!("WebGPU initialization failed: {:?}", e)
                            );
                        }
                    });
                }
            });

            get_logger().info(
                LogComponent::Presentation("WASM_API"),
                "✅ Chart initialized via Application Layer"
            );

            Ok(JsValue::from_str("chart_initialized"))
        })
    }

    /// **CLEAN** Загрузить данные через application layer
    #[wasm_bindgen(js_name = loadHistoricalData)]
    pub fn load_historical_data(
        &mut self,
        symbol: String,
        interval: String,
        limit: Option<usize>,
    ) -> Promise {
        future_to_promise(async move {
            get_logger().info(
                LogComponent::Presentation("WASM_API"),
                &format!("Loading data for {}-{} via Application Layer", symbol, interval)
            );

            // Парсим параметры через Domain Layer
            let symbol = Symbol::from(symbol.as_str());
            let interval = match interval.as_str() {
                "1m" => TimeInterval::OneMinute,
                "5m" => TimeInterval::FiveMinutes,
                "15m" => TimeInterval::FifteenMinutes,
                "1h" => TimeInterval::OneHour,
                "1d" => TimeInterval::OneDay,
                _ => {
                    return Err(JsValue::from_str(&format!("Invalid interval: {}", interval)));
                }
            };

            // Делегируем загрузку данных в Application Layer
            let mut chart_service = ChartApplicationService::new("main_chart".to_string());
            
            match chart_service
                .initialize_with_unified_stream(symbol, interval, limit.unwrap_or(300))
                .await
            {
                Ok(_) => {
                    let stats = chart_service.get_data_stats();
                    
                    // Передаем данные в координатор
                    GLOBAL_COORDINATOR.with(|global| {
                        if let Some(coordinator) = global.borrow_mut().as_mut() {
                            let chart = chart_service.get_chart();
                            let chart_guard = chart.lock().unwrap();
                            coordinator.set_chart(chart_guard.clone());
                        }
                    });

                    get_logger().info(
                        LogComponent::Presentation("WASM_API"),
                        &format!("✅ Data loaded: {} candles", stats.total_candles)
                    );

                    Ok(JsValue::from_str(&format!("data_loaded:{}", stats.total_candles)))
                }
                Err(e) => {
                    get_logger().error(
                        LogComponent::Presentation("WASM_API"),
                        &format!("Data loading failed: {:?}", e)
                    );
                    Err(JsValue::from_str("Data loading failed"))
                }
            }
        })
    }

    /// **CLEAN** Рендеринг через Application layer (убрали всю логику!)
    #[wasm_bindgen(js_name = renderChart)]
    pub fn render_chart(&self) -> Result<JsValue, JsValue> {
        get_logger().info(
            LogComponent::Presentation("WASM_API"),
            "Chart rendering requested via presentation layer"
        );

        // Только делегация в Application Layer!
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

    /// **CLEAN** Получить статистику через application layer
    #[wasm_bindgen(js_name = getChartStats)]
    pub fn get_chart_stats(&self) -> String {
        GLOBAL_COORDINATOR.with(|global| {
            if let Some(coordinator) = global.borrow().as_ref() {
                if let Some(chart) = coordinator.get_chart() {
                    format!(
                        "{{\"candleCount\":{},\"isInitialized\":{},\"width\":{},\"height\":{}}}",
                        chart.get_candle_count(),
                        coordinator.is_initialized(),
                        self.chart_width,
                        self.chart_height
                    )
                } else {
                    format!(
                        "{{\"candleCount\":0,\"isInitialized\":{},\"width\":{},\"height\":{}}}",
                        coordinator.is_initialized(),
                        self.chart_width,
                        self.chart_height
                    )
                }
            } else {
                format!(
                    "{{\"candleCount\":0,\"isInitialized\":false,\"width\":{},\"height\":{}}}",
                    self.chart_width,
                    self.chart_height
                )
            }
        })
    }

    /// **CLEAN** Обработка размеров через application layer
    #[wasm_bindgen(js_name = resizeChart)]
    pub fn resize_chart(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        self.chart_width = width;
        self.chart_height = height;

        // Делегируем в Application Layer
        GLOBAL_COORDINATOR.with(|global| {
            if let Some(coordinator) = global.borrow_mut().as_mut() {
                coordinator.resize(width, height);
            }
        });

        get_logger().info(
            LogComponent::Presentation("WASM_API"),
            &format!("Chart resized to {}x{} via Application layer", width, height)
        );

        Ok(())
    }
}

// Вспомогательные функции для тестирования (минимальные)
#[wasm_bindgen]
pub fn get_candles_count() -> usize {
    GLOBAL_COORDINATOR.with(|global| {
        global.borrow()
            .as_ref()
            .and_then(|coordinator| coordinator.get_chart())
            .map(|chart| chart.get_candle_count())
            .unwrap_or(0)
    })
}

#[wasm_bindgen]
pub fn get_performance_info() -> String {
    GLOBAL_COORDINATOR.with(|global| {
        global.borrow()
            .as_ref()
            .map(|coordinator| coordinator.get_performance_info())
            .unwrap_or_else(|| "{\"status\":\"not_initialized\"}".to_string())
    })
}

// Простая функция логирования для отладки
fn log(message: &str) {
    get_logger().info(
        LogComponent::Presentation("WASM_API"),
        message
    );
} 