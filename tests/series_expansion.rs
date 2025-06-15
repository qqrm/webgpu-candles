use price_chart_wasm::app::visible_range;
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn make_candle(i: u64) -> Candle {
    Candle::new(
        Timestamp::from_millis(i * 60_000),
        OHLCV::new(
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Volume::from(1.0),
        ),
    )
}

#[wasm_bindgen_test]
fn series_extension_preserves_view() {
    let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 400);
    for i in 100..200 {
        chart.add_candle(make_candle(i));
    }

    let (start_before, visible) = visible_range(chart.get_candle_count(), 1.0, -20.0);

    for i in 0..100 {
        chart.add_candle(make_candle(i));
    }

    let (start_after, visible_after) = visible_range(chart.get_candle_count(), 1.0, -20.0);

    assert_eq!(visible, visible_after);
    assert_eq!(start_after, start_before + 100);
}
