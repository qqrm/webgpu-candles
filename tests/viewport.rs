use price_chart_wasm::domain::chart::value_objects::Viewport;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn zoom_changes_time_range() {
    let mut vp = Viewport {
        start_time: 0.0,
        end_time: 100.0,
        min_price: 0.0,
        max_price: 100.0,
        width: 800,
        height: 600,
    };
    vp.zoom(2.0, 0.5);
    assert!((vp.start_time - 25.0).abs() < 1e-6);
    assert!((vp.end_time - 75.0).abs() < 1e-6);
}

#[wasm_bindgen_test]
fn pan_moves_viewport() {
    let mut vp = Viewport {
        start_time: 0.0,
        end_time: 100.0,
        min_price: 0.0,
        max_price: 100.0,
        width: 800,
        height: 600,
    };
    vp.pan(0.1, 0.1);
    assert!((vp.start_time - 10.0).abs() < 1e-6);
    assert!((vp.end_time - 110.0).abs() < 1e-6);
    assert!((vp.min_price - 10.0).abs() < 1e-6);
    assert!((vp.max_price - 110.0).abs() < 1e-6);
}

#[wasm_bindgen_test]
fn time_to_x_calculates() {
    let vp = Viewport {
        start_time: 0.0,
        end_time: 100.0,
        min_price: 0.0,
        max_price: 100.0,
        width: 200,
        height: 100,
    };
    let x = vp.time_to_x(50.0);
    assert!((x - 100.0).abs() < 1e-6);
}

#[wasm_bindgen_test]
fn zoom_round_trip_preserves_viewport() {
    let mut vp = Viewport {
        start_time: 0.0,
        end_time: 100.0,
        min_price: 0.0,
        max_price: 100.0,
        width: 800,
        height: 600,
    };
    let original = vp.clone();
    vp.zoom(2.0, 0.5);
    vp.zoom(0.5, 0.5);
    assert!((vp.start_time - original.start_time).abs() < 1e-6);
    assert!((vp.end_time - original.end_time).abs() < 1e-6);
}
