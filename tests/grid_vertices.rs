use price_chart_wasm::infrastructure::rendering::gpu_structures::CandleGeometry;
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn grid_vertex_count_and_bounds() {
    let verts = CandleGeometry::create_grid_vertices(800.0, 600.0, 4, 3);
    assert_eq!(verts.len(), ((4 + 1) + (3 + 1)) * 6);

    for v in &verts {
        assert!(v.position_x >= -1.0 - f32::EPSILON && v.position_x <= 1.0 + f32::EPSILON);
        assert!(v.position_y >= -1.0 - f32::EPSILON && v.position_y <= 1.0 + f32::EPSILON);
    }
}
