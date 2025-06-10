use price_chart_wasm::infrastructure::rendering::renderer::candle_x_position;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn candle_offset_calculation() {
    let visible = 10usize;
    let step = 2.0 / visible as f32;
    let expected_first = 1.0 - (visible as f32 - 0.5) * step;
    let x = candle_x_position(0, visible);
    assert!((x - expected_first).abs() < f32::EPSILON);

    let expected_last = 1.0 - 0.5 * step;
    let x_last = candle_x_position(visible - 1, visible);
    assert!((x_last - expected_last).abs() < f32::EPSILON);
}
