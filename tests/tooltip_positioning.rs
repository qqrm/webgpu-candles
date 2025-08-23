#![cfg(feature = "render")]
use price_chart_wasm::infrastructure::rendering::renderer::{
    EDGE_GAP, MAX_ELEMENT_WIDTH, MIN_ELEMENT_WIDTH, candle_x_position, spacing_ratio_for,
};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn tooltip_reverse_positioning() {
    // Ensure the reverse formula finds the candle index by mouse coordinates
    let visible_len = 10;

    for expected_index in 0..visible_len {
        // Get x position for the candle
        let x = candle_x_position(expected_index, visible_len);

        // Apply the reverse formula (as in tooltip logic)
        let step_size = 2.0 / visible_len as f64;
        let spacing = spacing_ratio_for(visible_len) as f64;
        let width =
            (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH as f64, MAX_ELEMENT_WIDTH as f64);
        let half_width = width / 2.0;
        let calculated_index =
            visible_len as f64 - 1.0 - (1.0 - EDGE_GAP as f64 - half_width - x as f64) / step_size;
        let rounded_index = calculated_index.round() as usize;

        // Verify we obtained the same index
        assert_eq!(
            rounded_index, expected_index,
            "For index {expected_index}, x={x:.6}, calculated_index={calculated_index:.6}, rounded={rounded_index}"
        );
    }
}

#[wasm_bindgen_test]
fn tooltip_mouse_boundaries() {
    let visible_len = 5;
    let step_size = 2.0 / visible_len as f64;
    let spacing = spacing_ratio_for(visible_len) as f64;
    let width =
        (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH as f64, MAX_ELEMENT_WIDTH as f64);
    let half_width = width / 2.0;

    // Test extreme coordinates

    // Left boundary should return index 0 or negative
    let left_boundary = -1.0;
    let left_index =
        visible_len as f64 - 1.0 - (1.0 - EDGE_GAP as f64 - half_width - left_boundary) / step_size;
    assert!(left_index <= 0.0, "Left boundary should give index <= 0, got {left_index}");

    // Right boundary should return the last index or higher
    let right_boundary = 1.0;
    let right_index = visible_len as f64
        - 1.0
        - (1.0 - EDGE_GAP as f64 - half_width - right_boundary) / step_size;
    assert!(
        right_index >= (visible_len - 1) as f64,
        "Right boundary should give index >= {}, got {}",
        visible_len - 1,
        right_index
    );
}

#[wasm_bindgen_test]
fn tooltip_positioning_consistency() {
    // Verify tooltip positioning matches candle positioning
    let test_cases = vec![1, 2, 5, 10, 50, 100, 300];

    for &visible_len in &test_cases {
        let step_size = 2.0 / visible_len as f64;
        let spacing = spacing_ratio_for(visible_len) as f64;
        let width =
            (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH as f64, MAX_ELEMENT_WIDTH as f64);
        let half_width = width / 2.0;

        // For each candle check that tooltip finds the correct index
        for expected_index in 0..visible_len {
            let candle_x = candle_x_position(expected_index, visible_len);

            // Convert to NDC coordinates (as in real code)
            let ndc_x = candle_x as f64;

            // Apply logic from app.rs
            let index_float =
                visible_len as f64 - 1.0 - (1.0 - EDGE_GAP as f64 - half_width - ndc_x) / step_size;
            let calculated_index = index_float.round() as i32;

            // Ensure index is within bounds and correct
            assert!(
                calculated_index >= 0,
                "Index should be non-negative for visible_len={visible_len}, expected_index={expected_index}, got {calculated_index}"
            );
            assert!(
                (calculated_index as usize) < visible_len,
                "Index should be less than visible_len for visible_len={visible_len}, expected_index={expected_index}, got {calculated_index}"
            );
            assert_eq!(
                calculated_index as usize, expected_index,
                "Should find correct index for visible_len={visible_len}, expected_index={expected_index}, got {calculated_index}"
            );
        }
    }
}
