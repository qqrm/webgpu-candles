#![cfg(feature = "render")]
mod support;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
use price_chart_wasm::app::{HISTORY_PRELOAD_THRESHOLD, should_fetch_history};

#[wasm_bindgen_test::wasm_bindgen_test]
fn start_index_triggers_history() {
    support::with_app_context(|| {
        assert!(should_fetch_history(HISTORY_PRELOAD_THRESHOLD - 5));
        assert!(!should_fetch_history(HISTORY_PRELOAD_THRESHOLD + 5));
    });
}
