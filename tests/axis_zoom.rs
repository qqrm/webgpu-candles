use price_chart_wasm::app::{price_levels, visible_range, visible_range_by_time};
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
fn price_levels_change_after_zoom() {
    let mut vp = Viewport {
        start_time: 0.0,
        end_time: 100.0,
        min_price: 0.0,
        max_price: 100.0,
        width: 800,
        height: 600,
    };

    let before = price_levels(&vp);
    vp.zoom_price(2.0, 0.5);
    let after = price_levels(&vp);

    assert_ne!(before, after);
    assert!((after[0] - 75.0).abs() < 1e-6);
    assert!((after[8] - 25.0).abs() < 1e-6);
}

#[wasm_bindgen_test]
fn time_axis_respects_zoom() {
    let candles: Vec<Candle> = (0..100).map(make_candle).collect();
    let mut vp = Viewport {
        start_time: 0.0,
        end_time: 100.0 * 60_000.0,
        min_price: 0.0,
        max_price: 1.0,
        width: 800,
        height: 600,
    };

    let (start_before, count_before) = visible_range_by_time(&candles, &vp, 1.0);
    assert_eq!(start_before, 0);
    assert_eq!(count_before, 32);

    vp.zoom(2.0, 0.5);
    let (start_after, count_after) = visible_range_by_time(&candles, &vp, 2.0);
    assert_eq!(start_after, 25);
    assert_eq!(count_after, 16);
}

#[test]
fn visible_range_updates_with_new_candles() {
    assert_eq!(visible_range(60, 1.0, 0.0), (28, 32));
    assert_eq!(visible_range(70, 1.0, 0.0), (38, 32));
    assert_eq!(visible_range(70, 2.0, 0.0), (54, 16));
}
