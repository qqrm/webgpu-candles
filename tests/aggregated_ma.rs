use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::services::MarketAnalysisService;
use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, TimeInterval, Timestamp, Volume,
};

fn build_minute_candles(minutes: u64, per_minute: u64) -> Vec<Candle> {
    let mut candles = Vec::new();
    for minute in 0..minutes {
        let minute_start = minute * 60_000;
        let base_close = 100.0 + minute as f64;
        for part in 0..per_minute {
            let offset = part * (60_000 / per_minute);
            let timestamp = minute_start + offset;
            let close = base_close + part as f64 * 0.25;
            let open = if part == 0 { base_close } else { base_close + (part - 1) as f64 * 0.25 };
            let high = close + 0.5;
            let low = open.min(close) - 0.5;
            let volume = 1.0 + minute as f64 * 0.1 + part as f64 * 0.1;

            candles.push(Candle::new(
                Timestamp::from_millis(timestamp),
                OHLCV::new(
                    Price::from(open),
                    Price::from(high),
                    Price::from(low.max(0.1)),
                    Price::from(close),
                    Volume::from(volume),
                ),
            ));
        }
    }
    candles
}

fn approx_eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn aggregated_minute_ma_matches_closes() {
    let minutes = 25;
    let per_minute = 3;
    let candles = build_minute_candles(minutes, per_minute);

    let mut chart = Chart::new("minute-ma".to_string(), ChartType::Candlestick, 512);
    chart.set_historical_data(candles);

    let minute_series = chart.get_series(TimeInterval::OneMinute).expect("minute series missing");
    let aggregated: Vec<Candle> = minute_series.get_candles().iter().cloned().collect();
    assert_eq!(aggregated.len(), minutes as usize);

    let expected_closes: Vec<f64> =
        (0..minutes).map(|m| 100.0 + m as f64 + (per_minute - 1) as f64 * 0.25).collect();
    let aggregated_closes: Vec<f64> = aggregated.iter().map(|c| c.ohlcv.close.value()).collect();
    for window in aggregated.windows(2) {
        let lhs = window[0].timestamp.value();
        let rhs = window[1].timestamp.value();
        assert_eq!(rhs - lhs, TimeInterval::OneMinute.duration_ms());
    }
    for (calc, exp) in aggregated_closes.iter().zip(expected_closes.iter()) {
        assert!(approx_eq(*calc, *exp));
    }

    let engine = chart.ma_engines.get(&TimeInterval::OneMinute).expect("minute MA engine missing");
    let ma_data = engine.data();

    let analysis = MarketAnalysisService::new();
    let expected_sma20 = analysis.calculate_sma(&aggregated, 20);
    assert_eq!(ma_data.sma_20.len(), expected_sma20.len());
    assert_eq!(ma_data.sma_20.len(), aggregated.len().saturating_sub(19));
    for (calc, exp) in ma_data.sma_20.iter().zip(expected_sma20.iter()) {
        assert!(approx_eq(calc.value(), exp.value()));
    }

    let expected_ema12 = analysis.calculate_ema(&aggregated, 12);
    assert_eq!(ma_data.ema_12.len(), expected_ema12.len());
    assert_eq!(ma_data.ema_12.len(), aggregated.len());
    for (calc, exp) in ma_data.ema_12.iter().zip(expected_ema12.iter()) {
        assert!(approx_eq(calc.value(), exp.value()));
    }

    assert!(ma_data.sma_50.is_empty());
    assert!(ma_data.sma_200.is_empty());
    assert!(ma_data.ema_26.is_empty());
}
