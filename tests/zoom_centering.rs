use price_chart_wasm::domain::chart::value_objects::Viewport;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn zoom_pan_moves_toward_center() {
    let mut vp = Viewport {
        start_time: 0.0,
        end_time: 100.0,
        min_price: 0.0,
        max_price: 100.0,
        width: 800,
        height: 600,
    };

    vp.zoom(2.0, 0.25);
    vp.pan(-0.25, 0.0);

    assert!((vp.start_time - 0.0).abs() < 1e-6);
    assert!((vp.end_time - 50.0).abs() < 1e-6);
}
