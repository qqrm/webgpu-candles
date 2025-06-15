use gloo_timers::future::sleep;
use leptos::*;
use price_chart_wasm::app::{
    abort_other_streams, current_symbol, start_websocket_stream, stream_abort_handles,
};
use price_chart_wasm::domain::market_data::Symbol;
use std::time::Duration;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test(async)]
async fn stream_creation_and_abort() {
    current_symbol().set(Symbol::from("BTCUSDT"));
    let (_, set_status) = create_signal(String::new());
    start_websocket_stream(set_status).await;
    sleep(Duration::from_millis(10)).await;
    abort_other_streams(&Symbol::from("BTCUSDT"));
    sleep(Duration::from_millis(10)).await;
    assert!(stream_abort_handles().with(|m| !m.contains_key(&Symbol::from("BTCUSDT"))));
}
