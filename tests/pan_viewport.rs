use price_chart_wasm::app::visible_range_by_time;
use price_chart_wasm::domain::chart::value_objects::Viewport;
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
fn range_uses_viewport_start_time() {
    let candles: Vec<Candle> = (0..400).map(make_candle).collect();
    let vp = Viewport {
        start_time: 50.0 * 60_000.0,
        end_time: 350.0 * 60_000.0,
        min_price: 0.0,
        max_price: 1.0,
        width: 800,
        height: 600,
    };

    let (start, count) = visible_range_by_time(&candles, &vp, 1.0);
    assert_eq!(start, 50);
    assert_eq!(count, 100);
}
