use price_chart_wasm::infrastructure::rendering::renderer::spacing_ratio_for;
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn spacing_decreases_on_zoom_in() {
    let wide = spacing_ratio_for(100);
    let zoomed = spacing_ratio_for(10);
    assert!(zoomed < wide, "Spacing should decrease when fewer candles are visible");
}
