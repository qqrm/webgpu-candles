use price_chart_wasm::domain::chart::value_objects::Viewport;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn zoom_price_not_negative() {
    let mut vp = Viewport {
        start_time: 0.0,
        end_time: 100.0,
        min_price: 10.0,
        max_price: 20.0,
        width: 800,
        height: 600,
    };

    for _ in 0..3 {
        vp.zoom_price(1.1, 1.0);
    }
    assert!(vp.min_price >= 0.1);
}
