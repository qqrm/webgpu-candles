use price_chart_wasm::app::{visible_range, visible_range_by_time};
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, TimeInterval, Timestamp, Volume,
};
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn test_candle(ts: u64) -> Candle {
    Candle::new(
        Timestamp::from_millis(ts),
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
fn realtime_start_visible_range() {
    let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 100);
    chart.set_historical_data(vec![test_candle(0), test_candle(60_000)]);

    chart.add_realtime_candle(test_candle(120_000));

    let candles: Vec<Candle> =
        chart.get_series(TimeInterval::OneMinute).unwrap().get_candles().iter().cloned().collect();

    let (start, visible) = visible_range_by_time(&candles, &chart.viewport, 1.0);
    assert_eq!(start + visible - 1, candles.len() - 1);

    let (start_pan, visible_pan) = visible_range(candles.len(), 1.0, 0.0);
    assert_eq!(start_pan + visible_pan - 1, candles.len() - 1);
}
