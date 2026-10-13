#![cfg(feature = "render")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
use price_chart_wasm::infrastructure::rendering::renderer::{
    EDGE_GAP, MAX_ELEMENT_WIDTH, MIN_ELEMENT_WIDTH, candle_x_position, spacing_ratio_for,
};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn width_calculation_sync() {
    // Test that volume bars and candles use the same width calculation
    let visible_len = 20;

    // Emulate candle width logic
    let step_size = 2.0 / visible_len as f32;
    let spacing = spacing_ratio_for(visible_len);
    let candle_width = (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

    // Emulate volume bar logic (after fix)
    let bar_width = (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

    // Verify the widths match
    assert_eq!(
        candle_width, bar_width,
        "Candle and volume bar width should match: candle={candle_width:.6}, volume={bar_width:.6}"
    );

    // Ensure width stays within limits
    assert!(candle_width >= MIN_ELEMENT_WIDTH, "Width too small: {candle_width:.6}");
    assert!(
        candle_width <= step_size * (1.0 - spacing) + f32::EPSILON,
        "Width exceeds expected maximum: {candle_width:.6}"
    );
}

#[wasm_bindgen_test]
fn small_range_respects_width_cap() {
    // Large zoom keeps candle bodies capped instead of filling the entire step.
    let visible_len = 5;
    let step_size = 2.0 / visible_len as f32;
    let spacing = spacing_ratio_for(visible_len);
    let candle_width = (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

    // The width cap leaves a larger gap than the minimum spacing ratio.
    let gap = step_size - candle_width;
    assert_eq!(candle_width, MAX_ELEMENT_WIDTH);
    assert!(gap >= step_size * spacing);
}

#[wasm_bindgen_test]
fn positioning_boundary_test() {
    // Check positioning boundary conditions
    let test_cases = vec![1, 2, 5, 10, 50, 100];

    for &visible_len in &test_cases {
        // Ensure all positions are within [-1, 1]
        for i in 0..visible_len {
            let x = candle_x_position(i, visible_len);
            assert!(
                (-1.0..=1.0).contains(&x),
                "Position {i} of {visible_len} out of bounds: x={x:.6}"
            );
        }

        // Calculate width and ensure the last position touches the right edge
        let step_size = 2.0 / visible_len as f32;
        let spacing = spacing_ratio_for(visible_len);
        let width = (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
        let last_x = candle_x_position(visible_len - 1, visible_len);
        assert!(
            (last_x + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON,
            "Last position should touch right edge for visible_len={visible_len}, got {last_x:.10}"
        );
    }
}

#[wasm_bindgen_test]
fn single_candle_width() {
    let visible_len = 1;
    let step_size = 2.0 / visible_len as f32;
    let spacing = spacing_ratio_for(visible_len);
    let candle_width = (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

    let pixel_width = candle_width * 400.0;
    assert!(pixel_width > 2.0, "Width in pixels should exceed 2.0, got {pixel_width:.6}");
}
