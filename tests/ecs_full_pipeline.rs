use leptos::*;
use price_chart_wasm::domain::market_data::{Symbol, TimeInterval};
use price_chart_wasm::global_state::{ecs_world, ensure_chart, push_realtime_candle};
use price_chart_wasm::infrastructure::rendering::renderer::dummy_renderer;
use price_chart_wasm::infrastructure::websocket::binance_client::BinanceWebSocketClient;
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
#[wasm_bindgen_test]
fn websocket_to_webgpu_pipeline() {
    ecs_world().lock().unwrap().world = hecs::World::new();

    let symbol = Symbol::from("PIPE");
    let chart = ensure_chart(&symbol);

    let client = BinanceWebSocketClient::new(symbol.clone(), TimeInterval::OneMinute);
    let msg = r#"{"k":{"t":0,"o":"1.0","h":"1.1","l":"0.9","c":"1.05","v":"1.0"}}"#;
    let candle = client.parse_message(msg).unwrap();
    push_realtime_candle(candle);

    let msg2 = r#"{"k":{"t":60000,"o":"1.05","h":"1.1","l":"1.0","c":"1.08","v":"1.5"}}"#;
    let candle2 = client.parse_message(msg2).unwrap();
    push_realtime_candle(candle2);

    assert_eq!(chart.with_untracked(|c| c.get_candle_count()), 2);

    let chart_clone = chart.with_untracked(|c| c.clone());
    let mut renderer = dummy_renderer();
    renderer.cache_geometry_for_test(&chart_clone);
    assert_ne!(renderer.cached_hash_for_test(), 0);
}

#[wasm_bindgen_test]
fn websocket_multiple_symbols() {
    ecs_world().lock().unwrap().world = hecs::World::new();

    let sym_a = Symbol::from("WSA");
    let sym_b = Symbol::from("WSB");
    let chart_a = ensure_chart(&sym_a);
    let chart_b = ensure_chart(&sym_b);

    let client_a = BinanceWebSocketClient::new(sym_a.clone(), TimeInterval::OneMinute);
    let msg_a = r#"{"k":{"t":0,"o":"1.0","h":"1.1","l":"0.9","c":"1.05","v":"1.0"}}"#;
    let candle_a = client_a.parse_message(msg_a).unwrap();
    push_realtime_candle(candle_a);

    let client_b = BinanceWebSocketClient::new(sym_b.clone(), TimeInterval::OneMinute);
    let msg_b = r#"{"k":{"t":0,"o":"2.0","h":"2.1","l":"1.9","c":"2.05","v":"2.0"}}"#;
    let candle_b = client_b.parse_message(msg_b).unwrap();
    push_realtime_candle(candle_b);

    assert_eq!(chart_a.with_untracked(|c| c.get_candle_count()), 1);
    assert_eq!(chart_b.with_untracked(|c| c.get_candle_count()), 1);
}
