#![cfg(feature = "render")]
use price_chart_wasm::app::visible_range;
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};

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

#[test]
fn pan_reset_shows_latest_candle() {
    let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 400);
    for i in 0..400 {
        chart.add_candle(make_candle(i as u64));
    }

    let pan = 0.0;
    chart.update_viewport_for_data();

    let len = chart.get_candle_count();
    let (start, visible) = visible_range(len, 1.0, pan);

    assert_eq!(pan, 0.0);
    assert_eq!(start + visible - 1, len - 1);
}
