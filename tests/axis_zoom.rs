use price_chart_wasm::app::price_levels;
use price_chart_wasm::domain::chart::value_objects::Viewport;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn price_levels_change_after_zoom() {
    let mut vp = Viewport {
        start_time: 0.0,
        end_time: 100.0,
        min_price: 0.0,
        max_price: 100.0,
        width: 800,
        height: 600,
    };

    let original = price_levels(&vp);
    vp.zoom_price(2.0, 0.5);
    let zoomed = price_levels(&vp);

    assert_ne!(original, zoomed);
    assert!((zoomed[0] - 75.0).abs() < 1e-6);
    assert!((zoomed[8] - 25.0).abs() < 1e-6);
}
