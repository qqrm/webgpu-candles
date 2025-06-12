use price_chart_wasm::domain::market_data::{Symbol, TimeInterval};
use price_chart_wasm::infrastructure::websocket::binance_client::BinanceWebSocketClient;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn parses_kline_message() {
    let client = BinanceWebSocketClient::new(Symbol::from("BTCUSDT"), TimeInterval::OneMinute);
    let msg = r#"{\"k\":{\"t\":123456789,\"o\":\"10000.0\",\"h\":\"10100.0\",\"l\":\"9900.0\",\"c\":\"10050.0\",\"v\":\"10.0\"}}"#;
    let candle = client.parse_message(msg).unwrap();
    assert_eq!(candle.timestamp.value(), 123456789);
    assert!((candle.ohlcv.open.value() - 10000.0).abs() < f64::EPSILON);
    assert!((candle.ohlcv.close.value() - 10050.0).abs() < f64::EPSILON);
    assert!((candle.ohlcv.high.value() - 10100.0).abs() < f64::EPSILON);
    assert!((candle.ohlcv.low.value() - 9900.0).abs() < f64::EPSILON);
    assert!((candle.ohlcv.volume.value() - 10.0).abs() < f64::EPSILON);
}
