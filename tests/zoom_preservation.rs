#![cfg(feature = "render")]
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use wasm_bindgen_test::*;

fn test_candle(ts: u64) -> Candle {
    Candle::new(
        Timestamp::from(ts),
        OHLCV::new(
            Price::from(100.0),
            Price::from(110.0),
            Price::from(90.0),
            Price::from(105.0),
            Volume::from(1.0),
        ),
    )
}

#[wasm_bindgen_test]
fn zoom_not_reset_by_realtime_candle() {
    // initialize chart with some data
    let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 100);
    chart.set_historical_data(vec![test_candle(0), test_candle(60_000)]);

    // zoom in around center
    chart.zoom(2.0, 0.5);
    let before = chart.viewport.clone();

    // add new realtime candle
    chart.add_realtime_candle(test_candle(120_000));

    // viewport should remain the same
    assert_eq!(chart.viewport, before);
}
