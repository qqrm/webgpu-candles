use super::*;
use crate::domain::logging::{LogComponent, get_logger};
use crate::domain::market_data::{Candle, OHLCV, Price, TimeInterval, Timestamp, Volume};
use crate::infrastructure::rendering::gpu_structures::{
    CURRENT_PRICE_COLOR, CandleGeometry, CandleInstance, EMA12_COLOR, EMA26_COLOR, IndicatorType,
    SMA20_COLOR, SMA50_COLOR, SMA200_COLOR,
};
use leptos::SignalGetUntracked;

/// Minimum element width (candle or volume bar)
pub const MIN_ELEMENT_WIDTH: f32 = 0.0001;
/// Maximum element width (candle or volume bar)
pub const MAX_ELEMENT_WIDTH: f32 = 0.24;
/// Minimum visible candle body height in physical canvas pixels.
pub const MIN_CANDLE_BODY_PX: f32 = 2.0;
/// Ratio of space left empty between elements
pub const SPACING_RATIO: f32 = 0.2;
/// Gap between the right edge and the last element
pub const EDGE_GAP: f32 = 0.003;
/// Keep overview geometry near one logical bar per two physical pixels.
pub const PIXELS_PER_RENDERED_CANDLE: usize = 2;

#[derive(Debug, Clone)]
struct RenderCandle {
    candle: Candle,
    source_index: usize,
}

/// Number of candles sent to the GPU after level-of-detail aggregation.
pub fn lod_candle_count(visible_count: usize, canvas_width: u32) -> usize {
    let pixel_budget = (canvas_width as usize / PIXELS_PER_RENDERED_CANDLE).clamp(128, 4096);
    visible_count.min(pixel_budget)
}

fn build_render_candles(
    candles: &std::collections::VecDeque<Candle>,
    start_index: usize,
    visible_count: usize,
    canvas_width: u32,
) -> Vec<RenderCandle> {
    let target = lod_candle_count(visible_count, canvas_width);
    if target == 0 {
        return Vec::new();
    }
    if target == visible_count {
        return candles
            .iter()
            .skip(start_index)
            .take(visible_count)
            .enumerate()
            .map(|(offset, candle)| RenderCandle {
                candle: candle.clone(),
                source_index: start_index + offset,
            })
            .collect();
    }

    let end_index = (start_index + visible_count).min(candles.len());
    let mut result = Vec::with_capacity(target);
    for bucket in 0..target {
        let bucket_start = start_index + bucket * visible_count / target;
        let bucket_end = (start_index + (bucket + 1) * visible_count / target).min(end_index);
        if bucket_start >= bucket_end {
            continue;
        }
        let first = &candles[bucket_start];
        let last = &candles[bucket_end - 1];
        let mut high = first.ohlcv.high.value();
        let mut low = first.ohlcv.low.value();
        let mut volume = 0.0;
        for candle in candles.iter().skip(bucket_start).take(bucket_end - bucket_start) {
            high = high.max(candle.ohlcv.high.value());
            low = low.min(candle.ohlcv.low.value());
            volume += candle.ohlcv.volume.value();
        }
        result.push(RenderCandle {
            candle: Candle::new(
                Timestamp::from_millis(first.timestamp.value()),
                OHLCV::new(
                    first.ohlcv.open,
                    Price::from(high),
                    Price::from(low),
                    last.ohlcv.close,
                    Volume::from(volume),
                ),
            ),
            source_index: bucket_end - 1,
        });
    }
    result
}

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
    let spacing = spacing_ratio_for(visible_len);
    let width = (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
    let base_x = 1.0 - (visible_len as f32 - index as f32 - 1.0) * step_size;
    base_x - width / 2.0 - EDGE_GAP
}

impl WebGpuRenderer {
    /// Convert pixel size to normalized device coordinates
    fn px_to_ndc(&self, px: f32) -> f32 {
        (px / self.height as f32) * 2.0
    }
    pub(super) fn create_geometry(
        &self,
        chart: &Chart,
    ) -> (Vec<CandleInstance>, Vec<CandleVertex>, ChartUniforms) {
        use crate::app::current_interval;

        let interval = current_interval().get_untracked();
        let candles = chart.get_series(interval).map(|s| s.get_candles()).unwrap_or_else(|| {
            chart.get_series(TimeInterval::TwoSeconds).expect("base series not found").get_candles()
        });

        if candles.is_empty() {
            get_logger()
                .error(LogComponent::Infrastructure("WebGpuRenderer"), "⚠️ No candles to render");

            return (Vec::new(), Vec::new(), ChartUniforms::new());
        }

        // 🔍 Apply zoom - show fewer candles when zooming in
        let (start_index, visible_count) =
            crate::app::visible_range_by_time_deque(candles, &chart.viewport, self.zoom_level);
        let visible_candles = build_render_candles(candles, start_index, visible_count, self.width);

        let mut vertices = Vec::with_capacity(visible_candles.len() * 24);

        // Moving averages precomputed in chart's indicator engines
        let engine = chart
            .ma_engines
            .get(&interval)
            .or_else(|| chart.ma_engines.get(&TimeInterval::TwoSeconds))
            .expect("engine not found");
        let mas = engine.data();

        // Scale candles based on currently visible data and indicator values
        let mut min_price = f32::INFINITY;
        let mut max_price = f32::NEG_INFINITY;
        for item in &visible_candles {
            min_price = min_price.min(item.candle.ohlcv.low.value() as f32);
            max_price = max_price.max(item.candle.ohlcv.high.value() as f32);
        }

        // Flat and near-flat 2s windows are valid market states. Expand only the
        // visual range instead of dropping the whole frame.
        let price_mid = (max_price + min_price) * 0.5;
        let minimum_range = (price_mid.abs() * 1e-6).max(1e-6);
        let display_range = (max_price - min_price).abs().max(minimum_range) * 1.1;
        min_price = (price_mid - display_range * 0.5).max(0.0);
        max_price = price_mid + display_range * 0.5;

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

        let mut max_volume = 0.0f32;
        for item in &visible_candles {
            max_volume = max_volume.max(item.candle.ohlcv.volume.value() as f32);
        }
        if max_volume <= 0.0 {
            max_volume = 1.0;
        }

        for (i, item) in visible_candles.iter().enumerate() {
            let candle = &item.candle;
            let x = candle_x_position(i, visible_candles.len());

            let open_y = price_norm(candle.ohlcv.open.value());
            let high_y = price_norm(candle.ohlcv.high.value());
            let low_y = price_norm(candle.ohlcv.low.value());
            let close_y = price_norm(candle.ohlcv.close.value());

            let is_bullish = candle.is_bullish();
            let body_mid = (open_y + close_y) * 0.5;
            let body_height = (close_y - open_y).abs().max(self.px_to_ndc(MIN_CANDLE_BODY_PX));
            let body_half = body_height * 0.5;
            let (visible_open_y, visible_close_y) = if is_bullish {
                (body_mid - body_half, body_mid + body_half)
            } else {
                (body_mid + body_half, body_mid - body_half)
            };
            let body_top = visible_open_y.max(visible_close_y);
            let body_bottom = visible_open_y.min(visible_close_y);

            instances.push(CandleInstance {
                x,
                width: candle_width,
                body_top,
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
                visible_open_y,
                high_y,
                low_y,
                visible_close_y,
                candle_width,
            );
            vertices.extend_from_slice(&candle_vertices);

            let vol_ratio = (candle.ohlcv.volume.value() as f32) / max_volume;
            let volume_vertices =
                CandleGeometry::create_volume_vertices(x, candle_width, vol_ratio, is_bullish);
            vertices.extend_from_slice(&volume_vertices);
        }

        let to_points = |values: &[Price], period: usize| -> Vec<(f32, f32)> {
            visible_candles
                .iter()
                .enumerate()
                .filter_map(|(display_index, item)| {
                    let value_index = item.source_index.checked_sub(period - 1)?;
                    let val = values.get(value_index)?;
                    let x = candle_x_position(display_index, visible_candles.len());
                    let y = price_norm(val.value());
                    Some((x, y))
                })
                .collect()
        };

        let line_width = self.px_to_ndc(2.0);

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
        if !visible_candles.is_empty() {
            let current_price = crate::app::global_current_price().get_untracked() as f32;
            let price_y = ((current_price - min_price) / price_range) * 2.0 - 1.0; // same area as candles

            // Keep the line width constant regardless of zoom level
            let line_thickness = 2.0 / self.height as f32;

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
            let cloud_width = self.px_to_ndc(2.0);
            vertices.extend(CandleGeometry::create_ichimoku_cloud(
                &span_a_pts,
                &span_b_pts,
                cloud_width,
            ));
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
            sma20_color: SMA20_COLOR,
            sma50_color: SMA50_COLOR,
            sma200_color: SMA200_COLOR,
            ema12_color: EMA12_COLOR,
            ema26_color: EMA26_COLOR,
            current_price_color: CURRENT_PRICE_COLOR,
            render_params: [candle_width, spacing, line_width, 0.0],
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
    use leptos::SignalSet;
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
                msaa_texture: std::mem::MaybeUninit::zeroed().assume_init(),
                msaa_view: std::mem::MaybeUninit::zeroed().assume_init(),
                template_vertices: 0,
                cached_vertices: Vec::new(),
                cached_uniforms: ChartUniforms::new(),
                cached_candle_count: 0,
                cached_zoom_level: 1.0,
                cached_hash: 0,
                cached_data_revision: 0,
                cached_line_visibility: LineVisibility::default(),
                zoom_level: 1.0,
                pan_offset: 0.0,
                render_time_log: VecDeque::new(),
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
        let (instances, verts, _uni) = renderer.create_geometry(&chart);

        assert_eq!(instances.len(), 3);
        assert!(instances[0].bullish > 0.5);
        assert!(instances[1].bullish < 0.5);
        let minimum_ndc_height = renderer.px_to_ndc(MIN_CANDLE_BODY_PX);
        assert!(
            instances[2].body_top - instances[2].body_bottom >= minimum_ndc_height - f32::EPSILON
        );
        let tiny = instances[2];
        let body_vertices: Vec<_> = verts
            .iter()
            .filter(|vertex| {
                vertex.element_type == 0.0
                    && (vertex.position_x - tiny.x).abs() <= tiny.width * 0.5 + f32::EPSILON
            })
            .map(|vertex| vertex.position_y)
            .collect();
        let body_min = body_vertices.iter().copied().fold(f32::INFINITY, f32::min);
        let body_max = body_vertices.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        assert!(body_max - body_min >= minimum_ndc_height - f32::EPSILON);
    }

    #[test]
    fn flat_market_still_produces_visible_geometry() {
        let candles: Vec<Candle> = (0..32)
            .map(|i| {
                Candle::new(
                    Timestamp::from_millis(i * 2_000),
                    OHLCV::new(
                        Price::from(100.0),
                        Price::from(100.0),
                        Price::from(100.0),
                        Price::from(100.0),
                        Volume::from(1.0),
                    ),
                )
            })
            .collect();
        let mut chart = Chart::new("flat".to_string(), ChartType::Candlestick, 50);
        chart.set_historical_data(candles);

        let renderer = dummy_renderer();
        let (instances, vertices, uniforms) = renderer.create_geometry(&chart);

        assert_eq!(instances.len(), 32);
        assert!(!vertices.is_empty());
        assert!(uniforms.viewport[3] > uniforms.viewport[2]);
        assert!(instances.iter().all(|instance| {
            instance.body_top - instance.body_bottom
                >= renderer.px_to_ndc(MIN_CANDLE_BODY_PX) - f32::EPSILON
        }));
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

        let engine = chart.ma_engines.get(&TimeInterval::TwoSeconds).unwrap();
        let mas = engine.data();

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

        let line_width = renderer.px_to_ndc(2.0);
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

    #[test]
    fn indicator_visibility_does_not_change_price_scale() {
        // 30 candles: first 20 around 100, last 10 around 200
        let candles: Vec<Candle> = (0..30)
            .map(|i| {
                let base = if i < 20 { 100.0 } else { 200.0 };
                Candle::new(
                    Timestamp::from_millis(i as u64 * 60_000),
                    OHLCV::new(
                        Price::from(base),
                        Price::from(base + 1.0),
                        Price::from(base - 1.0),
                        Price::from(base),
                        Volume::from(1.0),
                    ),
                )
            })
            .collect();

        let mut chart = Chart::new("t".to_string(), ChartType::Candlestick, 300);
        chart.set_historical_data(candles.clone());

        let mut renderer = dummy_renderer();
        renderer.zoom_level = 3.0; // show only last ~10 candles
        let (_, _, with_indicators) = renderer.create_geometry(&chart);
        renderer.line_visibility = LineVisibility {
            sma_20: false,
            sma_50: false,
            sma_200: false,
            ema_12: false,
            ema_26: false,
        };
        let (_, _, without_indicators) = renderer.create_geometry(&chart);

        assert_eq!(with_indicators.viewport[2..4], without_indicators.viewport[2..4]);
    }

    #[test]
    fn max_zoom_keeps_candles_dense() {
        let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 100);
        chart.set_historical_data((0..100).map(make_candle).collect());
        let mut renderer = dummy_renderer();
        renderer.zoom_level = crate::app::MAX_ZOOM_LEVEL;

        let (instances, _, _) = renderer.create_geometry(&chart);

        assert_eq!(instances.len(), crate::app::MIN_VISIBLE_CANDLES as usize);
        assert!(instances.iter().all(|instance| instance.width >= 0.2));
    }

    #[test]
    fn lod_caps_geometry_and_preserves_bucket_extremes() {
        let mut candles: VecDeque<Candle> = (0..10_000).map(make_candle).collect();
        candles[4_999].ohlcv.high = Price::from(50_000.0);
        candles[5_000].ohlcv.low = Price::from(1.0);

        let rendered = build_render_candles(&candles, 0, candles.len(), 1600);

        assert_eq!(rendered.len(), 800);
        assert_eq!(rendered.first().unwrap().candle.ohlcv.open, candles[0].ohlcv.open);
        assert_eq!(rendered.last().unwrap().candle.ohlcv.close, candles[9_999].ohlcv.close);
        assert!(rendered.iter().any(|item| item.candle.ohlcv.high.value() == 50_000.0));
        assert!(rendered.iter().any(|item| item.candle.ohlcv.low.value() == 1.0));
    }

    #[test]
    fn current_price_line_uses_signal() {
        let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 50);
        let candles: Vec<Candle> = (0..10).map(make_candle).collect();
        chart.set_historical_data(candles.clone());

        let new_price = candles.last().unwrap().ohlcv.close.value() + 5.0;
        crate::app::global_current_price().set(new_price);

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
        let pr = (max_price - min_price).abs().max(1e-6);
        min_price -= pr * 0.05;
        max_price += pr * 0.05;
        let price_range = max_price - min_price;
        let expected_y = ((new_price as f32 - min_price) / price_range) * 2.0 - 1.0;

        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        for v in verts.iter().filter(|v| (v.color_type - 7.0).abs() < f32::EPSILON) {
            min_y = min_y.min(v.position_y);
            max_y = max_y.max(v.position_y);
        }
        let mid_y = (min_y + max_y) * 0.5;

        assert!((mid_y - expected_y).abs() < 1e-6);
    }
}
