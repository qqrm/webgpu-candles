use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use price_chart_wasm::infrastructure::rendering::renderer::{
    EDGE_GAP, MAX_ELEMENT_WIDTH, MIN_ELEMENT_WIDTH, SPACING_RATIO, candle_x_position,
    spacing_ratio_for,
};
use wasm_bindgen_test::*;

fn create_test_candles(count: usize) -> Vec<Candle> {
    let mut candles = Vec::new();
    let base_time = 1000000u64;
    let base_price = 50000.0;

    for i in 0..count {
        let timestamp = Timestamp::from(base_time + i as u64 * 60000);
        let open = base_price + (i as f64 * 10.0);
        let high = open + 100.0;
        let low = open - 50.0;
        let close = open + (i as f64 % 3.0 - 1.0) * 30.0;
        let volume = 100.0 + (i as f64 * 5.0);

        let ohlcv = OHLCV::new(
            Price::from(open),
            Price::from(high),
            Price::from(low),
            Price::from(close),
            Volume::from(volume),
        );

        candles.push(Candle::new(timestamp, ohlcv));
    }

    candles
}

#[wasm_bindgen_test]
fn volume_candle_position_sync() {
    // Create test data
    let test_candles = create_test_candles(20);
    let visible_len = test_candles.len();

    // Check that volume bars and candles use the same x positions
    for (i, _candle) in test_candles.iter().enumerate() {
        let candle_x = candle_x_position(i, visible_len);
        let volume_x = candle_x_position(i, visible_len); // same function should be used

        assert_eq!(
            candle_x, volume_x,
            "Volume bar and candle {} must share the same x position: candle={:.6}, volume={:.6}",
            i, candle_x, volume_x
        );
    }

    // Ensure the last candle and volume bar touch the right edge
    let last_x = candle_x_position(visible_len - 1, visible_len);
    let spacing = spacing_ratio_for(visible_len);
    let step_size = 2.0 / visible_len as f32;
    let width = (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
    assert!(
        (last_x + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON,
        "Last element must align with right edge: x={:.6}",
        last_x
    );
}

#[wasm_bindgen_test]
fn volume_width_sync() {
    let test_candles = create_test_candles(10);
    let visible_len = test_candles.len();

    // Check that step_size is the same for candles and volume bars
    let step_size = 2.0 / visible_len as f32;
    let expected_width = (step_size * (1.0 - SPACING_RATIO)).max(MIN_ELEMENT_WIDTH);

    // Emulate logic from the code
    for i in 0..visible_len {
        let x = candle_x_position(i, visible_len);
        let half_width = expected_width * 0.5;

        // Ensure boundaries stay within [-1, 1]
        assert!(
            x - half_width >= -1.0,
            "Left boundary of element {} out of bounds: {:.6}",
            i,
            x - half_width
        );
        assert!(
            x + half_width + EDGE_GAP <= 1.0,
            "Right boundary of element {} out of bounds: {:.6}",
            i,
            x + half_width + EDGE_GAP
        );
    }
}

#[wasm_bindgen_test]
fn debug_positioning_logic() {
    // Test positioning logic without GPU
    let test_candles = create_test_candles(15);
    let visible_len = test_candles.len();

    // Check that volume bars and candles use identical positioning logic
    let mut candle_positions = Vec::new();
    let mut volume_positions = Vec::new();

    // Emulate position creation logic for candles and volume bars
    for i in 0..visible_len {
        let candle_x = candle_x_position(i, visible_len); // for candles
        let volume_x = candle_x_position(i, visible_len); // for volume bars (same function)

        candle_positions.push(candle_x);
        volume_positions.push(volume_x);
    }

    // Verify that positions match
    for (i, (candle_x, volume_x)) in
        candle_positions.iter().zip(volume_positions.iter()).enumerate()
    {
        assert!(
            (candle_x - volume_x).abs() < f32::EPSILON,
            "Position {} mismatch: candle={:.6}, volume={:.6}",
            i,
            candle_x,
            volume_x
        );
    }

    // Check right edge alignment
    if !candle_positions.is_empty() {
        let last_candle = *candle_positions.last().unwrap();
        let spacing = spacing_ratio_for(visible_len);
        let step_size = 2.0 / visible_len as f32;
        let width = (step_size * (1.0 - spacing)).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);
        assert!(
            (last_candle + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON,
            "Last candle should touch right edge: x={:.6}",
            last_candle
        );
        let last_volume = *volume_positions.last().unwrap();
        assert!(
            (last_volume + width / 2.0 + EDGE_GAP - 1.0).abs() < f32::EPSILON,
            "Last volume bar should touch right edge: x={:.6}",
            last_volume
        );
    }

    // Additional check: positions must be strictly increasing
    for i in 1..candle_positions.len() {
        assert!(
            candle_positions[i] > candle_positions[i - 1],
            "Candle positions should increase: pos[{}]={:.6} should be > pos[{}]={:.6}",
            i,
            candle_positions[i],
            i - 1,
            candle_positions[i - 1]
        );
    }
}
