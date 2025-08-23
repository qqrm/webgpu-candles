#![cfg(feature = "render")]
use price_chart_wasm::infrastructure::rendering::renderer::{
    EDGE_GAP, MAX_ELEMENT_WIDTH, MIN_ELEMENT_WIDTH, candle_x_position, spacing_ratio_for,
};
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
#[wasm_bindgen_test]
fn right_edge_alignment_basic() {
    for &visible_len in &[3usize, 10usize] {
        let step = 2.0 / visible_len as f32;
        let spacing = spacing_ratio_for(visible_len);
        let width = (step * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
        let pos = candle_x_position(visible_len - 1, visible_len);
        assert!((pos + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON);
    }
}
