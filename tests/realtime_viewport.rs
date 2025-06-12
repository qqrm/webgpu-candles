use price_chart_wasm::domain::chart::Chart;
use price_chart_wasm::domain::chart::value_objects::{ChartType, Viewport};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn add_realtime_candle_keeps_viewport() {
    let mut chart = Chart::new("test".into(), ChartType::Candlestick, 10);
    chart.viewport = Viewport {
        start_time: 100.0,
        end_time: 200.0,
        min_price: 10.0,
        max_price: 20.0,
        width: 800,
        height: 600,
    };

    let original = chart.viewport.clone();

    chart.add_realtime_candle(Candle::new(
        Timestamp::from_millis(201),
        OHLCV::new(
            Price::from(15.0),
            Price::from(16.0),
            Price::from(14.0),
            Price::from(15.5),
            Volume::from(1.0),
        ),
    ));

    assert_eq!(chart.viewport, original);
}
