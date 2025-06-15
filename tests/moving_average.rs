use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, Timestamp, Volume, services::MarketAnalysisService,
};
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn create_candle(close: f64, index: u64) -> Candle {
    Candle::new(
        Timestamp::from(index),
        OHLCV::new(
            Price::from(close),
            Price::from(close),
            Price::from(close),
            Price::from(close),
            Volume::from(1.0),
        ),
    )
}

#[wasm_bindgen_test]
fn moving_averages_match_manual_calculation() {
    let prices = [10.0, 12.0, 11.0, 13.0, 15.0, 14.0, 16.0];
    let candles: Vec<Candle> =
        prices.iter().enumerate().map(|(i, &p)| create_candle(p, i as u64)).collect();

    let service = MarketAnalysisService::new();

    let sma3 = service.calculate_sma(&candles, 3);
    let expected_sma3 = [11.0, 12.0, 13.0, 14.0, 15.0];
    assert_eq!(sma3.len(), expected_sma3.len());
    for (calc, exp) in sma3.iter().zip(expected_sma3.iter()) {
        assert!((calc.value() - exp).abs() < f64::EPSILON);
    }

    let sma5 = service.calculate_sma(&candles, 5);
    let expected_sma5 = [12.2, 13.0, 13.8];
    assert_eq!(sma5.len(), expected_sma5.len());
    for (calc, exp) in sma5.iter().zip(expected_sma5.iter()) {
        assert!((calc.value() - exp).abs() < f64::EPSILON);
    }

    let ema3 = service.calculate_ema(&candles, 3);
    let expected_ema3 = [11.0, 12.0, 13.5, 13.75, 14.875];
    assert_eq!(ema3.len(), expected_ema3.len());
    for (calc, exp) in ema3.iter().zip(expected_ema3.iter()) {
        assert!((calc.value() - exp).abs() < f64::EPSILON);
    }

    let ema5 = service.calculate_ema(&candles, 5);
    let expected_ema5 = [12.2, 12.8, 13.866666666666667];
    assert_eq!(ema5.len(), expected_ema5.len());
    for (calc, exp) in ema5.iter().zip(expected_ema5.iter()) {
        assert!((calc.value() - exp).abs() < f64::EPSILON);
    }
}

#[wasm_bindgen_test]
fn moving_average_short_input() {
    let svc = MarketAnalysisService::new();
    let candles: Vec<Candle> = (0..3).map(|i| create_candle(1.0, i)).collect();

    assert!(svc.calculate_sma(&candles, 5).is_empty());
    assert!(svc.calculate_ema(&candles, 5).is_empty());
}
