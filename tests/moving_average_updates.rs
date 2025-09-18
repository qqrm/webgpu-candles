#![cfg(feature = "render")]

use price_chart_wasm::domain::chart::Chart;
use price_chart_wasm::domain::chart::value_objects::ChartType;
use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, TimeInterval, Timestamp, Volume, indicator_engine::MovingAverageEngine,
};
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn candle(ts: u64, close: f64) -> Candle {
    Candle::new(
        Timestamp::from(ts),
        OHLCV::new(
            Price::from(close),
            Price::from(close),
            Price::from(close),
            Price::from(close),
            Volume::from(1.0),
        ),
    )
}

#[wasm_bindgen_test]
fn partial_realtime_updates_refresh_mas() {
    let mut chart = Chart::new("test".into(), ChartType::Candlestick, 256);

    for i in 0..19_u64 {
        chart.add_realtime_candle(candle(i * 2_000, (i + 1) as f64));
    }

    let last_ts = 19 * 2_000;
    chart.add_realtime_candle(candle(last_ts, 20.0));
    chart.add_realtime_candle(candle(last_ts, 40.0));

    let mut expected = MovingAverageEngine::new();
    for close in 1..=19 {
        expected.update_on_close(close as f64);
    }
    expected.update_on_close(40.0);

    let base_engine = chart.ma_engines.get(&TimeInterval::TwoSeconds).expect("base engine");
    assert_eq!(base_engine.data().sma_20.last(), expected.data().sma_20.last());
    assert_eq!(base_engine.data().ema_12.last(), expected.data().ema_12.last());
    assert_eq!(base_engine.data().ema_26.last(), expected.data().ema_26.last());

    let minute_engine = chart.ma_engines.get(&TimeInterval::OneMinute).expect("minute engine");
    assert_eq!(minute_engine.data().ema_12.last().map(|price| price.value()), Some(40.0));
    assert_eq!(minute_engine.data().ema_26.last().map(|price| price.value()), Some(40.0));
}
