#![cfg(feature = "render")]
#[cfg(not(target_arch = "wasm32"))]
use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, Timestamp, Volume, indicator_engine::MovingAverageEngine,
};

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn update_under_half_ms() {
    use std::time::Instant;
    let candles: Vec<Candle> = (0..10_000)
        .map(|i| {
            Candle::new(
                Timestamp::from(i as u64),
                OHLCV::new(
                    Price::from(1.0),
                    Price::from(1.0),
                    Price::from(1.0),
                    Price::from(i as f64),
                    Volume::from(1.0),
                ),
            )
        })
        .collect();
    let mut eng = MovingAverageEngine::new();
    eng.compute_historical(&candles);
    let start = Instant::now();
    eng.update_on_close(10_001.0);
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs_f64() * 1_000.0 < 0.5,
        "update took {:.3} ms",
        elapsed.as_secs_f64() * 1_000.0
    );
}
