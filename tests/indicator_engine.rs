#![cfg(feature = "render")]
mod support;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
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

#[wasm_bindgen_test::wasm_bindgen_test]
fn engine_and_service_keep_their_ema_seeds() {
    support::with_app_context(|| {
        let candles: Vec<Candle> = (1..=260).map(|i| make_candle(i, i as f64)).collect();
        let svc = MarketAnalysisService::new();
        let expected = svc.calculate_multiple_mas(&candles);
        let mut eng = MovingAverageEngine::new();
        eng.compute_historical(&candles);
        let data = eng.data();
        assert_eq!(data.sma_20, expected.sma_20);
        assert_eq!(data.sma_50, expected.sma_50);
        assert_eq!(data.sma_200, expected.sma_200);
        // These APIs intentionally retain their existing initial conditions:
        // streaming EMA starts at the first close; batch EMA starts at SMA(N).
        // Closed-form references for prices 1..=260 test every value, without
        // duplicating either implementation's recurrence.
        for (period, stream, batch) in
            [(12, &data.ema_12, &expected.ema_12), (26, &data.ema_26, &expected.ema_26)]
        {
            assert_eq!(stream.len(), candles.len());
            assert_eq!(batch.len(), candles.len() - period + 1);
            let lag = (period - 1) as f64 / 2.0;
            let decay = 1.0 - 2.0 / (period as f64 + 1.0);
            for (index, value) in stream.iter().enumerate() {
                let reference = index as f64 + 1.0 - lag * (1.0 - decay.powi(index as i32));
                assert!((value.value() - reference).abs() < 1e-9);
            }
            for (index, value) in batch.iter().enumerate() {
                let reference = (period as f64 + 1.0) / 2.0 + index as f64;
                assert!((value.value() - reference).abs() < 1e-9);
            }
        }
    });
}
