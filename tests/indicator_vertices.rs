use price_chart_wasm::infrastructure::rendering::gpu_structures::{CandleGeometry, IndicatorType};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn current_price_line_vertices() {
    let verts = CandleGeometry::create_current_price_line(0.5, 0.2);
    assert_eq!(verts.len(), 6);
    assert!((verts[0].position_x + 1.0).abs() < f32::EPSILON);
    assert!((verts[0].position_y - 0.4).abs() < f32::EPSILON);
    assert!((verts[0].color_type - 7.0).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn indicator_line_vertex_count() {
    let points = [(-1.0, 0.0), (0.0, 0.5), (1.0, 0.0)];
    let verts = CandleGeometry::create_indicator_line_vertices(&points, IndicatorType::SMA20, 0.1);
    assert_eq!(verts.len(), (points.len() - 1) * 6);
    assert!((verts[0].color_type - 2.0).abs() < f32::EPSILON);
}
