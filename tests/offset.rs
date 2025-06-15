use price_chart_wasm::infrastructure::rendering::renderer::{
    EDGE_GAP, MAX_ELEMENT_WIDTH, MIN_ELEMENT_WIDTH, candle_x_position, spacing_ratio_for,
};
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn candle_offset_calculation() {
    let visible = 10usize;
    let step = 2.0 / visible as f32;

    // First candle should be at position 1.0 - (visible-1) * step
    let expected_first = 1.0 - (visible as f32 - 1.0) * step;
    let x = candle_x_position(0, visible);
    assert!((x - expected_first).abs() < f32::EPSILON);

    // ✅ Last candle's right edge should align with 1.0
    let spacing = spacing_ratio_for(visible);
    let width = (step * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
    let x_last = candle_x_position(visible - 1, visible);
    assert!((x_last + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn candle_positioning_edge_cases() {
    // Test with a single candle - right edge alignment
    let step = 2.0 / 1.0_f32;
    let width_single =
        (step * (1.0 - spacing_ratio_for(1))).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
    let x_single = candle_x_position(0, 1);
    assert!((x_single + width_single / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON);

    // Test with two candles
    let step_two = 1.0;
    let width_two =
        (step_two * (1.0 - spacing_ratio_for(2))).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
    let x_first_of_two = candle_x_position(0, 2);
    let x_second_of_two = candle_x_position(1, 2);
    assert!(x_first_of_two < x_second_of_two); // order correct
    assert!((x_second_of_two + width_two / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON); // second right
}

#[wasm_bindgen_test]
fn single_candle_centered() {
    // When only one candle is visible it should still touch the right edge
    let step = 2.0;
    let width = (step * (1.0 - spacing_ratio_for(1))).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
    let pos = candle_x_position(0, 1);
    assert!((pos + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn candle_positioning_monotonic() {
    // Test that positions are monotonically increasing
    let visible = 5;
    let mut positions = Vec::new();

    for i in 0..visible {
        positions.push(candle_x_position(i, visible));
    }

    // Check that positions strictly increase
    for i in 1..positions.len() {
        assert!(
            positions[i] > positions[i - 1],
            "Position {} ({:.6}) should be greater than position {} ({:.6})",
            i,
            positions[i],
            i - 1,
            positions[i - 1]
        );
    }

    let step = 2.0 / visible as f32;
    let spacing = spacing_ratio_for(visible);
    let width = (step * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
    assert!((positions.last().unwrap() + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn single_candle_centered_duplicate() {
    let step = 2.0;
    let width = (step * (1.0 - spacing_ratio_for(1))).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
    let x = candle_x_position(0, 1);
    assert!((x + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON);
}
