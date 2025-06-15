use price_chart_wasm::app::{HISTORY_FETCH_THRESHOLD, PAN_SENSITIVITY_BASE, should_fetch_history};

#[test]
fn pan_direction_and_history_activation() {
    let mut offset = 0.0;
    let delta_x = 10.0;
    let zoom_level = 1.0;
    let pan_sensitivity = PAN_SENSITIVITY_BASE / zoom_level;

    // Simulate dragging to the right
    offset -= delta_x * pan_sensitivity;
    assert!(offset < 0.0);
    assert!(!should_fetch_history(offset));

    // Drag far enough to cross the history threshold
    let mut offset_big = 0.0;
    let delta_x_big = (HISTORY_FETCH_THRESHOLD.abs() + 1.0) / pan_sensitivity;
    offset_big -= delta_x_big * pan_sensitivity;
    assert!(should_fetch_history(offset_big));
}
