#![cfg(feature = "render")]
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

fn two_second_candle(timestamp: u64, open: f64) -> Candle {
    Candle::new(
        Timestamp::from_millis(timestamp),
        OHLCV::new(
            Price::from(open),
            Price::from(open + 0.5),
            Price::from(open - 0.5),
            Price::from(open + 0.1),
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

#[wasm_bindgen_test]
fn aggregates_one_minute_from_two_seconds() {
    let candles: Vec<Candle> =
        (0..30).map(|i| two_second_candle(i * 2_000, 100.0 + i as f64)).collect();

    let aggregated = Aggregator::aggregate(&candles, TimeInterval::OneMinute).unwrap();

    assert_eq!(aggregated.timestamp.value(), 0);
    assert!((aggregated.ohlcv.open.value() - 100.0).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.close.value() - 129.1).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.high.value() - 129.5).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.low.value() - 99.5).abs() < f64::EPSILON);
    assert!((aggregated.ohlcv.volume.value() - 30.0).abs() < f64::EPSILON);
}
