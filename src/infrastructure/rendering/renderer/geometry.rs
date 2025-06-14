use super::*;
use crate::domain::logging::{LogComponent, get_logger};
use crate::infrastructure::rendering::gpu_structures::{CandleGeometry, CandleInstance};
use leptos::SignalGetUntracked;

/// Base number of grid cells
pub const BASE_CANDLES: f32 = 100.0;

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

            return (Vec::new(), Vec::new(), ChartUniforms::new());
        }

        // ⚡ Performance: log less frequently
        if candles.len() % 100 == 0 {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!("🔧 Creating optimized geometry for {} candles", candles.len()),
            );
        }

        let candle_count = candles.len();
        let chart_width = 2.0; // NDC width (-1 to 1)

        // 🔍 Apply zoom - show fewer candles when zooming in
        let candle_vec: Vec<Candle> = candles.iter().cloned().collect();
        let (start_index, visible_count) =
            crate::app::visible_range_by_time(&candle_vec, &chart.viewport, self.zoom_level);
        let visible_candles: Vec<Candle> =
            candle_vec.iter().skip(start_index).take(visible_count).cloned().collect();

        let mut vertices = Vec::with_capacity(visible_candles.len() * 24);

        // Scale candles based on currently visible data
        let mut min_price = f32::INFINITY;
        let mut max_price = f32::NEG_INFINITY;
        for candle in &visible_candles {
            min_price = min_price.min(candle.ohlcv.low.value() as f32);
            max_price = max_price.max(candle.ohlcv.high.value() as f32);
        }

        let price_range = max_price - min_price;
        min_price -= price_range * 0.05;
        max_price += price_range * 0.05;

        // Calculate visible candle width and spacing
        let spacing_ratio = 0.2; // 20% spacing between candles
        let step_size = chart_width / candle_count as f64;
        let max_candle_width = step_size * (1.0 - spacing_ratio);
        let _candle_width = max_candle_width.clamp(0.01, 0.06); // Reasonable width limits

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!(
                "📏 Price range: {:.2} - {:.2}, Candle width: {:.4}, step:{:.4}",
                min_price, max_price, _candle_width, step_size
            ),
        );

        // Ensure we have a valid price range
        if (max_price - min_price).abs() < 0.01 {
            get_logger()
                .error(LogComponent::Infrastructure("WebGpuRenderer"), "❌ Invalid price range!");
            return (Vec::new(), Vec::new(), ChartUniforms::new());
        }

        // Log less often for performance
        if visible_candles.len() % 50 == 0 {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!(
                    "🔧 Rendering {} candles (showing last {} of {}) [zoom: {:.2}x]",
                    visible_candles.len(),
                    visible_count,
                    candles.len(),
                    self.zoom_level
                ),
            );
        }

        // Create instance data for each visible candle
        let step_size = 2.0 / visible_candles.len() as f32;
        let candle_width = (step_size * 0.8).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
        let mut instances = Vec::with_capacity(visible_candles.len());

        let half_width = candle_width * 0.5;

        for (i, candle) in visible_candles.iter().enumerate() {
            let x = candle_x_position(i, visible_candles.len());

            // Normalize Y - use the upper part of the screen [-0.5, 0.8] for candles
            let price_range = max_price - min_price;
            let price_norm = |price: f64| -> f32 {
                let normalized = (price as f32 - min_price) / price_range;
                -0.5 + normalized * 1.3 // Map to [-0.5, 0.8] - leave room for volume
            };

            let open_y = price_norm(candle.ohlcv.open.value());
            let high_y = price_norm(candle.ohlcv.high.value());
            let low_y = price_norm(candle.ohlcv.low.value());
            let close_y = price_norm(candle.ohlcv.close.value());

            // Log only the first 3 and last 3 candles
            if i < 3 || i >= visible_candles.len() - 3 {
                get_logger().info(
                    LogComponent::Infrastructure("WebGpuRenderer"),
                    &format!(
                        "🕯️ Candle {}: x={:.3}, Y=({:.3},{:.3},{:.3},{:.3}) width={:.4}",
                        i, x, open_y, high_y, low_y, close_y, candle_width
                    ),
                );
            }

            let body_top = open_y.max(close_y);
            let body_bottom = open_y.min(close_y);

            // Minimum height for visibility
            let min_height = 0.005;
            let actual_body_top = if (body_top - body_bottom).abs() < min_height {
                body_bottom + min_height
            } else {
                body_top
            };

            let is_bullish = close_y >= open_y;

            instances.push(CandleInstance {
                x,
                width: candle_width,
                body_top: actual_body_top,
                body_bottom,
                high: high_y,
                low: low_y,
                bullish: if is_bullish { 1.0 } else { 0.0 },
                _padding: 0.0,
            });

            // Candle body
            let body_vertices = vec![
                CandleVertex::body_vertex(x - half_width, body_bottom, is_bullish),
                CandleVertex::body_vertex(x + half_width, body_bottom, is_bullish),
                CandleVertex::body_vertex(x - half_width, actual_body_top, is_bullish),
                CandleVertex::body_vertex(x + half_width, body_bottom, is_bullish),
                CandleVertex::body_vertex(x + half_width, actual_body_top, is_bullish),
                CandleVertex::body_vertex(x - half_width, actual_body_top, is_bullish),
            ];
            vertices.extend_from_slice(&body_vertices);

            // Round corners with small triangles
            // Increase rounding for more pronounced candle corners
            let corner = candle_width * 0.35;
            let corners = vec![
                // Top left
                CandleVertex::body_vertex(x - half_width, actual_body_top - corner, is_bullish),
                CandleVertex::body_vertex(x - half_width + corner, actual_body_top, is_bullish),
                CandleVertex::body_vertex(x - half_width, actual_body_top, is_bullish),
                // Top right
                CandleVertex::body_vertex(x + half_width - corner, actual_body_top, is_bullish),
                CandleVertex::body_vertex(x + half_width, actual_body_top - corner, is_bullish),
                CandleVertex::body_vertex(x + half_width, actual_body_top, is_bullish),
                // Bottom left
                CandleVertex::body_vertex(x - half_width, body_bottom, is_bullish),
                CandleVertex::body_vertex(x - half_width + corner, body_bottom, is_bullish),
                CandleVertex::body_vertex(x - half_width, body_bottom + corner, is_bullish),
                // Bottom right
                CandleVertex::body_vertex(x + half_width, body_bottom, is_bullish),
                CandleVertex::body_vertex(x + half_width, body_bottom + corner, is_bullish),
                CandleVertex::body_vertex(x + half_width - corner, body_bottom, is_bullish),
            ];
            vertices.extend_from_slice(&corners);

            // Add wicks (upper and lower)
            let wick_width = candle_width * 0.1; // thin wicks
            let wick_half = wick_width * 0.5;

            // Upper wick
            if high_y > actual_body_top {
                let upper_wick = vec![
                    CandleVertex::wick_vertex(x - wick_half, actual_body_top),
                    CandleVertex::wick_vertex(x + wick_half, actual_body_top),
                    CandleVertex::wick_vertex(x - wick_half, high_y),
                    CandleVertex::wick_vertex(x + wick_half, actual_body_top),
                    CandleVertex::wick_vertex(x + wick_half, high_y),
                    CandleVertex::wick_vertex(x - wick_half, high_y),
                ];
                vertices.extend_from_slice(&upper_wick);
            }

            // Lower wick
            if low_y < body_bottom {
                let lower_wick = vec![
                    CandleVertex::wick_vertex(x - wick_half, low_y),
                    CandleVertex::wick_vertex(x + wick_half, low_y),
                    CandleVertex::wick_vertex(x - wick_half, body_bottom),
                    CandleVertex::wick_vertex(x + wick_half, low_y),
                    CandleVertex::wick_vertex(x + wick_half, body_bottom),
                    CandleVertex::wick_vertex(x - wick_half, body_bottom),
                ];
                vertices.extend_from_slice(&lower_wick);
            }
        }

        // Add a solid line for the current price
        if let Some(last_candle) = visible_candles.last() {
            let current_price = last_candle.ohlcv.close.value() as f32;
            let price_range = max_price - min_price;
            let price_y = -0.5 + ((current_price - min_price) / price_range) * 1.3; // same area as candles

            // Solid horizontal line across the entire screen
            let line_thickness = 0.002;
            let price_line = vec![
                CandleVertex::current_price_vertex(-1.0, price_y - line_thickness),
                CandleVertex::current_price_vertex(1.0, price_y - line_thickness),
                CandleVertex::current_price_vertex(-1.0, price_y + line_thickness),
                CandleVertex::current_price_vertex(1.0, price_y - line_thickness),
                CandleVertex::current_price_vertex(1.0, price_y + line_thickness),
                CandleVertex::current_price_vertex(-1.0, price_y + line_thickness),
            ];
            vertices.extend_from_slice(&price_line);
        }

        // Ichimoku cloud
        let ichimoku = &chart.ichimoku;
        if !ichimoku.senkou_span_a.is_empty() && !ichimoku.senkou_span_b.is_empty() {
            let span_len = ichimoku.senkou_span_a.len().min(ichimoku.senkou_span_b.len());
            let mut span_a_pts = Vec::new();
            let mut span_b_pts = Vec::new();
            for i in 0..span_len {
                let x = candle_x_position(i, visible_count);
                let y_a = -0.5
                    + ((ichimoku.senkou_span_a[i].value() as f32 - min_price) / price_range) * 1.3;
                let y_b = -0.5
                    + ((ichimoku.senkou_span_b[i].value() as f32 - min_price) / price_range) * 1.3;
                span_a_pts.push((x, y_a));
                span_b_pts.push((x, y_b));
            }
            vertices.extend(CandleGeometry::create_ichimoku_cloud(&span_a_pts, &span_b_pts, 0.002));
        }

        // Identity matrix - vertices are already in NDC coordinates [-1, 1]
        let view_proj_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        // Create uniforms with corrected parameters
        let uniforms = ChartUniforms {
            view_proj_matrix,
            viewport: [self.width as f32, self.height as f32, min_price, max_price],
            time_range: [0.0, visible_candles.len() as f32, visible_candles.len() as f32, 0.0],
            bullish_color: [0.455, 0.780, 0.529, 1.0], // #74c787 - green
            bearish_color: [0.882, 0.424, 0.282, 1.0], // #e16c48 - red
            wick_color: [0.6, 0.6, 0.6, 0.9],          // light gray
            sma20_color: [1.0, 0.2, 0.2, 0.9],         // bright red
            sma50_color: [1.0, 0.8, 0.0, 0.9],         // yellow
            sma200_color: [0.2, 0.4, 0.8, 0.9],        // blue
            ema12_color: [0.8, 0.2, 0.8, 0.9],         // purple
            ema26_color: [0.0, 0.8, 0.8, 0.9],         // cyan
            current_price_color: [1.0, 1.0, 0.0, 0.8], // 💰 bright yellow
            render_params: [candle_width, spacing_ratio as f32, 0.004, 0.0],
        };

        (instances, vertices, uniforms)
    }
}
