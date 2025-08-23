#![cfg(feature = "render")]
use price_chart_wasm::infrastructure::rendering::gpu_structures::CandleGeometry;
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn grid_vertex_count_and_bounds() {
    let vertices = CandleGeometry::create_grid_vertices(800.0, 600.0, 4, 3);
    assert_eq!(vertices.len(), ((4 + 1) + (3 + 1)) * 6);

    for (i, v) in vertices.iter().enumerate() {
        if !(v.position_x >= -1.0 - f32::EPSILON && v.position_x <= 1.0 + f32::EPSILON) {
            panic!(
                "Vertex {} position_x out of bounds: {}. Full vertex: {:#?}",
                i, v.position_x, v
            );
        }
        if !(v.position_y >= -1.0 - f32::EPSILON && v.position_y <= 1.0 + f32::EPSILON) {
            panic!(
                "Vertex {} position_y out of bounds: {}. Full vertex: {:#?}",
                i, v.position_y, v
            );
        }
    }
}
