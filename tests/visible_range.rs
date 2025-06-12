use price_chart_wasm::app::visible_range;

#[test]
fn visible_range_basic() {
    assert_eq!(visible_range(1000, 1.0), (700, 300));
    assert_eq!(visible_range(50, 2.0), (0, 50));
}
