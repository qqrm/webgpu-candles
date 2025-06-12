use price_chart_wasm::infrastructure::rendering::renderer::candle_x_position;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn candle_offset_calculation() {
    let visible = 10usize;
    let step = 2.0 / visible as f32;

    // First candle should be at position 1.0 - (visible-1) * step
    let expected_first = 1.0 - (visible as f32 - 1.0) * step;
    let x = candle_x_position(0, visible);
    assert!((x - expected_first).abs() < f32::EPSILON);

    // ✅ Last candle must now be EXACTLY at position x=1.0 (right edge)
    let expected_last = 1.0;
    let x_last = candle_x_position(visible - 1, visible);
    assert!((x_last - expected_last).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn candle_positioning_edge_cases() {
    // Test with a single candle - should be centered (x=1.0)
    let x_single = candle_x_position(0, 1);
    assert!((x_single - 1.0).abs() < f32::EPSILON);

    // Test with two candles
    let x_first_of_two = candle_x_position(0, 2);
    let x_second_of_two = candle_x_position(1, 2);
    assert!(x_first_of_two < x_second_of_two); // order correct
    assert!((x_second_of_two - 1.0).abs() < f32::EPSILON); // second exactly right
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

    // Ensure the last position is exactly 1.0
    assert!((positions.last().unwrap() - 1.0).abs() < f32::EPSILON);
}
