#![cfg(feature = "render")]
use price_chart_wasm::infrastructure::rendering::renderer::{
    EDGE_GAP, MAX_ELEMENT_WIDTH, MIN_ELEMENT_WIDTH, candle_x_position, spacing_ratio_for,
};
use wasm_bindgen_test::*;

// Regression test: ensure new logic didn't break basics

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
#[wasm_bindgen_test]
fn positioning_regression_basic() {
    // These values must remain stable across versions

    // Test for 10 candles
    let visible = 10;

    // Last candle exactly at the right
    let step = 2.0 / visible as f32;
    let spacing = spacing_ratio_for(visible);
    let width = (step * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
    let last = candle_x_position(9, visible);
    assert!((last + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON);

    // Penultimate candle to the left of the last
    assert!(candle_x_position(8, visible) < candle_x_position(9, visible));

    // First candle left of all others
    let first = candle_x_position(0, visible);
    for i in 1..visible {
        assert!(
            first < candle_x_position(i, visible),
            "First position {:.6} should be less than position {} ({:.6})",
            first,
            i,
            candle_x_position(i, visible)
        );
    }
}

/// Check that changes didn't break math
#[wasm_bindgen_test]
fn positioning_regression_math() {
    let test_cases = vec![
        (1, vec![1.0]),                          // one candle
        (2, vec![0.0, 1.0]),                     // two candles
        (3, vec![-0.33333334, 0.33333334, 1.0]), // three candles (float error)
        (4, vec![-0.5, 0.0, 0.5, 1.0]),          // four candles
    ];

    for (visible_len, expected_positions) in test_cases {
        for (i, expected) in expected_positions.iter().enumerate() {
            let actual = candle_x_position(i, visible_len);
            assert!(
                (actual - expected).abs() < 1e-6,
                "Position mismatch for visible_len={visible_len}, index={i}: expected {expected:.6}, got {actual:.6}"
            );
        }
    }
}

/// Tooltip logic compatibility test
#[wasm_bindgen_test]
fn tooltip_compatibility_regression() {
    // Ensure tooltip logic works with new positioning
    let visible_len = 5;
    let step_size = 2.0 / visible_len as f64;

    // For each position check reverse conversion
    for expected_index in 0..visible_len {
        let x = candle_x_position(expected_index, visible_len);

        // Apply tooltip logic from app.rs
        let index_float = visible_len as f64 - (1.0 - x as f64) / step_size - 1.0;
        let calculated_index = index_float.round() as i32;

        assert!(calculated_index >= 0, "Calculated index should be non-negative");
        assert!((calculated_index as usize) < visible_len, "Calculated index should be in bounds");

        assert_eq!(
            calculated_index as usize, expected_index,
            "Tooltip should find correct candle for index {expected_index}: got {calculated_index}"
        );
    }
}

/// Check viewport bounds
#[wasm_bindgen_test]
fn viewport_bounds_regression() {
    let test_sizes = vec![1, 2, 5, 10, 20, 50, 100, 300];

    for &size in &test_sizes {
        // First position should not be left of -1.0
        let first = candle_x_position(0, size);

        assert!(first >= -1.0, "First position {first:.6} should be >= -1.0 for size {size}");

        // Last position must be exactly 1.0
        let step = 2.0 / size as f32;
        let spacing = spacing_ratio_for(size);
        let width = (step * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
        let last = candle_x_position(size - 1, size);
        assert!(
            (last + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON,
            "Last position should be exactly 1.0 for size {size}, got {last:.10}"
        );

        // All intermediate positions within bounds
        for i in 0..size {
            let pos = candle_x_position(i, size);
            assert!(
                (-1.0..=1.0).contains(&pos),
                "Position {pos:.6} out of bounds [-1, 1] for size {size} index {i}"
            );
        }

        // Ensure we use the viewport optimally
        if size > 1 {
            let total_span = last - first;
            let expected_span = 2.0 * (size - 1) as f32 / size as f32;
            assert!(
                (total_span - expected_span).abs() < 1e-6,
                "Total span should be {expected_span:.6}, got {total_span:.6} for size {size}"
            );
        }
    }
}

/// Ensure spacing is uniform
#[wasm_bindgen_test]
fn spacing_uniformity_regression() {
    let sizes = vec![2, 3, 5, 10, 20, 50];

    for &size in &sizes {
        let expected_step = 2.0 / size as f32;

        for i in 1..size {
            let prev_pos = candle_x_position(i - 1, size);
            let curr_pos = candle_x_position(i, size);
            let actual_step = curr_pos - prev_pos;

            assert!(
                (actual_step - expected_step).abs() < f32::EPSILON,
                "Step size mismatch for size {} between positions {} and {}: expected {:.6}, got {:.6}",
                size,
                i - 1,
                i,
                expected_step,
                actual_step
            );
        }
    }
}
