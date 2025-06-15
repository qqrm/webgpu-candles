use price_chart_wasm::app::should_auto_scroll;

#[test]
fn detects_right_edge() {
    assert!(should_auto_scroll(100, 2.0, 0.0));
    assert!(!should_auto_scroll(100, 2.0, -1.0));
}
