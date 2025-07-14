use price_chart_wasm::infrastructure::rendering::renderer::{MIN_ELEMENT_WIDTH, SPACING_RATIO};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn candle_width_at_max_zoom() {
    let max_zoom: f64 = 32.0;
    let visible = ((32.0f64 / max_zoom).max(1.0)) as usize;
    let step_size = 2.0 / visible as f32;
    let candle_width = (step_size * (1.0 - SPACING_RATIO)).max(MIN_ELEMENT_WIDTH);
    let pixel_width = candle_width * 400.0; // canvas width 800 -> half width per NDC unit
    assert!(pixel_width >= 30.0, "width {:.2} too small", pixel_width);
}
