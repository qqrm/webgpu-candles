use price_chart_wasm::infrastructure::rendering::renderer::{BASE_CANDLES, candle_x_position};

#[test]
fn candle_offset_calculation() {
    let visible = 10usize;
    let step = 2.0 / BASE_CANDLES;
    let expected_first = -1.0 + (BASE_CANDLES - visible as f32) * step + 0.5 * step;
    let x = candle_x_position(0, visible);
    assert!((x - expected_first).abs() < f32::EPSILON);

    let expected_last = -1.0 + (BASE_CANDLES - visible as f32) * step + (visible as f32 - 0.5) * step;
    let x_last = candle_x_position(visible - 1, visible);
    assert!((x_last - expected_last).abs() < f32::EPSILON);
}
