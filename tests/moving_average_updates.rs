use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, TimeInterval, Timestamp, Volume, indicator_engine::MovingAverageEngine,
};

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

#[test]
fn moving_averages_follow_partial_updates() {
    let mut chart = Chart::new("ma-test".to_string(), ChartType::Candlestick, 100);
    let mut final_closes = Vec::new();
    let step = TimeInterval::TwoSeconds.duration_ms();

    for i in 0..19 {
        let close = 100.0 + i as f64;
        final_closes.push(close);
        chart.add_realtime_candle(candle(i as u64 * step, close));
    }

    let last_ts = 19 * step;
    let partial_close = 119.0;
    let final_close = 150.0;

    chart.add_realtime_candle(candle(last_ts, partial_close));
    final_closes.push(final_close);
    chart.add_realtime_candle(candle(last_ts, final_close));

    let mut expected = MovingAverageEngine::new();
    for close in &final_closes {
        expected.update_on_close(*close);
    }
    let expected_data = expected.data();
    let expected_sma20 = expected_data.sma_20.last().copied().expect("sma20 computed");
    let expected_ema12 = expected_data.ema_12.last().copied().expect("ema12 computed");
    let expected_ema26 = expected_data.ema_26.last().copied().expect("ema26 computed");

    let engine = chart.ma_engines.get(&TimeInterval::TwoSeconds).expect("base engine exists");
    let data = engine.data();

    assert_eq!(data.sma_20.last().copied(), Some(expected_sma20));
    assert_eq!(data.ema_12.last().copied(), Some(expected_ema12));
    assert_eq!(data.ema_26.last().copied(), Some(expected_ema26));
}
