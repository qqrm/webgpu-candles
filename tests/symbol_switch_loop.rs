use futures::future::AbortHandle;
use leptos::*;
use price_chart_wasm::app::{abort_other_streams, current_symbol, stream_abort_handles};
use price_chart_wasm::domain::market_data::Symbol;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn wasm_memory_bytes() -> u32 {
    let mem = wasm_bindgen::memory().unchecked_into::<js_sys::WebAssembly::Memory>();
    js_sys::Uint8Array::new(&mem.buffer()).length()
}

#[wasm_bindgen_test(async)]
async fn switch_symbols_twenty_times_no_leak() {
    let start_mem = wasm_memory_bytes();
    let symbols = [Symbol::from("BTCUSDT"), Symbol::from("ETHUSDT"), Symbol::from("SOLUSDT")];
    for i in 0..20 {
        let sym = symbols[i % symbols.len()].clone();
        current_symbol().set(sym.clone());
        abort_other_streams(&sym);
        let (handle, _) = AbortHandle::new_pair();
        stream_abort_handles().update(|m| {
            m.insert(sym.clone(), handle);
        });
    }
    let end_mem = wasm_memory_bytes();
    assert!(end_mem as f64 <= start_mem as f64 * 1.05);
    assert_eq!(stream_abort_handles().with(|m| m.len()), 1);
}
