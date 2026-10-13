#![cfg(feature = "render")]
use price_chart_wasm::infrastructure::rendering::gpu_structures::CandleGeometry;
use wasm_bindgen_test::*;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn grid_vertex_count_and_bounds() {
    let vertices = CandleGeometry::create_grid_vertices(800.0, 600.0, 4, 3);
    assert_eq!(vertices.len(), ((4 + 1) + (3 + 1)) * 6);

    // Border lines are centered at +/-1. Their outer half is clipped by the GPU.
    let half_stroke = 0.001;
    let limit = 1.0 + half_stroke + f32::EPSILON;
    for (i, v) in vertices.iter().enumerate() {
        if !(v.position_x >= -limit && v.position_x <= limit) {
            panic!(
                "Vertex {} position_x out of bounds: {}. Full vertex: {:#?}",
                i, v.position_x, v
            );
        }
        if !(v.position_y >= -limit && v.position_y <= limit) {
            panic!(
                "Vertex {} position_y out of bounds: {}. Full vertex: {:#?}",
                i, v.position_y, v
            );
        }
    }

    for (index, line) in vertices.as_chunks::<6>().0.iter().enumerate() {
        let (center, low, high) = if index <= 4 {
            (index as f32 / 4.0 * 2.0 - 1.0, line[0].position_x, line[1].position_x)
        } else {
            ((index - 5) as f32 / 3.0 * 2.0 - 1.0, line[0].position_y, line[2].position_y)
        };
        assert!(((low + high) * 0.5 - center).abs() <= f32::EPSILON);
        assert!((high - low - half_stroke * 2.0).abs() <= f32::EPSILON);
    }
}
