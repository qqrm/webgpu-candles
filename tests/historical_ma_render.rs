#![cfg(feature = "render")]
use price_chart_wasm::app::{current_interval, visible_range_by_time};
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, TimeInterval, Timestamp, Volume,
};
use price_chart_wasm::infrastructure::rendering::renderer::dummy_renderer;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
#[test]
fn historical_sma20_rendered() {
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

    let candles: Vec<Candle> = (0..30).map(make_candle).collect();

    let mut chart = Chart::new("hist-ma".to_string(), ChartType::Candlestick, 100);
    chart.set_historical_data(candles);

    let renderer = dummy_renderer();
    let (_, verts, _) = renderer.create_geometry_for_test(&chart);

    let sma20_vertices: Vec<_> =
        verts.iter().filter(|v| (v.color_type - 2.0).abs() < f32::EPSILON).collect();

    assert!(!sma20_vertices.is_empty());
}

#[test]
fn minute_ma_tracks_closed_buckets() {
    const BASE_PRICE: f64 = 100.0;
    const MINUTES: u64 = 15;
    const EMA_PERIOD: usize = 12;

    fn minute_batch(minute: u64, price: f64) -> Vec<Candle> {
        (0..30)
            .map(|i| {
                let ts = minute * 60_000 + i * 2_000;
                Candle::new(
                    Timestamp::from_millis(ts),
                    OHLCV::new(
                        Price::from(price),
                        Price::from(price + 0.5),
                        Price::from(price - 0.5),
                        Price::from(price),
                        Volume::from(1.0),
                    ),
                )
            })
            .collect()
    }

    let mut candles = Vec::new();
    for minute in 0..MINUTES {
        candles.extend(minute_batch(minute, BASE_PRICE + minute as f64));
    }

    let mut chart = Chart::new("hist-minute-ma".to_string(), ChartType::Candlestick, 1000);
    chart.set_historical_data(candles);

    let minute_series = chart.get_series(TimeInterval::OneMinute).expect("minute series available");
    let aggregated: Vec<Candle> = minute_series.get_candles().iter().cloned().collect();
    assert_eq!(aggregated.len(), MINUTES as usize);

    let closes: Vec<f64> = aggregated.iter().map(|c| c.ohlcv.close.value()).collect();
    for (idx, close) in closes.iter().enumerate() {
        let expected = BASE_PRICE + idx as f64;
        assert!((close - expected).abs() < f64::EPSILON);
    }

    assert!(closes.len() > 1, "need at least one closed bucket");
    let closed_closes = &closes[..closes.len() - 1];

    let engine =
        chart.ma_engines.get(&TimeInterval::OneMinute).expect("ma engine for minute interval");
    let ema12 = &engine.data().ema_12;
    assert_eq!(ema12.len(), closed_closes.len());

    let alpha = 2.0 / (EMA_PERIOD as f64 + 1.0);
    let mut expected_ema = Vec::with_capacity(closed_closes.len());
    let mut last = closed_closes[0];
    expected_ema.push(last);
    for &close in closed_closes.iter().skip(1) {
        last = alpha * close + (1.0 - alpha) * last;
        expected_ema.push(last);
    }

    for (calc, exp) in ema12.iter().zip(expected_ema.iter()) {
        assert!((calc.value() - *exp).abs() < 1e-6);
    }

    let prev_interval = current_interval().get_untracked();
    current_interval().set(TimeInterval::OneMinute);
    let renderer = dummy_renderer();
    let (_, vertices, _) = renderer.create_geometry_for_test(&chart);
    current_interval().set(prev_interval);

    let ema_vertices: Vec<_> = vertices
        .iter()
        .filter(|v| {
            (v.element_type - 2.0).abs() < f32::EPSILON && (v.color_type - 5.0).abs() < f32::EPSILON
        })
        .collect();

    let (start_idx, visible_len) = visible_range_by_time(&aggregated, &chart.viewport, 1.0);
    let period_offset = EMA_PERIOD - 1;
    let drawn_points = ema12
        .iter()
        .enumerate()
        .filter(|(idx, _)| {
            let candle_idx = idx + period_offset;
            *candle_idx >= start_idx && *candle_idx < start_idx + visible_len
        })
        .count();

    assert!(drawn_points >= 2, "EMA line should have at least two points");
    let expected_segments = drawn_points - 1;
    assert_eq!(ema_vertices.len(), expected_segments * 6);
}
