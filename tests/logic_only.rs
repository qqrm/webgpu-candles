#![cfg(feature = "logic-only")]

use price_chart_wasm::time_utils::format_time_label;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn format_time_label_basic() {
    assert_eq!(format_time_label(0, 2.0), "00:00");
    assert_eq!(format_time_label(0, 1.5), "01.01");
    assert_eq!(format_time_label(0, 0.5), "01.1970");
}
