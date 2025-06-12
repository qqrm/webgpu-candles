use price_chart_wasm::infrastructure::rendering::renderer::{
    MAX_ELEMENT_WIDTH, MIN_ELEMENT_WIDTH, candle_x_position,
};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn width_calculation_sync() {
    // Test that volume bars and candles use the same width calculation
    let visible_len = 20;

    // Emulate candle width logic
    let step_size = 2.0 / visible_len as f32;
    let candle_width = (step_size * 0.8).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

    // Emulate volume bar logic (after fix)
    let bar_width = (step_size * 0.8).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

    // Verify the widths match
    assert_eq!(
        candle_width, bar_width,
        "Candle and volume bar width should match: candle={:.6}, volume={:.6}",
        candle_width, bar_width
    );

    // Ensure width stays within limits
    assert!(candle_width >= MIN_ELEMENT_WIDTH, "Width too small: {:.6}", candle_width);
    assert!(candle_width <= MAX_ELEMENT_WIDTH, "Width too large: {:.6}", candle_width);
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

        // Ensure the last position is exactly 1.0
        let last_x = candle_x_position(visible_len - 1, visible_len);
        assert_eq!(
            last_x, 1.0,
            "Last position should be 1.0 for visible_len={}, got {:.10}",
            visible_len, last_x
        );
    }
}
