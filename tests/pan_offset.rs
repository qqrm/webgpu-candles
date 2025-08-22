use price_chart_wasm::app::{HISTORY_PRELOAD_THRESHOLD, should_fetch_history};

#[test]
fn start_index_triggers_history() {
    assert!(should_fetch_history(HISTORY_PRELOAD_THRESHOLD - 5));
    assert!(!should_fetch_history(HISTORY_PRELOAD_THRESHOLD + 5));
}
