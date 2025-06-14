use price_chart_wasm::domain::market_data::services::MarketAnalysisService;
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
fn moving_average_short_input() {
    let svc = MarketAnalysisService::new();
    let candles: Vec<Candle> = (0..3).map(make_candle).collect();

    assert!(svc.calculate_sma(&candles, 5).is_empty());
    assert!(svc.calculate_ema(&candles, 5).is_empty());
}
