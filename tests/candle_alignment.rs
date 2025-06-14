use price_chart_wasm::infrastructure::rendering::renderer::candle_x_position;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn right_edge_alignment_basic() {
    for &visible_len in &[3usize, 10usize] {
        let pos = candle_x_position(visible_len - 1, visible_len);
        assert_eq!(pos, 1.0);
    }
}
