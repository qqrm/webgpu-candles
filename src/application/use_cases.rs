use crate::domain::market_data::{
    repositories::MarketDataRepository, Symbol, TimeInterval, 
    services::{MarketAnalysisService, DataValidationService},
    Candle, entities::CandleSeries
};
use crate::domain::chart::{Chart, services::ChartRenderingService, value_objects::ChartType};
use crate::infrastructure::websocket::BinanceHttpClient;
use wasm_bindgen::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;

/// Helper function for logging
fn log(s: &str) {
    #[allow(unused_unsafe)]
    unsafe {
        web_sys::console::log_1(&s.into());
    }
}

/// Use Case для подключения к потоку рыночных данных
pub struct ConnectToMarketDataUseCase<T> {
    repository: T,
    validation_service: DataValidationService,
}

impl<T> ConnectToMarketDataUseCase<T> {
    pub fn new(repository: T) -> Self {
        Self {
            repository,
            validation_service: DataValidationService::new(),
        }
    }

    pub fn get_repository(&self) -> &T {
        &self.repository
    }

    pub fn get_repository_mut(&mut self) -> &mut T {
        &mut self.repository
    }
}

/// Use Case для анализа рыночных данных
pub struct AnalyzeMarketDataUseCase {
    analysis_service: MarketAnalysisService,
}

impl AnalyzeMarketDataUseCase {
    pub fn new() -> Self {
        Self {
            analysis_service: MarketAnalysisService::new(),
        }
    }

    pub fn process_candle(&self, candle: Candle, chart: &mut Chart) -> Result<(), JsValue> {
        // Domain validation через analysis service
        if self.analysis_service.validate_candle(&candle) {
            log("✅ Candle validation passed, adding to chart...");
            chart.add_candle(candle);
            
            // Log chart state update
            log(&format!(
                "📊 ChartState updated: Total candles: {}, Latest: {}",
                chart.data.count(),
                if let Some(latest) = chart.data.get_candles().last() {
                    format!("O:{} H:{} L:{} C:{} V:{}",
                        latest.ohlcv.open.value(),
                        latest.ohlcv.high.value(),
                        latest.ohlcv.low.value(),
                        latest.ohlcv.close.value(),
                        latest.ohlcv.volume.value()
                    )
                } else {
                    "No candles".to_string()
                }
            ));
            
            Ok(())
        } else {
            let error_msg = "❌ Candle validation failed";
            log(error_msg);
            Err(JsValue::from_str(error_msg))
        }
    }
}

/// Use Case для рендеринга графика - чистый WebGPU! 🚀
pub struct RenderChartUseCase {
    webgpu_renderer: Option<crate::infrastructure::rendering::WebGpuRenderer>,
    webgpu_supported: bool,
}

impl RenderChartUseCase {
    pub fn new() -> Self {
        Self {
            webgpu_renderer: None,
            webgpu_supported: false,
        }
    }

    /// Асинхронная инициализация с WebGPU рендерером 🚀
    pub async fn initialize_webgpu_renderer(canvas_id: String, width: u32, height: u32) -> Self {
        use crate::domain::logging::{LogComponent, get_logger};
        
        // Проверяем поддержку WebGPU
        let webgpu_supported = crate::infrastructure::rendering::WebGpuRenderer::is_webgpu_supported().await;
        
        get_logger().info(
            LogComponent::Application("RenderUseCase"),
            &format!("🔍 WebGPU supported: {}", webgpu_supported)
        );

        let mut renderer = Self {
            webgpu_renderer: None,
            webgpu_supported,
        };

        // Инициализируем WebGPU рендерер
        if webgpu_supported {
            let mut webgpu_renderer = crate::infrastructure::rendering::WebGpuRenderer::new(
                canvas_id, width, height
            );
            
            match webgpu_renderer.initialize().await {
                Ok(_) => {
                    get_logger().info(
                        LogComponent::Application("RenderUseCase"),
                        "🚀 WebGPU renderer initialized successfully"
                    );
                    renderer.webgpu_renderer = Some(webgpu_renderer);
                }
                Err(e) => {
                    get_logger().error(
                        LogComponent::Application("RenderUseCase"),
                        &format!("❌ WebGPU initialization failed: {:?}", e)
                    );
                    renderer.webgpu_supported = false;
                }
            }
        } else {
            get_logger().error(
                LogComponent::Application("RenderUseCase"),
                "❌ WebGPU not supported in this browser"
            );
        }

        renderer
    }

    pub fn set_webgpu_renderer(&mut self, renderer: crate::infrastructure::rendering::WebGpuRenderer) {
        self.webgpu_renderer = Some(renderer);
        self.webgpu_supported = true;
    }

    pub fn prepare_chart_for_rendering(&self, chart: &Chart) -> Result<(), JsValue> {
        // Здесь может быть логика подготовки данных для рендеринга
        // Например, вычисление индикаторов, фильтрация данных и т.д.
        log(&format!("🎨 Chart prepared for rendering: {} candles", chart.data.count()));
        Ok(())
    }

    /// 🚀 Чистый WebGPU рендеринг для максимальной производительности
    pub fn render_chart(&self, chart: &Chart) -> Result<(), JsValue> {
        use crate::domain::logging::{LogComponent, get_logger};
        
        let candle_count = chart.data.count();
        
        // WebGPU-only рендеринг
        if let Some(webgpu_renderer) = &self.webgpu_renderer {
            get_logger().info(
                LogComponent::Application("RenderUseCase"),
                &format!("🚀 WebGPU rendering {} candles (GPU parallel)", candle_count)
            );
            return webgpu_renderer.render_chart_parallel(chart);
        }
        
        let error_msg = "WebGPU renderer not configured or not supported";
        get_logger().error(
            LogComponent::Application("RenderUseCase"),
            error_msg
        );
        Err(JsValue::from_str(error_msg))
    }

    /// Получить информацию о WebGPU рендерере
    pub fn get_renderer_info(&self) -> String {
        let webgpu_available = self.webgpu_renderer.is_some();
        
        format!(
            "{{\"backend\":\"WebGPU\",\"available\":{},\"supported\":{},\"gpu_parallel\":true}}",
            webgpu_available,
            self.webgpu_supported
        )
    }

    /// Проверить готовность WebGPU рендерера
    pub fn is_webgpu_ready(&self) -> bool {
        self.webgpu_supported && self.webgpu_renderer.is_some()
    }
}

/// **NEW** Use Case для загрузки исторических данных
pub struct LoadHistoricalDataUseCase {
    http_client: BinanceHttpClient,
    validation_service: DataValidationService,
}

impl LoadHistoricalDataUseCase {
    pub fn new() -> Self {
        Self {
            http_client: BinanceHttpClient::new(),
            validation_service: DataValidationService::new(),
        }
    }

    pub fn with_testnet() -> Self {
        Self {
            http_client: BinanceHttpClient::with_testnet(),
            validation_service: DataValidationService::new(),
        }
    }

    /// Загрузить исторические данные и преобразовать в CandleSeries
    pub async fn load_historical_candles(
        &self,
        symbol: &Symbol,
        interval: TimeInterval,
        limit: usize,
    ) -> Result<CandleSeries, JsValue> {
        log(&format!(
            "📡 Use Case: Loading historical data for {} with {} interval, limit: {}",
            symbol.value(),
            interval.to_binance_str(),
            limit
        ));

        // Получаем данные через HTTP
        let candles = self.http_client
            .get_recent_candles(symbol, interval, limit)
            .await?;

        log(&format!("📊 Use Case: Received {} historical candles", candles.len()));

        // Создаем CandleSeries и валидируем через Domain Layer
        let mut candle_series = CandleSeries::new(limit + 100); // Запас для live данных
        
        for (i, candle) in candles.into_iter().enumerate() {
            // Domain валидация каждой свечи через ValidationService
            match self.validation_service.validate_candle(&candle) {
                Ok(_) => {
                    candle_series.add_candle(candle);
                }
                Err(e) => {
                    log(&format!("⚠️ Use Case: Invalid candle at index {}: {}, skipping", i, e));
                }
            }
        }

        log(&format!(
            "✅ Use Case: Successfully created CandleSeries with {} validated candles",
            candle_series.count()
        ));

        Ok(candle_series)
    }

    /// Загрузить исторические данные и добавить их в Chart
    pub async fn load_and_populate_chart(
        &self,
        chart: &mut Chart,
        symbol: &Symbol,
        interval: TimeInterval,
        limit: usize,
    ) -> Result<(), JsValue> {
        log(&format!(
            "🔄 Use Case: Loading historical data into chart for {}",
            symbol.value()
        ));

        let candle_series = self.load_historical_candles(symbol, interval, limit).await?;
        
        // Заменяем данные в chart на исторические
        chart.data = candle_series;
        
        // Обновляем viewport на основе новых данных
        chart.update_viewport_for_data();
        
        log(&format!(
            "📈 Use Case: Chart populated with {} historical candles",
            chart.data.count()
        ));

        Ok(())
    }
}

/// Главный координатор всех Use Cases
pub struct ChartApplicationCoordinator<T> {
    connect_use_case: ConnectToMarketDataUseCase<T>,
    analyze_use_case: AnalyzeMarketDataUseCase,
    render_use_case: RenderChartUseCase,
    historical_use_case: LoadHistoricalDataUseCase,
    chart: Chart,
}

impl<T> ChartApplicationCoordinator<T> {
    pub fn new(repository: T) -> Self {
        Self {
            connect_use_case: ConnectToMarketDataUseCase::new(repository),
            analyze_use_case: AnalyzeMarketDataUseCase::new(),
            render_use_case: RenderChartUseCase::new(),
            historical_use_case: LoadHistoricalDataUseCase::new(),
            chart: Chart::new("main-chart".to_string(), ChartType::Candlestick, 1000),
        }
    }

    /// Асинхронная инициализация с WebGPU рендерером 🚀
    pub async fn initialize_with_webgpu_renderer(
        repository: T,
        canvas_id: String,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            connect_use_case: ConnectToMarketDataUseCase::new(repository),
            analyze_use_case: AnalyzeMarketDataUseCase::new(),
            render_use_case: RenderChartUseCase::initialize_webgpu_renderer(canvas_id, width, height).await,
            historical_use_case: LoadHistoricalDataUseCase::new(),
            chart: Chart::new("main-chart".to_string(), ChartType::Candlestick, 1000),
        }
    }

    /// Получить информацию о рендерере
    pub fn get_render_info(&self) -> String {
        self.render_use_case.get_renderer_info()
    }

    pub fn get_chart(&self) -> &Chart {
        &self.chart
    }

    pub fn get_chart_mut(&mut self) -> &mut Chart {
        &mut self.chart
    }

    pub fn get_connect_use_case_mut(&mut self) -> &mut ConnectToMarketDataUseCase<T> {
        &mut self.connect_use_case
    }

    /// **NEW** Загрузить исторические данные перед подключением к live потоку
    pub async fn initialize_with_historical_data(
        &mut self,
        symbol: &Symbol,
        interval: TimeInterval,
        historical_limit: usize,
    ) -> Result<(), JsValue> {
        log("🚀 Application: Initializing chart with historical data...");

        // Загружаем исторические данные
        self.historical_use_case
            .load_and_populate_chart(&mut self.chart, symbol, interval, historical_limit)
            .await?;

        log(&format!(
            "✅ Application: Chart initialized with {} historical candles",
            self.chart.data.count()
        ));

        Ok(())
    }

    pub fn process_new_candle(&mut self, candle: Candle) -> Result<(), JsValue> {
        log(&format!(
            "📨 Use Case: Received candle data - {} O:{} H:{} L:{} C:{} V:{}",
            candle.timestamp.value(),
            candle.ohlcv.open.value(),
            candle.ohlcv.high.value(),
            candle.ohlcv.low.value(),
            candle.ohlcv.close.value(),
            candle.ohlcv.volume.value()
        ));

        self.analyze_use_case.process_candle(candle, &mut self.chart)
    }

    pub fn prepare_for_rendering(&self) -> Result<(), JsValue> {
        self.render_use_case.prepare_chart_for_rendering(&self.chart)
    }

    pub fn render_chart(&self) -> Result<(), JsValue> {
        self.render_use_case.render_chart(&self.chart)
    }
} 