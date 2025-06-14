use price_chart_wasm::infrastructure::rendering::renderer::{
    MAX_ELEMENT_WIDTH, MIN_ELEMENT_WIDTH, spacing_ratio_for,
};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn single_candle_pixel_width() {
    let canvas_width = 800.0;
    let _canvas_height = 500.0; // not used in width calculation
    let visible_len = 1;

    let step_size = 2.0 / visible_len as f32;
    let spacing = spacing_ratio_for(visible_len);
    let ndc_width = (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
    let pixel_width = ndc_width * canvas_width / 2.0;

    assert!(pixel_width > 2.0, "Single candle width should exceed 2px, got {:.2} px", pixel_width);
}
