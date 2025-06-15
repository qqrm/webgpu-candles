use price_chart_wasm::infrastructure::rendering::gpu_structures::CandleGeometry;
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn volume_vertex_basic() {
    let verts = CandleGeometry::create_volume_vertices(0.0, 0.2, 1.0, true);
    assert_eq!(verts.len(), 6);
    for v in &verts {
        assert!((v.element_type - 5.0).abs() < f32::EPSILON);
    }
}

#[wasm_bindgen_test]
fn volume_height_scaling() {
    let low = CandleGeometry::create_volume_vertices(0.0, 0.2, 0.5, true);
    let high = CandleGeometry::create_volume_vertices(0.0, 0.2, 1.0, true);
    let low_top = low.iter().map(|v| v.position_y).fold(-1.0, f32::max);
    let high_top = high.iter().map(|v| v.position_y).fold(-1.0, f32::max);
    assert!(high_top > low_top);
}
