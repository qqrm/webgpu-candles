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
    pub(super) fn create_geometry(&self, chart: &Chart) -> (Vec<CandleVertex>, ChartUniforms) {
        use crate::app::current_interval;

        let interval = current_interval().get_untracked();
        let candles = chart
            .get_series(interval)
            .map(|s| s.get_candles())
            .unwrap_or_else(|| chart.get_series_for_zoom(self.zoom_level).get_candles());

        if candles.is_empty() {
            get_logger()
                .error(LogComponent::Infrastructure("WebGpuRenderer"), "⚠️ No candles to render");

            return (vec![], ChartUniforms::new());
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
        let _chart_height = 2.0; // NDC height (-1 to 1)

        // 🔍 Apply zoom - show fewer candles when zooming in
        let (start_index, visible_count) =
            crate::app::visible_range(candle_count, self.zoom_level, self.pan_offset);
        let visible_candles: Vec<Candle> =
            candles.iter().skip(start_index).take(visible_count).cloned().collect();

        let mut vertices = Vec::with_capacity(visible_candles.len() * 24);

        // Use viewport values for vertical panning
        let mut min_price = chart.viewport.min_price;
        let mut max_price = chart.viewport.max_price;
        if (max_price - min_price).abs() < f32::EPSILON {
            // If the range is zero, calculate it from data
            for candle in &visible_candles {
                min_price = min_price.min(candle.ohlcv.low.value() as f32);
                max_price = max_price.max(candle.ohlcv.high.value() as f32);
            }

            let price_range = max_price - min_price;
            min_price -= price_range * 0.05;
            max_price += price_range * 0.05;
        }

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
            return (vec![], ChartUniforms::new());
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

        // Create vertices for each visible candle
        let step_size = 2.0 / visible_candles.len() as f32;
        let candle_width = (step_size * 0.8).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

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

            let half_width = candle_width * 0.5;
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
            let corner = candle_width * 0.2;
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

        // 📊 Add chart grid for a professional look
        vertices.extend(self.create_grid_lines(min_price, max_price, visible_candles.len()));

        // 📊 Add volume bars below the chart
        vertices.extend(self.create_volume_bars(&visible_candles));

        // 📈 Add moving averages (SMA20 and EMA12)
        vertices.extend(self.create_moving_averages(&visible_candles, min_price, max_price));

        // Log only when there are many vertices
        if vertices.len() > 1000 {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!(
                    "✅ Generated {} vertices for {} visible candles + indicators",
                    vertices.len(),
                    visible_candles.len()
                ),
            );
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

        (vertices, uniforms)
    }

    /// 📈 Create geometry for moving averages
    fn create_moving_averages(
        &self,
        candles: &[crate::domain::market_data::Candle],
        min_price: f32,
        max_price: f32,
    ) -> Vec<CandleVertex> {
        use crate::infrastructure::rendering::gpu_structures::{CandleGeometry, IndicatorType};

        if candles.len() < 20 {
            return Vec::new(); // Not enough data for SMA20
        }

        let mut vertices = Vec::with_capacity(candles.len() * 6);
        let candle_count = candles.len();

        let price_range = max_price - min_price;

        // Helper to normalize price to NDC coordinates
        let price_to_ndc = |price: f32| -> f32 { -0.8 + ((price - min_price) / price_range) * 1.6 };

        // Calculate SMA20 (Simple Moving Average 20)
        let mut sma20_points = Vec::with_capacity(candles.len().saturating_sub(19));
        for i in 19..candle_count {
            // Start from the 20th candle
            let sum: f32 = candles[i - 19..=i].iter().map(|c| c.ohlcv.close.value() as f32).sum();
            let sma20 = sum / 20.0;
            let x = candle_x_position(i, candle_count);
            let y = price_to_ndc(sma20);
            sma20_points.push((x, y));
        }

        // Calculate EMA12 (Exponential Moving Average 12)
        let mut ema12_points = Vec::with_capacity(candles.len().saturating_sub(11));
        if candle_count >= 12 {
            let multiplier = 2.0 / (12.0 + 1.0); // EMA multiplier
            let mut ema = candles[0].ohlcv.close.value() as f32; // initial value

            for (i, candle) in candles.iter().enumerate().skip(1) {
                let close = candle.ohlcv.close.value() as f32;
                ema = (close * multiplier) + (ema * (1.0 - multiplier));

                if i >= 11 {
                    // Show EMA only after 12 candles
                    let x = candle_x_position(i, candle_count);
                    let y = price_to_ndc(ema);
                    ema12_points.push((x, y));
                }
            }
        }

        // Build geometry for the lines
        if !sma20_points.is_empty() {
            let sma20_vertices = CandleGeometry::create_indicator_line_vertices(
                &sma20_points,
                IndicatorType::SMA20,
                0.003, // line thickness
            );
            vertices.extend(sma20_vertices);
        }

        if !ema12_points.is_empty() {
            let ema12_vertices = CandleGeometry::create_indicator_line_vertices(
                &ema12_points,
                IndicatorType::EMA12,
                0.003, // line thickness
            );
            vertices.extend(ema12_vertices);
        }

        if !vertices.is_empty() {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!(
                    "📈 Generated {} SMA20 points, {} EMA12 points, {} total MA vertices",
                    sma20_points.len(),
                    ema12_points.len(),
                    vertices.len()
                ),
            );
        }

        vertices
    }

    /// 📊 Create chart grid (horizontal and vertical lines)
    fn create_grid_lines(
        &self,
        min_price: f32,
        max_price: f32,
        candle_count: usize,
    ) -> Vec<CandleVertex> {
        let num_price_lines = 8; // 8 horizontal lines
        let num_vertical_lines = 10; // 10 vertical lines
        let mut vertices = Vec::with_capacity((num_price_lines + num_vertical_lines) * 6);
        let line_thickness = 0.001; // thin grid lines

        // Horizontal grid lines (price levels)
        let price_range = max_price - min_price;

        for i in 1..num_price_lines {
            let price_level = min_price + (price_range * i as f32 / num_price_lines as f32);
            let y = -0.5 + ((price_level - min_price) / price_range) * 1.3; // same area as candles

            // Horizontal line across the entire chart
            let horizontal_line = vec![
                CandleVertex::grid_vertex(-1.0, y - line_thickness),
                CandleVertex::grid_vertex(1.0, y - line_thickness),
                CandleVertex::grid_vertex(-1.0, y + line_thickness),
                CandleVertex::grid_vertex(1.0, y - line_thickness),
                CandleVertex::grid_vertex(1.0, y + line_thickness),
                CandleVertex::grid_vertex(-1.0, y + line_thickness),
            ];
            vertices.extend_from_slice(&horizontal_line);
        }

        // Vertical grid lines (time intervals) covering the whole chart
        if candle_count > 0 {
            let num_vertical_lines = 10; // 10 vertical lines
            let vertical_step = candle_count / num_vertical_lines;

            for i in 1..num_vertical_lines {
                let candle_index = i * vertical_step;
                if candle_index < candle_count {
                    let x = candle_x_position(candle_index, candle_count);

                    // Vertical line through the entire chart (including volume area)
                    let vertical_line = vec![
                        CandleVertex::grid_vertex(x - line_thickness, -1.0), // from bottom
                        CandleVertex::grid_vertex(x + line_thickness, -1.0),
                        CandleVertex::grid_vertex(x - line_thickness, 0.8), // to top of candles
                        CandleVertex::grid_vertex(x + line_thickness, -1.0),
                        CandleVertex::grid_vertex(x + line_thickness, 0.8),
                        CandleVertex::grid_vertex(x - line_thickness, 0.8),
                    ];
                    vertices.extend_from_slice(&vertical_line);
                }
            }
        }

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("📊 Generated {} grid vertices", vertices.len()),
        );

        vertices
    }

    /// 📊 Create volume bars below the main chart
    fn create_volume_bars(
        &self,
        candles: &[crate::domain::market_data::Candle],
    ) -> Vec<CandleVertex> {
        if candles.is_empty() {
            return Vec::new();
        }

        let candle_count = candles.len();
        let mut vertices = Vec::with_capacity(candle_count * 6);

        // Find the maximum volume for normalization
        let max_volume =
            candles.iter().map(|c| c.ohlcv.volume.value() as f32).fold(0.0f32, |a, b| a.max(b));

        if max_volume <= 0.0 {
            return Vec::new();
        }

        // Volume area occupies the lower part of the screen [-1.0, -0.6]
        let volume_top = -0.6;
        let volume_bottom = -1.0;
        let volume_height = volume_top - volume_bottom;

        let step_size = 2.0 / candle_count as f32;
        let bar_width = (step_size * 0.8).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

        for (i, candle) in candles.iter().enumerate() {
            let x = candle_x_position(i, candle_count);
            let volume_normalized = (candle.ohlcv.volume.value() as f32) / max_volume;
            let bar_height = volume_height * volume_normalized;
            let bar_top = volume_bottom + bar_height;

            let half_width = bar_width * 0.5;

            // Determine volume bar color: green if price rose, red if it fell
            let is_bullish = candle.ohlcv.close.value() >= candle.ohlcv.open.value();

            // Volume bar as a rectangle (2 triangles)
            let volume_bar = vec![
                CandleVertex::volume_vertex(x - half_width, volume_bottom, is_bullish),
                CandleVertex::volume_vertex(x + half_width, volume_bottom, is_bullish),
                CandleVertex::volume_vertex(x - half_width, bar_top, is_bullish),
                CandleVertex::volume_vertex(x + half_width, volume_bottom, is_bullish),
                CandleVertex::volume_vertex(x + half_width, bar_top, is_bullish),
                CandleVertex::volume_vertex(x - half_width, bar_top, is_bullish),
            ];
            vertices.extend_from_slice(&volume_bar);
        }

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!(
                "📊 Generated {} volume vertices for {} candles (max volume: {:.2})",
                vertices.len(),
                candles.len(),
                max_volume
            ),
        );

        vertices
    }
}
