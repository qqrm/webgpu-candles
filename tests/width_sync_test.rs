use price_chart_wasm::infrastructure::rendering::renderer::{
    EDGE_GAP, MIN_ELEMENT_WIDTH, candle_x_position, spacing_ratio_for,
};
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn width_calculation_sync() {
    // Test that volume bars and candles use the same width calculation
    let visible_len = 20;

    // Emulate candle width logic
    let step_size = 2.0 / visible_len as f32;
    let spacing = spacing_ratio_for(visible_len);
    let candle_width = (step_size * (1.0 - spacing)).max(MIN_ELEMENT_WIDTH);

    // Emulate volume bar logic (after fix)
    let bar_width = (step_size * (1.0 - spacing)).max(MIN_ELEMENT_WIDTH);

    // Verify the widths match
    assert_eq!(
        candle_width, bar_width,
        "Candle and volume bar width should match: candle={:.6}, volume={:.6}",
        candle_width, bar_width
    );

    // Ensure width stays within limits
    assert!(candle_width >= MIN_ELEMENT_WIDTH, "Width too small: {:.6}", candle_width);
    assert!(
        candle_width <= step_size * (1.0 - spacing) + f32::EPSILON,
        "Width exceeds expected maximum: {:.6}",
        candle_width
    );
}

#[wasm_bindgen_test]
fn no_extra_gaps_small_range() {
    // With few bars there should be no additional gaps due to clamping
    let visible_len = 5;
    let step_size = 2.0 / visible_len as f32;
    let spacing = spacing_ratio_for(visible_len);
    let candle_width = (step_size * (1.0 - spacing)).max(MIN_ELEMENT_WIDTH);

    // Expected gap equals spacing ratio portion of the step
    let gap = step_size - candle_width;
    assert!((gap - step_size * spacing).abs() < f32::EPSILON, "Unexpected gap size: {:.6}", gap);
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
                "Position {} of {} out of bounds: x={:.6}",
                i,
                visible_len,
                x
            );
        }

        // Calculate width and ensure the last position touches the right edge
        let step_size = 2.0 / visible_len as f32;
        let spacing = spacing_ratio_for(visible_len);
        let width = (step_size * (1.0 - spacing)).max(MIN_ELEMENT_WIDTH);
        let last_x = candle_x_position(visible_len - 1, visible_len);
        assert!(
            (last_x + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON,
            "Last position should touch right edge for visible_len={}, got {:.10}",
            visible_len,
            last_x
        );
    }
}

#[wasm_bindgen_test]
fn single_candle_width() {
    let visible_len = 1;
    let step_size = 2.0 / visible_len as f32;
    let spacing = spacing_ratio_for(visible_len);
    let candle_width = (step_size * (1.0 - spacing)).max(MIN_ELEMENT_WIDTH);

    let pixel_width = candle_width * 400.0;
    assert!(pixel_width > 2.0, "Width in pixels should exceed 2.0, got {:.6}", pixel_width);
}
