use price_chart_wasm::infrastructure::rendering::gpu_structures::{CandleInstance, CandleVertex};
use wasm_bindgen_test::*;

fn apply_vs(v: &CandleVertex, inst: &CandleInstance) -> (f32, f32) {
    let x = inst.x + v.position_x * inst.width;
    let y = if v.element_type < 0.5 {
        inst.body_bottom + (inst.body_top - inst.body_bottom) * v.position_y
    } else if v.element_type < 1.5 {
        inst.body_top + (inst.high - inst.body_top) * v.position_y
    } else {
        inst.low + (inst.body_bottom - inst.low) * v.position_y
    };
    (x, y)
}

#[wasm_bindgen_test]
fn vertex_shader_formula() {
    let inst = CandleInstance {
        x: 0.3,
        width: 0.2,
        body_top: 0.6,
        body_bottom: 0.4,
        high: 0.7,
        low: 0.3,
        bullish: 1.0,
        _padding: 0.0,
    };

    let v = CandleVertex::body_vertex(-0.5, 0.0, true);
    let (x, y) = apply_vs(&v, &inst);
    assert!((x - 0.2).abs() < 1e-6);
    assert!((y - inst.body_bottom).abs() < 1e-6);

    let v = CandleVertex::body_vertex(0.5, 1.0, true);
    let (x, y) = apply_vs(&v, &inst);
    assert!((x - 0.4).abs() < 1e-6);
    assert!((y - inst.body_top).abs() < 1e-6);

    let v = CandleVertex::wick_vertex(0.0, 1.0);
    let (x, y) = apply_vs(&v, &inst);
    assert!((x - inst.x).abs() < 1e-6);
    assert!((y - inst.high).abs() < 1e-6);

    let v = CandleVertex { position_x: 0.0, position_y: 0.0, element_type: 2.0, color_type: 0.5 };
    let (x, y) = apply_vs(&v, &inst);
    assert!((x - inst.x).abs() < 1e-6);
    assert!((y - inst.low).abs() < 1e-6);
}
