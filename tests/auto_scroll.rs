#![cfg(feature = "render")]
mod support;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
use price_chart_wasm::app::should_auto_scroll;

#[wasm_bindgen_test::wasm_bindgen_test]
fn detects_right_edge() {
    support::with_app_context(|| {
        assert!(should_auto_scroll(100, 2.0, 0.0));
        assert!(!should_auto_scroll(100, 2.0, -1.0));
    });
}
