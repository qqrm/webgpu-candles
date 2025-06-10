use crate::domain::market_data::{Candle, CandleSeries};
use super::value_objects::{ChartType, Viewport};

/// Доменная сущность - График
#[derive(Debug, Clone)]
pub struct Chart {
    pub id: String,
    pub chart_type: ChartType,
    pub data: CandleSeries,
    pub viewport: Viewport,
    pub indicators: Vec<Indicator>,
}

impl Chart {
    pub fn new(id: String, chart_type: ChartType, max_candles: usize) -> Self {
        Self {
            id,
            chart_type,
            data: CandleSeries::new(max_candles),
            viewport: Viewport::default(),
            indicators: Vec::new(),
        }
    }

    pub fn add_candle(&mut self, candle: Candle) {
        self.data.add_candle(candle);
    }

    /// Добавить исторические данные (замещает существующие)
    pub fn set_historical_data(&mut self, mut candles: Vec<Candle>) {
        // Сортируем по времени для стабильности
        candles.sort_by(|a, b| a.timestamp.value().cmp(&b.timestamp.value()));
        
        // Создаем новую серию с исходным лимитом
        let limit = self.data.max_size();
        self.data = CandleSeries::new(limit);
        
        // Добавляем исторические свечи (уже отсортированные)
        for candle in candles {
            self.data.add_candle(candle);
        }
        
        // Обновляем viewport
        self.update_viewport_for_data();
    }

    /// Добавить новую свечу в реальном времени
    pub fn add_realtime_candle(&mut self, candle: Candle) {
        // CandleSeries уже обрабатывает обновления существующих свечей
        self.data.add_candle(candle);
        
        // Обновляем viewport
        self.update_viewport_for_data();
    }

    /// Получить общее количество свечей
    pub fn get_candle_count(&self) -> usize {
        self.data.count()
    }

    /// Проверить, есть ли данные
    pub fn has_data(&self) -> bool {
        self.data.count() > 0
    }

    pub fn add_indicator(&mut self, indicator: Indicator) {
        self.indicators.push(indicator);
    }

    pub fn remove_indicator(&mut self, indicator_id: &str) {
        self.indicators.retain(|ind| ind.id != indicator_id);
    }

    /// Обновить viewport на основе данных свечей
    pub fn update_viewport_for_data(&mut self) {
        if let Some((min_price, max_price)) = self.data.price_range() {
            // Добавляем отступы для лучшей визуализации (5% сверху и снизу)
            let price_range = max_price.value() - min_price.value();
            let padding = (price_range * 0.05) as f32;
            
            self.viewport.min_price = (min_price.value() as f32 - padding).max(0.1); // Минимум $0.1
            self.viewport.max_price = max_price.value() as f32 + padding;
            
            // Обновляем временной диапазон
            let candles = self.data.get_candles();
            if !candles.is_empty() {
                self.viewport.start_time = candles.front().unwrap().timestamp.value() as f64;
                self.viewport.end_time = candles.back().unwrap().timestamp.value() as f64;
            }
        }
    }

    #[allow(dead_code)]
    fn update_viewport(&mut self) {
        if let Some((min_price, max_price)) = self.data.price_range() {
            let padding = (max_price.value() - min_price.value()) * 0.1; // 10% padding
            self.viewport.min_price = min_price.value() as f32 - padding as f32;
            self.viewport.max_price = max_price.value() as f32 + padding as f32;
        }

        let candles = self.data.get_candles();
        if !candles.is_empty() {
            self.viewport.start_time = candles.front().unwrap().timestamp.as_f64();
            self.viewport.end_time = candles.back().unwrap().timestamp.as_f64();
        }
    }

    pub fn zoom(&mut self, factor: f32, center_x: f32) {
        self.viewport.zoom(factor, center_x);
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.viewport.pan(delta_x, delta_y);
    }
}

/// Simplified Indicator entity - only essential fields
#[derive(Debug, Clone)]
pub struct Indicator {
    pub id: String,
    pub indicator_type: IndicatorType,
}

impl Indicator {
    pub fn new(id: String, indicator_type: IndicatorType) -> Self {
        Self {
            id,
            indicator_type,
        }
    }
}

/// Essential indicator types only
#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorType {
    SimpleMovingAverage,
    ExponentialMovingAverage,
    MACD,
}

// Removed unused complex structures:
// - IndicatorParameters, IndicatorStyle, PriceSource, LineStyle
// - RenderLayer, RenderElement 
// - CandlestickStyle, TextStyle, FontWeight, ShapeType, ShapeStyle
// These are handled directly in the WebGPU renderer for better performance 