use super::*;
use crate::domain::logging::{LogComponent, get_logger};
use crate::domain::market_data::Price;
use crate::domain::market_data::services::MarketAnalysisService;
use crate::infrastructure::rendering::gpu_structures::{
    CandleGeometry, CandleInstance, IndicatorType,
};
use leptos::SignalGetUntracked;

/// Minimum element width (candle or volume bar)
pub const MIN_ELEMENT_WIDTH: f32 = 0.002;
/// Maximum element width (candle or volume bar)
pub const MAX_ELEMENT_WIDTH: f32 = 0.1;
/// Ratio of space left empty between elements
pub const SPACING_RATIO: f32 = 0.2;

/// Dynamic spacing based on number of visible candles
pub fn spacing_ratio_for(visible_len: usize) -> f32 {
    assert!(visible_len > 0, "visible_len must be > 0");
    let factor = (visible_len as f32 / 100.0).min(1.0);
    SPACING_RATIO * factor
}

/// Candle/bar position taking right edge into account
pub fn candle_x_position(index: usize, visible_len: usize) -> f32 {
    assert!(visible_len > 0, "visible_len must be > 0");
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

        let price_range = (max_price - min_price).abs().max(1e-6);
        min_price -= price_range * 0.05;
        max_price += price_range * 0.05;

        // Log estimated candle width using the number of visible candles
        let step_size = chart_width / visible_candles.len() as f64;
        let candle_width_estimate =
            step_size * (1.0 - spacing_ratio_for(visible_candles.len()) as f64);

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!(
                "📏 Price range: {:.2} - {:.2}, Candle width: {:.4}, step:{:.4}",
                min_price, max_price, candle_width_estimate, step_size
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
        let spacing = spacing_ratio_for(visible_candles.len());
        let candle_width =
            (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
        let mut instances = Vec::with_capacity(visible_candles.len());

        let price_range = max_price - min_price;
        let price_norm = |price: f64| -> f32 {
            let normalized = (price as f32 - min_price) / price_range;
            normalized * 2.0 - 1.0
        };

        for (i, candle) in visible_candles.iter().enumerate() {
            let x = candle_x_position(i, visible_candles.len());

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

            let candle_vertices = CandleGeometry::create_candle_vertices(
                candle.timestamp.as_f64(),
                candle.ohlcv.open.value() as f32,
                candle.ohlcv.high.value() as f32,
                candle.ohlcv.low.value() as f32,
                candle.ohlcv.close.value() as f32,
                x,
                open_y,
                high_y,
                low_y,
                close_y,
                candle_width,
            );
            vertices.extend_from_slice(&candle_vertices);
        }

        // Calculate moving averages for indicator lines using the full data set
        let analysis = MarketAnalysisService::new();
        let mas = analysis.calculate_multiple_mas(&candle_vec);

        let to_points = |values: &[Price], period: usize| -> Vec<(f32, f32)> {
            values
                .iter()
                .enumerate()
                .filter_map(|(idx, val)| {
                    let candle_idx = idx + period - 1;
                    if candle_idx < start_index || candle_idx >= start_index + visible_candles.len()
                    {
                        return None;
                    }
                    let x = candle_x_position(candle_idx - start_index, visible_candles.len());
                    let y = price_norm(val.value());
                    Some((x, y))
                })
                .collect()
        };

        let line_width = 0.004;

        if self.line_visibility.sma_20 {
            let points = to_points(&mas.sma_20, 20);
            vertices.extend_from_slice(&CandleGeometry::create_indicator_line_vertices(
                &points,
                IndicatorType::SMA20,
                line_width,
            ));
        }

        if self.line_visibility.sma_50 {
            let points = to_points(&mas.sma_50, 50);
            vertices.extend_from_slice(&CandleGeometry::create_indicator_line_vertices(
                &points,
                IndicatorType::SMA50,
                line_width,
            ));
        }

        if self.line_visibility.sma_200 {
            let points = to_points(&mas.sma_200, 200);
            vertices.extend_from_slice(&CandleGeometry::create_indicator_line_vertices(
                &points,
                IndicatorType::SMA200,
                line_width,
            ));
        }

        if self.line_visibility.ema_12 {
            let points = to_points(&mas.ema_12, 12);
            vertices.extend_from_slice(&CandleGeometry::create_indicator_line_vertices(
                &points,
                IndicatorType::EMA12,
                line_width,
            ));
        }

        if self.line_visibility.ema_26 {
            let points = to_points(&mas.ema_26, 26);
            vertices.extend_from_slice(&CandleGeometry::create_indicator_line_vertices(
                &points,
                IndicatorType::EMA26,
                line_width,
            ));
        }

        // Add a solid line for the current price
        if let Some(last_candle) = visible_candles.last() {
            let current_price = last_candle.ohlcv.close.value() as f32;
            let price_range = max_price - min_price;
            let price_y = ((current_price - min_price) / price_range) * 2.0 - 1.0; // same area as candles

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
                let y_a = ((ichimoku.senkou_span_a[i].value() as f32 - min_price) / price_range)
                    * 2.0
                    - 1.0;
                let y_b = ((ichimoku.senkou_span_b[i].value() as f32 - min_price) / price_range)
                    * 2.0
                    - 1.0;
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
            render_params: [candle_width, spacing, 0.004, 0.0],
        };

        (instances, vertices, uniforms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        chart::{Chart, value_objects::ChartType},
        market_data::{Candle, OHLCV, Price, Timestamp, Volume},
    };
    use std::collections::VecDeque;

    #[allow(invalid_value)]
    fn dummy_renderer() -> WebGpuRenderer {
        unsafe {
            WebGpuRenderer {
                _canvas_id: String::new(),
                width: 800,
                height: 600,
                surface: std::mem::MaybeUninit::zeroed().assume_init(),
                device: std::mem::MaybeUninit::zeroed().assume_init(),
                queue: std::mem::MaybeUninit::zeroed().assume_init(),
                config: std::mem::MaybeUninit::zeroed().assume_init(),
                render_pipeline: std::mem::MaybeUninit::zeroed().assume_init(),
                vertex_buffer: std::mem::MaybeUninit::zeroed().assume_init(),
                uniform_buffer: std::mem::MaybeUninit::zeroed().assume_init(),
                uniform_bind_group: std::mem::MaybeUninit::zeroed().assume_init(),
                template_vertices: 0,
                cached_vertices: Vec::new(),
                cached_uniforms: ChartUniforms::new(),
                cached_candle_count: 0,
                cached_zoom_level: 1.0,
                cached_hash: 0,
                cached_data_hash: 0,
                zoom_level: 1.0,
                pan_offset: 0.0,
                last_frame_time: 0.0,
                fps_log: VecDeque::new(),
                line_visibility: LineVisibility::default(),
            }
        }
    }

    fn make_candle(i: u64) -> Candle {
        let base = 100.0 + i as f64;
        Candle::new(
            Timestamp::from_millis(i * 60_000),
            OHLCV::new(
                Price::from(base),
                Price::from(base + 1.0),
                Price::from(base - 1.0),
                Price::from(base),
                Volume::from(1.0),
            ),
        )
    }

    #[test]
    fn indicator_vertices_present() {
        let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 300);
        let candles: Vec<Candle> = (0..210).map(make_candle).collect();
        chart.set_historical_data(candles);

        let renderer = dummy_renderer();
        let (_, verts, _) = renderer.create_geometry(&chart);

        assert!(verts.iter().any(|v| (v.color_type - 2.0).abs() < f32::EPSILON));
        assert!(verts.iter().any(|v| (v.color_type - 3.0).abs() < f32::EPSILON));
        assert!(verts.iter().any(|v| (v.color_type - 4.0).abs() < f32::EPSILON));
        assert!(verts.iter().any(|v| (v.color_type - 5.0).abs() < f32::EPSILON));
        assert!(verts.iter().any(|v| (v.color_type - 6.0).abs() < f32::EPSILON));
    }

    #[test]
    fn candle_height_and_color() {
        let candles = vec![
            Candle::new(
                Timestamp::from_millis(0),
                OHLCV::new(
                    Price::from(100.0),
                    Price::from(101.0),
                    Price::from(99.0),
                    Price::from(101.0),
                    Volume::from(1.0),
                ),
            ),
            Candle::new(
                Timestamp::from_millis(60_000),
                OHLCV::new(
                    Price::from(101.0),
                    Price::from(102.0),
                    Price::from(100.0),
                    Price::from(100.5),
                    Volume::from(1.0),
                ),
            ),
            Candle::new(
                Timestamp::from_millis(120_000),
                OHLCV::new(
                    Price::from(100.5),
                    Price::from(100.6),
                    Price::from(100.4),
                    Price::from(100.5),
                    Volume::from(1.0),
                ),
            ),
        ];

        let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 50);
        chart.set_historical_data(candles);

        let renderer = dummy_renderer();
        let (instances, _verts, _uni) = renderer.create_geometry(&chart);

        assert_eq!(instances.len(), 3);
        assert!(instances[0].bullish > 0.5);
        assert!(instances[1].bullish < 0.5);
        assert!(instances[2].body_top - instances[2].body_bottom >= 0.005 - f32::EPSILON);
    }

    #[test]
    fn moving_averages_from_full_data() {
        let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 300);
        let candles: Vec<Candle> = (0..250).map(make_candle).collect();
        chart.set_historical_data(candles.clone());

        let renderer = dummy_renderer();
        let (_, verts, _) = renderer.create_geometry(&chart);

        let (start_index, visible_count) =
            crate::app::visible_range_by_time(&candles, &chart.viewport, renderer.zoom_level);
        let visible: Vec<Candle> =
            candles.iter().skip(start_index).take(visible_count).cloned().collect();

        let mut min_price = f32::INFINITY;
        let mut max_price = f32::NEG_INFINITY;
        for c in &visible {
            min_price = min_price.min(c.ohlcv.low.value() as f32);
            max_price = max_price.max(c.ohlcv.high.value() as f32);
        }
        let pr = max_price - min_price;
        min_price -= pr * 0.05;
        max_price += pr * 0.05;
        let price_norm =
            |p: f64| -> f32 { ((p as f32 - min_price) / (max_price - min_price)) * 2.0 - 1.0 };

        let analysis = MarketAnalysisService::new();
        let mas = analysis.calculate_multiple_mas(&candles);

        let to_points = |vals: &[Price], period: usize| -> Vec<(f32, f32)> {
            vals.iter()
                .enumerate()
                .filter_map(|(idx, v)| {
                    let ci = idx + period - 1;
                    if ci < start_index || ci >= start_index + visible_count {
                        return None;
                    }
                    let x = candle_x_position(ci - start_index, visible_count);
                    let y = price_norm(v.value());
                    Some((x, y))
                })
                .collect()
        };

        let line_width = 0.004;
        let checks = [
            (&mas.sma_20, IndicatorType::SMA20, 2.0, 20usize),
            (&mas.sma_50, IndicatorType::SMA50, 3.0, 50usize),
            (&mas.sma_200, IndicatorType::SMA200, 4.0, 200usize),
            (&mas.ema_12, IndicatorType::EMA12, 5.0, 12usize),
            (&mas.ema_26, IndicatorType::EMA26, 6.0, 26usize),
        ];

        for (values, t, color, period) in checks {
            let pts = to_points(values, period);
            let expected = CandleGeometry::create_indicator_line_vertices(&pts, t, line_width);
            let actual: Vec<CandleVertex> = verts
                .iter()
                .filter(|v| (v.color_type - color).abs() < f32::EPSILON)
                .cloned()
                .collect();
            assert_eq!(actual.len(), expected.len());
            for (a, e) in actual.iter().zip(expected.iter()) {
                assert!((a.position_x - e.position_x).abs() < 1e-6);
                assert!((a.position_y - e.position_y).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn price_normalization_range() {
        let candles = vec![
            Candle::new(
                Timestamp::from_millis(0),
                OHLCV::new(
                    Price::from(100.0),
                    Price::from(110.0),
                    Price::from(90.0),
                    Price::from(105.0),
                    Volume::from(1.0),
                ),
            ),
            Candle::new(
                Timestamp::from_millis(60_000),
                OHLCV::new(
                    Price::from(105.0),
                    Price::from(108.0),
                    Price::from(100.0),
                    Price::from(107.0),
                    Volume::from(1.0),
                ),
            ),
            Candle::new(
                Timestamp::from_millis(120_000),
                OHLCV::new(
                    Price::from(107.0),
                    Price::from(109.0),
                    Price::from(106.0),
                    Price::from(108.0),
                    Volume::from(1.0),
                ),
            ),
        ];

        let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 50);
        chart.set_historical_data(candles);

        let renderer = dummy_renderer();
        let (instances, _verts, _uni) = renderer.create_geometry(&chart);

        assert_eq!(instances.len(), 3);

        let mut min_v = f32::INFINITY;
        let mut max_v = f32::NEG_INFINITY;
        for inst in &instances {
            for v in [inst.high, inst.low, inst.body_top, inst.body_bottom] {
                assert!((-1.0..=1.0).contains(&v));
                if v < min_v {
                    min_v = v;
                }
                if v > max_v {
                    max_v = v;
                }
            }
        }

        assert!((min_v + 1.0).abs() < 0.1);
        assert!((max_v - 1.0).abs() < 0.1);
    }
}
