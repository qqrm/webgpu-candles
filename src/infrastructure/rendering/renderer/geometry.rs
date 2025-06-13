use super::*;
use crate::domain::logging::{LogComponent, get_logger};
use leptos::SignalGetUntracked;

/// Base number of grid cells
pub const BASE_CANDLES: f32 = 300.0;

/// Template of 18 vertices for one candle (body + upper and lower wick)
pub const BASE_TEMPLATE: [CandleVertex; 18] = [
    // Body
    CandleVertex { position_x: -0.5, position_y: 0.0, element_type: 0.0, color_type: 0.0 },
    CandleVertex { position_x: 0.5, position_y: 0.0, element_type: 0.0, color_type: 0.0 },
    CandleVertex { position_x: -0.5, position_y: 1.0, element_type: 0.0, color_type: 0.0 },
    CandleVertex { position_x: 0.5, position_y: 0.0, element_type: 0.0, color_type: 0.0 },
    CandleVertex { position_x: 0.5, position_y: 1.0, element_type: 0.0, color_type: 0.0 },
    CandleVertex { position_x: -0.5, position_y: 1.0, element_type: 0.0, color_type: 0.0 },
    // Upper wick
    CandleVertex { position_x: -0.05, position_y: 0.0, element_type: 1.0, color_type: 0.5 },
    CandleVertex { position_x: 0.05, position_y: 0.0, element_type: 1.0, color_type: 0.5 },
    CandleVertex { position_x: -0.05, position_y: 1.0, element_type: 1.0, color_type: 0.5 },
    CandleVertex { position_x: 0.05, position_y: 0.0, element_type: 1.0, color_type: 0.5 },
    CandleVertex { position_x: 0.05, position_y: 1.0, element_type: 1.0, color_type: 0.5 },
    CandleVertex { position_x: -0.05, position_y: 1.0, element_type: 1.0, color_type: 0.5 },
    // Lower wick
    CandleVertex { position_x: -0.05, position_y: 0.0, element_type: 2.0, color_type: 0.5 },
    CandleVertex { position_x: 0.05, position_y: 0.0, element_type: 2.0, color_type: 0.5 },
    CandleVertex { position_x: -0.05, position_y: 1.0, element_type: 2.0, color_type: 0.5 },
    CandleVertex { position_x: 0.05, position_y: 0.0, element_type: 2.0, color_type: 0.5 },
    CandleVertex { position_x: 0.05, position_y: 1.0, element_type: 2.0, color_type: 0.5 },
    CandleVertex { position_x: -0.05, position_y: 1.0, element_type: 2.0, color_type: 0.5 },
];

/// Minimum element width (candle or volume bar)
pub const MIN_ELEMENT_WIDTH: f32 = 0.002;
/// Maximum element width (candle or volume bar)
pub const MAX_ELEMENT_WIDTH: f32 = 0.1;

/// Candle/bar position taking right edge into account
pub fn candle_x_position(index: usize, visible_len: usize) -> f32 {
    let step_size = 2.0 / visible_len as f32;
    // Snap last candle exactly to the right edge (x=1.0)
    // First candle will be at (1.0 - (visible_len-1) * step_size)
    1.0 - (visible_len as f32 - index as f32 - 1.0) * step_size
}

impl WebGpuRenderer {
    pub(super) fn create_geometry(
        &self,
        chart: &Chart,
    ) -> (Vec<CandleInstance>, Vec<CandleVertex>, ChartUniforms) {
        use crate::app::current_interval;

        let interval = current_interval().get_untracked();
        let candles = chart
            .get_series(interval)
            .map(|s| s.get_candles())
            .unwrap_or_else(|| chart.get_series_for_zoom(self.zoom_level).get_candles());

        if candles.is_empty() {
            get_logger()
                .error(LogComponent::Infrastructure("WebGpuRenderer"), "⚠️ No candles to render");
            return (vec![], BASE_TEMPLATE.to_vec(), ChartUniforms::new());
        }

        let candle_vec: Vec<Candle> = candles.iter().cloned().collect();
        let (start_index, visible_count) =
            crate::app::visible_range_by_time(&candle_vec, &chart.viewport, self.zoom_level);
        let visible_candles: Vec<Candle> =
            candle_vec.iter().skip(start_index).take(visible_count).cloned().collect();

        let mut min_price = chart.viewport.min_price;
        let mut max_price = chart.viewport.max_price;
        if (max_price - min_price).abs() < f32::EPSILON {
            for candle in &visible_candles {
                min_price = min_price.min(candle.ohlcv.low.value() as f32);
                max_price = max_price.max(candle.ohlcv.high.value() as f32);
            }

            let price_range = max_price - min_price;
            min_price -= price_range * 0.05;
            max_price += price_range * 0.05;
        }

        let step_size = 2.0 / visible_candles.len() as f32;
        let candle_width = (step_size * 0.8).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

        let mut instances = Vec::with_capacity(visible_candles.len());
        let price_range = max_price - min_price;
        let price_norm = |p: f64| -> f32 {
            let normalized = (p as f32 - min_price) / price_range;
            -0.5 + normalized * 1.3
        };

        for (i, candle) in visible_candles.iter().enumerate() {
            let x = candle_x_position(i, visible_candles.len());

            let open_y = price_norm(candle.ohlcv.open.value());
            let high_y = price_norm(candle.ohlcv.high.value());
            let low_y = price_norm(candle.ohlcv.low.value());
            let close_y = price_norm(candle.ohlcv.close.value());

            let body_top = open_y.max(close_y);
            let body_bottom = open_y.min(close_y);

            instances.push(CandleInstance {
                x,
                width: candle_width,
                body_top,
                body_bottom,
                high: high_y,
                low: low_y,
                bullish: if close_y >= open_y { 1.0 } else { 0.0 },
                _padding: 0.0,
            });
        }

        let view_proj_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        let uniforms = ChartUniforms {
            view_proj_matrix,
            viewport: [self.width as f32, self.height as f32, min_price, max_price],
            time_range: [0.0, visible_candles.len() as f32, visible_candles.len() as f32, 0.0],
            bullish_color: [0.455, 0.780, 0.529, 1.0],
            bearish_color: [0.882, 0.424, 0.282, 1.0],
            wick_color: [0.6, 0.6, 0.6, 0.9],
            sma20_color: [1.0, 0.2, 0.2, 0.9],
            sma50_color: [1.0, 0.8, 0.0, 0.9],
            sma200_color: [0.2, 0.4, 0.8, 0.9],
            ema12_color: [0.8, 0.2, 0.8, 0.9],
            ema26_color: [0.0, 0.8, 0.8, 0.9],
            current_price_color: [1.0, 1.0, 0.0, 0.8],
            render_params: [candle_width, 0.2, 0.004, 0.0],
        };

        (instances, BASE_TEMPLATE.to_vec(), uniforms)
    }
}
