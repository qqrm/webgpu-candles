use price_chart_wasm::domain::market_data::{
    Candle, CandleSeries, OHLCV, Price, Timestamp, Volume,
};
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
#[wasm_bindgen_test]
fn candle_methods() {
    let candle = Candle::new(
        Timestamp::from_millis(0),
        OHLCV::new(
            Price::from(10.0),
            Price::from(12.0),
            Price::from(9.0),
            Price::from(11.0),
            Volume::from(1.0),
        ),
    );
    assert!(candle.is_bullish());
    assert!(!candle.is_bearish());
    assert_eq!(candle.body_size().value(), 1.0);
    assert_eq!(candle.wick_high().value(), 1.0);
    assert_eq!(candle.wick_low().value(), 0.0);
}

#[wasm_bindgen_test]
fn candle_series_add_and_price_range() {
    let mut series = CandleSeries::new(3);
    series.add_candle(Candle::new(
        Timestamp::from_millis(0),
        OHLCV::new(
            Price::from(10.0),
            Price::from(12.0),
            Price::from(9.0),
            Price::from(11.0),
            Volume::from(1.0),
        ),
    ));
    series.add_candle(Candle::new(
        Timestamp::from_millis(1),
        OHLCV::new(
            Price::from(11.0),
            Price::from(13.0),
            Price::from(10.0),
            Price::from(12.0),
            Volume::from(1.0),
        ),
    ));
    series.add_candle(Candle::new(
        Timestamp::from_millis(2),
        OHLCV::new(
            Price::from(12.0),
            Price::from(14.0),
            Price::from(11.0),
            Price::from(13.0),
            Volume::from(1.0),
        ),
    ));
    assert_eq!(series.count(), 3);
    let (min, max) = series.price_range().unwrap();
    assert_eq!(min.value(), 9.0);
    assert_eq!(max.value(), 14.0);
    // add the 4th candle and check size limit
    series.add_candle(Candle::new(
        Timestamp::from_millis(3),
        OHLCV::new(
            Price::from(13.0),
            Price::from(15.0),
            Price::from(12.0),
            Price::from(14.0),
            Volume::from(1.0),
        ),
    ));
    assert_eq!(series.count(), 3);
    assert_eq!(series.get_candles().front().unwrap().timestamp.value(), 1);
}
