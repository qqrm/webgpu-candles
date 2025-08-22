use price_chart_wasm::app::{HISTORY_PRELOAD_THRESHOLD, should_fetch_history};
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
#[test]
fn history_threshold_check() {
    assert!(should_fetch_history(HISTORY_PRELOAD_THRESHOLD - 1));
    assert!(!should_fetch_history(HISTORY_PRELOAD_THRESHOLD));
}
