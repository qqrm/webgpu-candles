use price_chart_wasm::infrastructure::rendering::renderer::{
    EDGE_GAP, MAX_ELEMENT_WIDTH, MIN_ELEMENT_WIDTH, candle_x_position, spacing_ratio_for,
};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn chart_positioning_edge_cases() {
    // Test various visible_len sizes
    let test_cases = vec![1, 2, 3, 5, 10, 20, 50, 100, 300];

    for &visible_len in &test_cases {
        // Ensure the last candle touches the right edge
        let last_x = candle_x_position(visible_len - 1, visible_len);
        let step = 2.0 / visible_len as f32;
        let spacing = spacing_ratio_for(visible_len);
        let width = (step * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
        assert!(
            (last_x + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON,
            "Last candle should touch right edge for visible_len={}, got x={:.10}",
            visible_len,
            last_x
        );

        // Ensure the first candle is in the correct position
        let first_x = candle_x_position(0, visible_len);
        let expected_first =
            1.0 - (visible_len as f32 - 1.0) * (2.0 / visible_len as f32) - width / 2.0 - EDGE_GAP;
        assert!(
            (first_x - expected_first).abs() < f32::EPSILON,
            "First candle position mismatch for visible_len={}: expected {:.6}, got {:.6}",
            visible_len,
            expected_first,
            first_x
        );

        // Ensure all positions are within the correct range
        for i in 0..visible_len {
            let x = candle_x_position(i, visible_len);
            assert!(
                (-1.0..=1.0).contains(&x),
                "Position out of bounds for visible_len={}, index={}: x={:.6}",
                visible_len,
                i,
                x
            );
        }
    }
}

#[wasm_bindgen_test]
fn right_edge_alignment() {
    // Specific test for right edge alignment
    let test_cases = vec![1, 5, 10, 50, 100, 300];

    for &visible_len in &test_cases {
        let last_position = candle_x_position(visible_len - 1, visible_len);
        let step = 2.0 / visible_len as f32;
        let spacing = spacing_ratio_for(visible_len);
        let width = (step * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

        // The last candle must touch the right edge
        assert!(
            (last_position + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON,
            "Last candle must touch right edge for visible_len={}, got x={:.15}",
            visible_len,
            last_position
        );

        // If there is a penultimate candle, it should be to the left
        if visible_len > 1 {
            let second_last = candle_x_position(visible_len - 2, visible_len);
            assert!(
                second_last < 1.0,
                "Second-to-last candle should be < 1.0 for visible_len={}, got x={:.6}",
                visible_len,
                second_last
            );
        }
    }
}

#[wasm_bindgen_test]
fn monotonic_positioning() {
    // Test position monotonicity
    let visible_len = 20;
    let mut positions = Vec::new();

    for i in 0..visible_len {
        positions.push(candle_x_position(i, visible_len));
    }

    // Ensure strict increase
    for i in 1..positions.len() {
        assert!(
            positions[i] > positions[i - 1],
            "Positions should be strictly increasing: pos[{}]={:.6} should be > pos[{}]={:.6}",
            i,
            positions[i],
            i - 1,
            positions[i - 1]
        );
    }

    // Ensure uniform intervals
    let step = 2.0 / visible_len as f32;
    for i in 1..positions.len() {
        let actual_step = positions[i] - positions[i - 1];
        assert!(
            (actual_step - step).abs() < f32::EPSILON,
            "Step size should be uniform: expected {:.6}, got {:.6} between pos[{}] and pos[{}]",
            step,
            actual_step,
            i - 1,
            i
        );
    }
}
