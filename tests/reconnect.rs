#![cfg(feature = "render")]
use futures::future::select;
use gloo_timers::future::sleep;
use price_chart_wasm::domain::market_data::{Symbol, TimeInterval};
use price_chart_wasm::infrastructure::websocket::binance_client::BinanceWebSocketClient;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = "
let original;
export function blockSocket() {
  original = globalThis.WebSocket;
  globalThis.WebSocket = class { constructor() { throw new Error('mock connection failure'); } };
}
export function restoreSocket() { globalThis.WebSocket = original; }")]
extern "C" {
    fn blockSocket();
    fn restoreSocket();
}
#[wasm_bindgen_test(async)]
async fn reconnect_called_on_failure() {
    blockSocket();
    let mut client = BinanceWebSocketClient::new(Symbol::from("BTCUSDT"), TimeInterval::OneMinute);
    let called = Rc::new(RefCell::new(0));
    let flag = called.clone();
    let fut = client.start_stream_with_callback(|_| {}, || *flag.borrow_mut() += 1);
    let _ = select(Box::pin(fut), Box::pin(sleep(Duration::from_millis(10)))).await;
    restoreSocket();
    assert!(*called.borrow() > 0);
}
