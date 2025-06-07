use crate::domain::market_data::{
    repositories::MarketDataRepository, Symbol, TimeInterval, 
    services::{MarketAnalysisService, DataValidationService}
};
use crate::domain::chart::{Chart, services::ChartRenderingService};
use wasm_bindgen::JsValue;
use std::rc::Rc;
use std::cell::RefCell;

/// Use Case: Подключение к реальным данным
pub struct ConnectToMarketDataUseCase<T: MarketDataRepository> {
    repository: T,
    validation_service: DataValidationService,
    chart: Rc<RefCell<Chart>>,
}

impl<T: MarketDataRepository> ConnectToMarketDataUseCase<T> {
    pub fn new(repository: T, chart: Rc<RefCell<Chart>>) -> Self {
        Self {
            repository,
            validation_service: DataValidationService::new(),
            chart,
        }
    }

    pub fn execute(&mut self, symbol: Symbol, interval: TimeInterval) -> Result<(), JsValue> {
        let chart_clone = self.chart.clone();
        let validation_service = self.validation_service.clone();
        
        #[allow(unused_unsafe)]
        unsafe {
            web_sys::console::log_1(&format!(
                "🔗 Use Case: Setting up WebSocket subscription for {} {}",
                symbol.value(),
                interval.to_binance_str()
            ).into());
        }
        
        self.repository.subscribe_to_updates(
            &symbol,
            interval,
            Box::new(move |candle| {
                #[allow(unused_unsafe)]
                unsafe {
                    web_sys::console::log_1(&format!(
                        "📨 Use Case: Received candle data - {} O:{} H:{} L:{} C:{} V:{}",
                        candle.timestamp.value(),
                        candle.ohlcv.open.value(),
                        candle.ohlcv.high.value(),
                        candle.ohlcv.low.value(),
                        candle.ohlcv.close.value(),
                        candle.ohlcv.volume.value()
                    ).into());
                }

                // Валидация данных
                if let Err(e) = validation_service.validate_candle(&candle) {
                    #[allow(unused_unsafe)]
                    unsafe {
                        web_sys::console::error_1(&format!("❌ Invalid candle data: {}", e).into());
                    }
                    return;
                }

                #[allow(unused_unsafe)]
                unsafe {
                    web_sys::console::log_1(&"✅ Candle validation passed, adding to chart...".into());
                }

                // Добавление в график
                let candle_for_log = candle.clone();
                chart_clone.borrow_mut().add_candle(candle);
                
                // Логируем состояние графика после добавления
                let chart_borrowed = chart_clone.borrow();
                let total_candles = chart_borrowed.data.count();
                
                #[allow(unused_unsafe)]
                unsafe {
                    web_sys::console::log_1(&format!(
                        "📊 ChartState updated: Total candles: {}, Latest: O:{} H:{} L:{} C:{} V:{}",
                        total_candles,
                        candle_for_log.ohlcv.open.value(),
                        candle_for_log.ohlcv.high.value(),
                        candle_for_log.ohlcv.low.value(),
                        candle_for_log.ohlcv.close.value(),
                        candle_for_log.ohlcv.volume.value()
                    ).into());
                }
            })
        )
    }
}

/// Use Case: Анализ рыночных данных
pub struct AnalyzeMarketDataUseCase {
    analysis_service: MarketAnalysisService,
}

impl AnalyzeMarketDataUseCase {
    pub fn new() -> Self {
        Self {
            analysis_service: MarketAnalysisService::new(),
        }
    }

    pub fn calculate_moving_average(&self, chart: &Chart, period: usize) -> Vec<f32> {
        let candles = chart.data.get_candles();
        self.analysis_service
            .calculate_sma(candles, period)
            .into_iter()
            .map(|price| price.value())
            .collect()
    }

    pub fn calculate_volatility(&self, chart: &Chart, period: usize) -> Option<f32> {
        let candles = chart.data.get_candles();
        self.analysis_service.calculate_volatility(candles, period)
    }

    pub fn find_support_resistance(&self, chart: &Chart, window: usize) -> (Vec<usize>, Vec<usize>) {
        let candles = chart.data.get_candles();
        self.analysis_service.find_extremes(candles, window)
    }
}

/// Use Case: Рендеринг графика
pub struct RenderChartUseCase {
    rendering_service: ChartRenderingService,
}

impl RenderChartUseCase {
    pub fn new() -> Self {
        Self {
            rendering_service: ChartRenderingService::new(),
        }
    }

    pub fn prepare_render_data(&self, chart: &Chart) -> ChartRenderData {
        let layers = self.rendering_service.create_render_layers(chart);
        
        ChartRenderData {
            layers,
            viewport: chart.viewport.clone(),
            style: chart.style.clone(),
            candle_count: chart.data.count(),
        }
    }
}

/// DTO для передачи данных рендеринга
#[derive(Debug, Clone)]
pub struct ChartRenderData {
    pub layers: Vec<crate::domain::chart::RenderLayer>,
    pub viewport: crate::domain::chart::Viewport,
    pub style: crate::domain::chart::ChartStyle,
    pub candle_count: usize,
}

/// Координатор всех use cases
pub struct ChartApplicationCoordinator<T: MarketDataRepository> {
    connect_use_case: ConnectToMarketDataUseCase<T>,
    analyze_use_case: AnalyzeMarketDataUseCase,
    render_use_case: RenderChartUseCase,
}

impl<T: MarketDataRepository> ChartApplicationCoordinator<T> {
    pub fn new(repository: T, chart: Rc<RefCell<Chart>>) -> Self {
        Self {
            connect_use_case: ConnectToMarketDataUseCase::new(repository, chart),
            analyze_use_case: AnalyzeMarketDataUseCase::new(),
            render_use_case: RenderChartUseCase::new(),
        }
    }

    /// Полный сценарий: подключение + анализ + рендеринг
    pub fn start_live_chart(&mut self, symbol: Symbol, interval: TimeInterval) -> Result<(), JsValue> {
        #[allow(unused_unsafe)]
        unsafe {
            web_sys::console::log_1(&format!(
                "Starting live chart for {} with {} interval", 
                symbol.value(), 
                interval.to_binance_str()
            ).into());
        }

        self.connect_use_case.execute(symbol, interval)
    }

    pub fn get_analysis(&self, chart: &Chart) -> MarketAnalysisResult {
        MarketAnalysisResult {
            sma_20: self.analyze_use_case.calculate_moving_average(chart, 20),
            sma_50: self.analyze_use_case.calculate_moving_average(chart, 50),
            volatility: self.analyze_use_case.calculate_volatility(chart, 20),
            support_resistance: self.analyze_use_case.find_support_resistance(chart, 5),
        }
    }

    pub fn prepare_chart_render(&self, chart: &Chart) -> ChartRenderData {
        self.render_use_case.prepare_render_data(chart)
    }
}

/// Результат анализа рынка
#[derive(Debug, Clone)]
pub struct MarketAnalysisResult {
    pub sma_20: Vec<f32>,
    pub sma_50: Vec<f32>,
    pub volatility: Option<f32>,
    pub support_resistance: (Vec<usize>, Vec<usize>),
} 