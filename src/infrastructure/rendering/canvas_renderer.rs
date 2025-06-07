use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d};
use rayon::prelude::*; // Параллельная обработка
use crate::domain::{
    chart::Chart,
    logging::{LogComponent, get_logger},
    market_data::entities::Candle,
};

/// Данные для рендеринга одной свечи (предвычислено)
#[derive(Debug, Clone)]
struct CandleRenderData {
    x: f64,
    high_y: f64,
    low_y: f64,
    open_y: f64,
    close_y: f64,
    color: String,
    is_bullish: bool,
    body_width: f64,
    candle_width: f64,
}

/// Параметры масштабирования для всего графика
#[derive(Debug, Clone)]
struct ScaleParams {
    padding: f64,
    text_space: f64,
    chart_width: f64,
    chart_height: f64,
    min_price: f64,
    max_price: f64,
    price_range: f64,
    candle_width: f64,
}

/// Canvas 2D renderer for charts - Infrastructure implementation
pub struct CanvasRenderer {
    canvas_id: String,
    width: u32,
    height: u32,
    parallel_threshold: usize, // Минимум свечей для параллелизации
}

impl CanvasRenderer {
    pub fn new(canvas_id: String, width: u32, height: u32) -> Self {
        Self {
            canvas_id,
            width,
            height,
            parallel_threshold: 100, // Параллелим если больше 100 свечей
        }
    }

    /// Get canvas element and context
    fn get_canvas_context(&self) -> Result<(HtmlCanvasElement, CanvasRenderingContext2d), JsValue> {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let canvas = document
            .get_element_by_id(&self.canvas_id)
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("Failed to get canvas element"))?;

        canvas.set_width(self.width);
        canvas.set_height(self.height);

        let context = canvas
            .get_context("2d")
            .map_err(|_| JsValue::from_str("Failed to get 2D context"))?
            .unwrap()
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| JsValue::from_str("Failed to cast to 2D context"))?;

        Ok((canvas, context))
    }

    /// Render chart with candlestick data (with parallel processing)
    pub fn render_chart(&self, chart: &Chart) -> Result<(), JsValue> {
        get_logger().debug(
            LogComponent::Infrastructure("CanvasRenderer"),
            "Starting parallel chart rendering..."
        );

        let (_canvas, context) = self.get_canvas_context()?;

        // Clear canvas
        context.clear_rect(0.0, 0.0, self.width as f64, self.height as f64);

        // Dark background for modern UI
        context.set_fill_style(&JsValue::from("#1a1a1a"));
        context.fill_rect(0.0, 0.0, self.width as f64, self.height as f64);

        let candles = chart.data.get_candles();

        if !candles.is_empty() {
            let use_parallel = candles.len() >= self.parallel_threshold;
            
            get_logger().info(
                LogComponent::Infrastructure("CanvasRenderer"),
                &format!("Rendering {} candles (parallel: {})", candles.len(), use_parallel)
            );

            if use_parallel {
                self.render_candles_parallel(&context, candles)?;
            } else {
                self.render_candles_sequential(&context, candles)?;
            }
            
            self.render_price_scale(&context, candles)?;
            self.render_current_price_line(&context, candles)?;
        } else {
            self.render_no_data_message(&context)?;
        }

        // Render title
        self.render_title(&context)?;

        get_logger().debug(
            LogComponent::Infrastructure("CanvasRenderer"),
            "Chart rendered successfully"
        );

        Ok(())
    }

    /// 🚀 Параллельный рендеринг свечей
    fn render_candles_parallel(&self, context: &CanvasRenderingContext2d, candles: &[Candle]) -> Result<(), JsValue> {
        let start_time = web_sys::window().unwrap().performance().unwrap().now();
        
        // Вычисляем параметры масштабирования
        let scale_params = self.calculate_scale_params(candles);
        
        get_logger().debug(
            LogComponent::Infrastructure("CanvasRenderer"),
            &format!("🔥 Parallel processing {} candles with {} threads", 
                candles.len(), 
                rayon::current_num_threads())
        );

        // 🚀 ПАРАЛЛЕЛЬНО вычисляем данные для рендеринга всех свечей
        let render_data: Vec<CandleRenderData> = candles
            .par_iter() // Параллельная итерация!
            .enumerate()
            .map(|(i, candle)| {
                self.calculate_candle_render_data(i, candle, &scale_params)
            })
            .collect();

        let calc_time = web_sys::window().unwrap().performance().unwrap().now();
        
        // Последовательно рендерим (Canvas 2D не thread-safe)
        for data in render_data {
            self.render_single_candle(&context, &data)?;
        }

        let end_time = web_sys::window().unwrap().performance().unwrap().now();
        
        get_logger().info(
            LogComponent::Infrastructure("CanvasRenderer"),
            &format!("⚡ Parallel rendering: calc={:.1}ms, draw={:.1}ms, total={:.1}ms", 
                calc_time - start_time,
                end_time - calc_time,
                end_time - start_time)
        );

        Ok(())
    }

    /// Последовательный рендеринг (для малого количества свечей)
    fn render_candles_sequential(&self, context: &CanvasRenderingContext2d, candles: &[Candle]) -> Result<(), JsValue> {
        let start_time = web_sys::window().unwrap().performance().unwrap().now();
        
        let scale_params = self.calculate_scale_params(candles);

        for (i, candle) in candles.iter().enumerate() {
            let render_data = self.calculate_candle_render_data(i, candle, &scale_params);
            self.render_single_candle(&context, &render_data)?;
        }

        let end_time = web_sys::window().unwrap().performance().unwrap().now();
        
        get_logger().debug(
            LogComponent::Infrastructure("CanvasRenderer"),
            &format!("🐌 Sequential rendering: {:.1}ms", end_time - start_time)
        );

        Ok(())
    }

    /// Вычисляем параметры масштабирования один раз для всех свечей
    fn calculate_scale_params(&self, candles: &[Candle]) -> ScaleParams {
        let padding = 50.0;
        let text_space = 80.0;
        let chart_width = self.width as f64 - (padding * 2.0) - text_space;
        let chart_height = self.height as f64 - (padding * 2.0);

        // Находим ценовой диапазон
        let mut min_price = f64::INFINITY;
        let mut max_price = f64::NEG_INFINITY;

        for candle in candles {
            min_price = min_price.min(candle.ohlcv.low.value() as f64);
            max_price = max_price.max(candle.ohlcv.high.value() as f64);
        }

        let price_range = max_price - min_price;
        let candle_width = chart_width / candles.len() as f64;

        ScaleParams {
            padding,
            text_space,
            chart_width,
            chart_height,
            min_price,
            max_price,
            price_range,
            candle_width,
        }
    }

    /// 🔥 Вычисляем данные для рендеринга одной свечи (thread-safe)
    fn calculate_candle_render_data(&self, index: usize, candle: &Candle, params: &ScaleParams) -> CandleRenderData {
        let x = params.padding + (index as f64 * params.candle_width) + (params.candle_width / 2.0);

        // Convert prices to Y coordinates (invert because Y grows down)
        let high_y = params.padding + ((params.max_price - candle.ohlcv.high.value() as f64) / params.price_range) * params.chart_height;
        let low_y = params.padding + ((params.max_price - candle.ohlcv.low.value() as f64) / params.price_range) * params.chart_height;
        let open_y = params.padding + ((params.max_price - candle.ohlcv.open.value() as f64) / params.price_range) * params.chart_height;
        let close_y = params.padding + ((params.max_price - candle.ohlcv.close.value() as f64) / params.price_range) * params.chart_height;

        let is_bullish = candle.ohlcv.close.value() >= candle.ohlcv.open.value();
        let color = if is_bullish { "#00ff88".to_string() } else { "#ff4444".to_string() };
        let body_width = params.candle_width * 0.6;

        CandleRenderData {
            x,
            high_y,
            low_y,
            open_y,
            close_y,
            color,
            is_bullish,
            body_width,
            candle_width: params.candle_width,
        }
    }

    /// Рендеринг одной свечи по предвычисленным данным
    fn render_single_candle(&self, context: &CanvasRenderingContext2d, data: &CandleRenderData) -> Result<(), JsValue> {
        // Draw wick (high-low)
        context.set_stroke_style(&JsValue::from("#888888"));
        context.set_line_width(1.0);
        context.begin_path();
        context.move_to(data.x, data.high_y);
        context.line_to(data.x, data.low_y);
        context.stroke();

        // Draw candle body
        context.set_fill_style(&JsValue::from(&data.color));
        context.set_stroke_style(&JsValue::from(&data.color));
        context.set_line_width(1.0);

        let body_top = data.open_y.min(data.close_y);
        let body_height = (data.open_y - data.close_y).abs();

        if body_height < 1.0 {
            // Doji - draw line
            context.begin_path();
            context.move_to(data.x - data.body_width / 2.0, data.open_y);
            context.line_to(data.x + data.body_width / 2.0, data.open_y);
            context.stroke();
        } else {
            // Normal candle
            if data.is_bullish {
                // Bullish candle - outline
                context.stroke_rect(data.x - data.body_width / 2.0, body_top, data.body_width, body_height);
            } else {
                // Bearish candle - filled
                context.fill_rect(data.x - data.body_width / 2.0, body_top, data.body_width, body_height);
            }
        }

        Ok(())
    }

    fn render_price_scale(&self, context: &CanvasRenderingContext2d, candles: &[Candle]) -> Result<(), JsValue> {
        let padding = 50.0;
        let text_space = 80.0;
        let chart_height = self.height as f64 - (padding * 2.0);

        // Find price range
        let mut min_price = f64::INFINITY;
        let mut max_price = f64::NEG_INFINITY;

        for candle in candles {
            min_price = min_price.min(candle.ohlcv.low.value() as f64);
            max_price = max_price.max(candle.ohlcv.high.value() as f64);
        }

        // Render price scale
        context.set_fill_style(&JsValue::from("#aaaaaa"));
        context.set_font("12px Arial");

        // Maximum price
        let max_text = format!("${:.2}", max_price);
        context.fill_text(&max_text, 10.0, padding + 15.0)?;

        // Minimum price
        let min_text = format!("${:.2}", min_price);
        context.fill_text(&min_text, 10.0, padding + chart_height)?;

        Ok(())
    }

    fn render_current_price_line(&self, context: &CanvasRenderingContext2d, candles: &[Candle]) -> Result<(), JsValue> {
        let padding = 50.0;
        let text_space = 80.0;
        let chart_width = self.width as f64 - (padding * 2.0) - text_space;
        let chart_height = self.height as f64 - (padding * 2.0);

        if let Some(latest) = candles.last() {
            // Find price range for scaling
            let mut min_price = f64::INFINITY;
            let mut max_price = f64::NEG_INFINITY;

            for candle in candles {
                min_price = min_price.min(candle.ohlcv.low.value() as f64);
                max_price = max_price.max(candle.ohlcv.high.value() as f64);
            }

            let price_range = max_price - min_price;
            let current_price = latest.ohlcv.close.value();
            let current_y = padding + ((max_price - current_price as f64) / price_range) * chart_height;
            let current_text = format!("${:.2}", current_price);

            // Horizontal line for current price
            context.set_stroke_style(&JsValue::from("#00ff88"));
            context.set_line_width(1.0);
            context.begin_path();
            context.move_to(padding, current_y);
            context.line_to(padding + chart_width, current_y);
            context.stroke();

            // Price text to the right of the line with offset
            let line_end = padding + chart_width;
            let text_offset = 10.0;
            context.set_fill_style(&JsValue::from("#00ff88"));
            context.fill_text(&current_text, line_end + text_offset, current_y + 5.0)?;
        }

        Ok(())
    }

    fn render_no_data_message(&self, context: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        context.set_fill_style(&JsValue::from("#ffffff"));
        context.set_font("16px Arial");
        let text = "No chart data available - Loading...";
        context.fill_text(text, 50.0, self.height as f64 / 2.0)?;

        get_logger().warn(
            LogComponent::Infrastructure("CanvasRenderer"),
            "No candle data to render"
        );

        Ok(())
    }

    fn render_title(&self, context: &CanvasRenderingContext2d) -> Result<(), JsValue> {
        context.set_fill_style(&JsValue::from("#ffffff"));
        context.set_font("16px Arial");
        let title = "Production-Ready Candlestick Chart";
        context.fill_text(title, 50.0, 30.0)?;
        Ok(())
    }

    /// Update canvas dimensions
    pub fn set_dimensions(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
} 