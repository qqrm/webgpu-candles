use crate::domain::chart::{Chart, RenderLayer, RenderElement};

/// Доменный сервис для рендеринга графиков
pub struct ChartRenderingService;

impl ChartRenderingService {
    pub fn new() -> Self {
        Self
    }

    /// Создает слои рендеринга для графика
    pub fn create_render_layers(&self, chart: &Chart) -> Vec<RenderLayer> {
        let mut layers = Vec::new();

        // Базовый слой для свечей
        let mut candle_layer = RenderLayer::new("candles".to_string(), 0);
        
        // Добавляем элементы свечей
        for candle in chart.data.get_candles() {
            let element = RenderElement::Candlestick {
                timestamp: candle.timestamp.as_f64(),
                open: candle.ohlcv.open.value(),
                high: candle.ohlcv.high.value(),
                low: candle.ohlcv.low.value(),
                close: candle.ohlcv.close.value(),
                volume: candle.ohlcv.volume.value(),
                style: Default::default(),
            };
            candle_layer.add_element(element);
        }
        
        layers.push(candle_layer);

        // Слои для индикаторов
        for (i, _indicator) in chart.indicators.iter().enumerate() {
            let indicator_layer = RenderLayer::new(format!("indicator_{}", i), i as i32 + 1);
            // TODO: Здесь должна быть логика создания элементов индикаторов
            layers.push(indicator_layer);
        }

        layers
    }
}

/// Доменный сервис для управления состоянием графика
pub struct ChartStateService;

impl ChartStateService {
    pub fn new() -> Self {
        Self
    }

    /// Обновляет viewport графика при изменении размеров
    pub fn update_viewport_size(&self, chart: &mut Chart, width: u32, height: u32) {
        chart.viewport.width = width;
        chart.viewport.height = height;
    }

    /// Автоматически масштабирует график по данным
    pub fn auto_scale(&self, chart: &mut Chart) {
        if let Some((min_price, max_price)) = chart.data.price_range() {
            let padding = (max_price.value() - min_price.value()) * 0.1;
            chart.viewport.min_price = min_price.value() - padding;
            chart.viewport.max_price = max_price.value() + padding;
        }

        let candles = chart.data.get_candles();
        if !candles.is_empty() {
            chart.viewport.start_time = candles.first().unwrap().timestamp.as_f64();
            chart.viewport.end_time = candles.last().unwrap().timestamp.as_f64();
        }
    }

    /// Проверяет, нужно ли обновить график
    pub fn should_update(&self, chart: &Chart) -> bool {
        // Простая логика - обновляем если есть данные
        chart.data.count() > 0
    }
}