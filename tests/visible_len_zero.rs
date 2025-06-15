use price_chart_wasm::infrastructure::rendering::renderer::candle_x_position;
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
#[should_panic]
fn candle_x_position_panics_on_zero_visible_len() {
    let _ = candle_x_position(0, 0);
}
