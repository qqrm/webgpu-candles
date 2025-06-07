use crate::domain::market_data::{Candle, CandleSeries};
use super::value_objects::{ChartType, Viewport, ChartStyle, Color};

/// Доменная сущность - График
#[derive(Debug, Clone)]
pub struct Chart {
    pub id: String,
    pub chart_type: ChartType,
    pub data: CandleSeries,
    pub viewport: Viewport,
    pub style: ChartStyle,
    pub indicators: Vec<Indicator>,
}

impl Chart {
    pub fn new(id: String, chart_type: ChartType, max_candles: usize) -> Self {
        Self {
            id,
            chart_type,
            data: CandleSeries::new(max_candles),
            viewport: Viewport::default(),
            style: ChartStyle::default(),
            indicators: Vec::new(),
        }
    }

    pub fn add_candle(&mut self, candle: Candle) {
        self.data.add_candle(candle);
    }

    /// Добавить исторические данные (замещает существующие)
    pub fn set_historical_data(&mut self, candles: Vec<Candle>) {
        // Создаем новую серию с тем же размером
        self.data = CandleSeries::new(1000); // Максимум 1000 свечей
        
        // Добавляем исторические свечи
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
            let padding = price_range * 0.05;
            
            self.viewport.min_price = (min_price.value() - padding).max(0.1); // Минимум $0.1
            self.viewport.max_price = max_price.value() + padding;
            
            // Обновляем временной диапазон
            let candles = self.data.get_candles();
            if !candles.is_empty() {
                self.viewport.start_time = candles.first().unwrap().timestamp.value() as f64;
                self.viewport.end_time = candles.last().unwrap().timestamp.value() as f64;
            }
        }
    }

    #[allow(dead_code)]
    fn update_viewport(&mut self) {
        if let Some((min_price, max_price)) = self.data.price_range() {
            let padding = (max_price.value() - min_price.value()) * 0.1; // 10% padding
            self.viewport.min_price = min_price.value() - padding;
            self.viewport.max_price = max_price.value() + padding;
        }

        let candles = self.data.get_candles();
        if !candles.is_empty() {
            self.viewport.start_time = candles.first().unwrap().timestamp.as_f64();
            self.viewport.end_time = candles.last().unwrap().timestamp.as_f64();
        }
    }

    pub fn zoom(&mut self, factor: f32, center_x: f32) {
        self.viewport.zoom(factor, center_x);
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.viewport.pan(delta_x, delta_y);
    }
}

/// Доменная сущность - Индикатор
#[derive(Debug, Clone)]
pub struct Indicator {
    pub id: String,
    pub indicator_type: IndicatorType,
    pub parameters: IndicatorParameters,
    pub style: IndicatorStyle,
}

impl Indicator {
    pub fn new(id: String, indicator_type: IndicatorType, parameters: IndicatorParameters) -> Self {
        Self {
            id,
            indicator_type,
            parameters,
            style: IndicatorStyle::default(),
        }
    }
}

/// Типы индикаторов
#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorType {
    SimpleMovingAverage,
    ExponentialMovingAverage,
    RelativeStrengthIndex,
    BollingerBands,
    MACD,
    Volume,
    SupportResistance,
}

/// Параметры индикатора
#[derive(Debug, Clone)]
pub struct IndicatorParameters {
    pub period: Option<usize>,
    pub multiplier: Option<f32>,
    pub source: PriceSource,
    pub custom_params: std::collections::HashMap<String, f32>,
}

impl Default for IndicatorParameters {
    fn default() -> Self {
        Self {
            period: Some(20),
            multiplier: Some(2.0),
            source: PriceSource::Close,
            custom_params: std::collections::HashMap::new(),
        }
    }
}

/// Источник цены для индикатора
#[derive(Debug, Clone, PartialEq)]
pub enum PriceSource {
    Open,
    High,
    Low,
    Close,
    Volume,
    HL2,   // (High + Low) / 2
    HLC3,  // (High + Low + Close) / 3
    OHLC4, // (Open + High + Low + Close) / 4
}

/// Стиль индикатора
#[derive(Debug, Clone)]
pub struct IndicatorStyle {
    pub color: Color,
    pub line_width: f32,
    pub line_style: LineStyle,
    pub visible: bool,
}

impl Default for IndicatorStyle {
    fn default() -> Self {
        Self {
            color: Color::new(0.0, 1.0, 0.0, 1.0), // Зеленый
            line_width: 1.0,
            line_style: LineStyle::Solid,
            visible: true,
        }
    }
}

/// Стили линий
#[derive(Debug, Clone, PartialEq)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
}

/// Доменная сущность - Слой рендеринга
#[derive(Debug, Clone)]
pub struct RenderLayer {
    pub id: String,
    pub z_order: i32,
    pub visible: bool,
    pub opacity: f32,
    pub elements: Vec<RenderElement>,
}

impl RenderLayer {
    pub fn new(id: String, z_order: i32) -> Self {
        Self {
            id,
            z_order,
            visible: true,
            opacity: 1.0,
            elements: Vec::new(),
        }
    }

    pub fn add_element(&mut self, element: RenderElement) {
        self.elements.push(element);
    }

    pub fn clear(&mut self) {
        self.elements.clear();
    }
}

/// Элемент рендеринга
#[derive(Debug, Clone)]
pub enum RenderElement {
    Candlestick {
        timestamp: f64,
        open: f32,
        high: f32,
        low: f32,
        close: f32,
        volume: f32,
        style: CandlestickStyle,
    },
    Line {
        points: Vec<(f64, f32)>,
        style: IndicatorStyle,
    },
    Text {
        x: f64,
        y: f32,
        text: String,
        style: TextStyle,
    },
    Shape {
        shape_type: ShapeType,
        points: Vec<(f64, f32)>,
        style: ShapeStyle,
    },
}

/// Стиль свечи
#[derive(Debug, Clone)]
pub struct CandlestickStyle {
    pub bullish_color: Color,
    pub bearish_color: Color,
    pub wick_color: Color,
    pub border_width: f32,
}

impl Default for CandlestickStyle {
    fn default() -> Self {
        Self {
            bullish_color: Color::new(0.0, 1.0, 0.0, 1.0), // Зеленый
            bearish_color: Color::new(1.0, 0.0, 0.0, 1.0), // Красный
            wick_color: Color::new(0.5, 0.5, 0.5, 1.0),    // Серый
            border_width: 1.0,
        }
    }
}

/// Стиль текста
#[derive(Debug, Clone)]
pub struct TextStyle {
    pub color: Color,
    pub font_size: f32,
    pub font_weight: FontWeight,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: Color::new(1.0, 1.0, 1.0, 1.0), // Белый
            font_size: 12.0,
            font_weight: FontWeight::Normal,
        }
    }
}

/// Толщина шрифта
#[derive(Debug, Clone, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
}

/// Типы фигур
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeType {
    Rectangle,
    Circle,
    Triangle,
    Arrow,
}

/// Стиль фигуры
#[derive(Debug, Clone)]
pub struct ShapeStyle {
    pub fill_color: Option<Color>,
    pub border_color: Color,
    pub border_width: f32,
}

impl Default for ShapeStyle {
    fn default() -> Self {
        Self {
            fill_color: None,
            border_color: Color::new(1.0, 1.0, 1.0, 1.0), // Белый
            border_width: 1.0,
        }
    }
} 