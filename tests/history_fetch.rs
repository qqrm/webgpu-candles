use price_chart_wasm::app::should_fetch_history;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
#[test]
fn history_threshold_check() {
    assert!(should_fetch_history(-60.0));
    assert!(!should_fetch_history(-10.0));
}
