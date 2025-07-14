use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use wasm_bindgen_test::*;

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
fn viewport_clamped_to_data() {
    let candles: Vec<Candle> = (0..5).map(make_candle).collect();
    let mut chart = Chart::new("test".into(), ChartType::Candlestick, 100);
    chart.set_historical_data(candles);

    chart.zoom(0.5, 1.0);
    chart.pan(1.0, 0.0);

    let last_ts = 4 * 60_000u64;
    assert!((chart.viewport.end_time - last_ts as f64).abs() < f64::EPSILON);
}
