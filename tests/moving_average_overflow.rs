#![cfg(feature = "render")]

use price_chart_wasm::domain::{
    chart::{Chart, value_objects::ChartType},
    market_data::{Candle, OHLCV, Price, TimeInterval, Timestamp, Volume},
};
use price_chart_wasm::infrastructure::rendering::renderer::dummy_renderer;

#[test]
fn moving_averages_continue_after_capacity_overflow() {
    let max_candles = 40usize;
    let total_candles = max_candles * 30 + 60;
    let mut chart = Chart::new("ma-overflow".to_string(), ChartType::Candlestick, max_candles);

    let base_interval_ms = TimeInterval::TwoSeconds.duration_ms();
    let minute_interval_ms = TimeInterval::OneMinute.duration_ms();

    let mut minute_closes = Vec::new();
    let mut current_bucket: Option<u64> = None;

    for i in 0..total_candles {
        let timestamp = i as u64 * base_interval_ms;
        let close = i as f64;
        let candle = Candle::new(
            Timestamp::from_millis(timestamp),
            OHLCV::new(
                Price::from(close),
                Price::from(close),
                Price::from(close),
                Price::from(close),
                Volume::from(1.0),
            ),
        );

        let bucket_start = timestamp / minute_interval_ms * minute_interval_ms;
        match current_bucket {
            Some(active) if active == bucket_start => {
                if let Some(last_close) = minute_closes.last_mut() {
                    *last_close = close;
                }
            }
            _ => {
                current_bucket = Some(bucket_start);
                minute_closes.push(close);
            }
        }

        chart.add_candle(candle);
    }

    chart.update_viewport_for_data();

    let base_engine = chart.ma_engines.get(&TimeInterval::TwoSeconds).expect("base engine");
    let base_data = base_engine.data();
    assert_eq!(base_data.ema_12.len(), total_candles);
    let expected_base_sma20_len = total_candles - 20 + 1;
    assert_eq!(base_data.sma_20.len(), expected_base_sma20_len);
    let last_index = total_candles - 1;
    let expected_base_avg = ((last_index - 19) as f64 + last_index as f64) / 2.0;
    let base_last = base_data.sma_20.last().expect("latest SMA20 value for base interval");
    assert!((base_last.value() - expected_base_avg).abs() < 1e-9);

    let minute_engine = chart.ma_engines.get(&TimeInterval::OneMinute).expect("minute engine");
    let minute_data = minute_engine.data();
    assert_eq!(minute_data.ema_12.len(), minute_closes.len());
    let expected_minute_sma_len = minute_closes.len() - 20 + 1;
    assert_eq!(minute_data.sma_20.len(), expected_minute_sma_len);
    let minute_avg: f64 =
        minute_closes[minute_closes.len() - 20..].iter().copied().sum::<f64>() / 20.0;
    let minute_last = minute_data.sma_20.last().expect("latest SMA20 value for minute interval");
    assert!((minute_last.value() - minute_avg).abs() < 1e-9);

    let renderer = dummy_renderer();
    let (_, vertices, _) = renderer.create_geometry_for_test(&chart);
    for color in 2..=6 {
        assert!(
            vertices.iter().any(|v| (v.color_type - color as f32).abs() < f32::EPSILON),
            "missing indicator vertices for color {color}"
        );
    }
}
