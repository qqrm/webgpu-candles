use price_chart_wasm::domain::chart::Chart;
use price_chart_wasm::domain::chart::value_objects::{ChartType, Viewport};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

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

#[wasm_bindgen_test]
fn pan_reveals_latest_realtime_candle() {
    let mut chart = Chart::new("test".into(), ChartType::Candlestick, 10);

    let candles = vec![
        Candle::new(
            Timestamp::from_millis(0),
            OHLCV::new(
                Price::from(1.0),
                Price::from(1.0),
                Price::from(1.0),
                Price::from(1.0),
                Volume::from(1.0),
            ),
        ),
        Candle::new(
            Timestamp::from_millis(60_000),
            OHLCV::new(
                Price::from(1.0),
                Price::from(1.0),
                Price::from(1.0),
                Price::from(1.0),
                Volume::from(1.0),
            ),
        ),
    ];

    chart.set_historical_data(candles);

    chart.add_realtime_candle(Candle::new(
        Timestamp::from_millis(120_000),
        OHLCV::new(
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Volume::from(1.0),
        ),
    ));

    chart.pan(1.0, 0.0);

    assert_eq!(chart.viewport.start_time, 60_000.0);
    assert_eq!(chart.viewport.end_time, 120_000.0);
}
