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

    assert_eq!(chart.with(|c| c.get_candle_count()), 1);

    let chart_clone = chart.with(|c| c.clone());
    let mut renderer = dummy_renderer();
    renderer.cache_geometry_for_test(&chart_clone);
    assert_ne!(renderer.cached_hash_for_test(), 0);
}
