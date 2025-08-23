#![cfg(feature = "render")]
use price_chart_wasm::app::visible_range;

#[test]
fn visible_range_basic() {
    assert_eq!(visible_range(1000, 1.0, 0.0), (968, 32));
    assert_eq!(visible_range(50, 2.0, 0.0), (34, 16));
}

#[test]
fn visible_range_with_pan() {
    assert_eq!(visible_range(1000, 1.0, -50.0), (918, 32));
    assert_eq!(visible_range(100, 1.0, -200.0), (0, 32));
}
