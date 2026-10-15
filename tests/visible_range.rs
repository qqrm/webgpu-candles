#![cfg(feature = "render")]
mod support;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
use price_chart_wasm::app::visible_range;

#[wasm_bindgen_test::wasm_bindgen_test]
fn visible_range_basic() {
    support::with_app_context(|| {
        assert_eq!(visible_range(1000, 1.0, 0.0), (968, 32));
        assert_eq!(visible_range(50, 2.0, 0.0), (34, 16));
    });
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn visible_range_with_pan() {
    support::with_app_context(|| {
        assert_eq!(visible_range(1000, 1.0, -50.0), (918, 32));
        assert_eq!(visible_range(100, 1.0, -200.0), (0, 32));
    });
}
