use price_chart_wasm::app::price_levels;
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use wasm_bindgen_test::*;

fn base_candle(ts: u64) -> Candle {
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

fn big_candle(ts: u64) -> Candle {
    Candle::new(
        Timestamp::from(ts),
        OHLCV::new(
            Price::from(150.0),
            Price::from(200.0),
            Price::from(140.0),
            Price::from(190.0),
            Volume::from(1.0),
        ),
    )
}

#[wasm_bindgen_test]
fn price_levels_update_after_data_change() {
    let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 10);
    chart.add_candle(base_candle(0));
    chart.add_candle(base_candle(60_000));

    chart.update_viewport_for_data();
    let before = price_levels(&chart.viewport);

    chart.add_candle(big_candle(120_000));
    chart.update_viewport_for_data();
    let after = price_levels(&chart.viewport);

    assert!(after[0] > before[0]);
}
