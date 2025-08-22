use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, Timestamp, Volume, indicator_engine::MovingAverageEngine,
    services::MarketAnalysisService,
};

fn make_candle(ts: u64, close: f64) -> Candle {
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

#[test]
fn engine_matches_service() {
    let candles: Vec<Candle> = (1..=30).map(|i| make_candle(i, i as f64)).collect();
    let svc = MarketAnalysisService::new();
    let expected = svc.calculate_multiple_mas(&candles);
    let mut eng = MovingAverageEngine::new();
    eng.compute_historical(&candles);
    let data = eng.data();
    assert_eq!(data.sma_20, expected.sma_20);
    assert_eq!(data.ema_12, expected.ema_12);
}
