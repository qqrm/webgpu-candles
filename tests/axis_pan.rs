use price_chart_wasm::app::price_levels;
use price_chart_wasm::domain::chart::value_objects::Viewport;
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn price_levels_change_after_pan() {
    let mut vp = Viewport {
        start_time: 0.0,
        end_time: 100.0,
        min_price: 0.0,
        max_price: 100.0,
        width: 800,
        height: 600,
        zoom_level: 1.0,
        pan_offset: 0.0,
    };

    let original = price_levels(&vp);
    vp.pan(0.0, 0.1);
    let moved = price_levels(&vp);

    assert_ne!(original, moved);
    assert!((moved[0] - 110.0).abs() < 1e-6);
    assert!((moved[8] - 10.0).abs() < 1e-6);
}
