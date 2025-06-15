use price_chart_wasm::domain::market_data::services::Aggregator;
use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, TimeInterval, Timestamp, Volume,
};
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn minute_candle(timestamp: u64, open: f64) -> Candle {
    Candle::new(
        Timestamp::from_millis(timestamp),
        OHLCV::new(
            Price::from(open),
            Price::from(open + 5.0),
            Price::from(open - 5.0),
            Price::from(open + 1.0),
            Volume::from(1.0),
        ),
    )
}

#[wasm_bindgen_test]
fn aggregates_five_minutes() {
    let candles: Vec<Candle> =
        (0..5).map(|i| minute_candle(i * 60_000, 100.0 + i as f64)).collect();

    let aggregated = Aggregator::aggregate(&candles, TimeInterval::FiveMinutes).unwrap();

    assert_eq!(aggregated.timestamp.value(), 0);
    assert!((aggregated.ohlcv.open.value() - 100.0).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.close.value() - 105.0).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.high.value() - 109.0).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.low.value() - 95.0).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.volume.value() - 5.0).abs() < f64::EPSILON);
}

#[wasm_bindgen_test]
fn aggregates_fifteen_minutes() {
    let candles: Vec<Candle> =
        (0..15).map(|i| minute_candle(i * 60_000, 100.0 + i as f64)).collect();

    let aggregated = Aggregator::aggregate(&candles, TimeInterval::FifteenMinutes).unwrap();

    assert_eq!(aggregated.timestamp.value(), 0);
    assert!((aggregated.ohlcv.open.value() - 100.0).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.close.value() - 115.0).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.high.value() - 119.0).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.low.value() - 95.0).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.volume.value() - 15.0).abs() < f64::EPSILON);
}
