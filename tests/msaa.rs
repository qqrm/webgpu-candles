use price_chart_wasm::infrastructure::rendering::renderer::MSAA_SAMPLE_COUNT;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn msaa_sample_count_is_four() {
    assert_eq!(MSAA_SAMPLE_COUNT, 4);
}
