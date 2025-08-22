use futures::future::AbortHandle;
use leptos::*;
use price_chart_wasm::app::{abort_other_streams, current_symbol, stream_abort_handles};
use price_chart_wasm::domain::market_data::Symbol;
use price_chart_wasm::global_state::domain_state;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn heap_usage() -> f64 {
    js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("performance"))
        .ok()
        .and_then(|perf| js_sys::Reflect::get(&perf, &JsValue::from_str("memory")).ok())
        .and_then(|mem| js_sys::Reflect::get(&mem, &JsValue::from_str("usedJSHeapSize")).ok())
        .and_then(|val| val.as_f64())
        .unwrap_or(0.0)
}

#[wasm_bindgen_test]
fn rapid_symbol_switch_no_leak() {
    let start = heap_usage();
    for i in 0..20 {
        let sym = if i % 2 == 0 { "BTCUSDT" } else { "ETHUSDT" };
        current_symbol().set(Symbol::from(sym));
        let (handle, _) = AbortHandle::new_pair();
        stream_abort_handles().update(|m| {
            m.insert(Symbol::from(sym), handle);
        });
        abort_other_streams(&Symbol::from(sym));
        domain_state().update(|ds| {
            ds.candles = Arc::new(Vec::new());
            ds.indicators = Arc::new(Vec::new());
        });
    }
    assert_eq!(stream_abort_handles().with(|m| m.len()), 1);
    let end = heap_usage();
    if start > 0.0 && end > 0.0 {
        let growth = (end - start) / start;
        assert!(growth <= 0.05);
    }
}
