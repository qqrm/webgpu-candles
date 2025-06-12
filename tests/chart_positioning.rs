use price_chart_wasm::infrastructure::rendering::renderer::candle_x_position;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn chart_positioning_edge_cases() {
    // Тест различных размеров visible_len
    let test_cases = vec![1, 2, 3, 5, 10, 20, 50, 100, 300];

    for &visible_len in &test_cases {
        // Проверяем что последняя свеча всегда в позиции x=1.0
        let last_x = candle_x_position(visible_len - 1, visible_len);
        assert!(
            (last_x - 1.0).abs() < f32::EPSILON,
            "Last candle should be at x=1.0 for visible_len={}, got x={:.10}",
            visible_len,
            last_x
        );

        // Проверяем что первая свеча в правильной позиции
        let first_x = candle_x_position(0, visible_len);
        let expected_first = 1.0 - (visible_len as f32 - 1.0) * (2.0 / visible_len as f32);
        assert!(
            (first_x - expected_first).abs() < f32::EPSILON,
            "First candle position mismatch for visible_len={}: expected {:.6}, got {:.6}",
            visible_len,
            expected_first,
            first_x
        );

        // Проверяем что все позиции в правильном диапазоне
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
    // Специфический тест для привязки к правому краю
    let test_cases = vec![1, 5, 10, 50, 100, 300];

    for &visible_len in &test_cases {
        let last_position = candle_x_position(visible_len - 1, visible_len);

        // Последняя свеча должна быть ТОЧНО в x=1.0
        assert_eq!(
            last_position, 1.0,
            "Last candle must be exactly at x=1.0 for visible_len={}, got x={:.15}",
            visible_len, last_position
        );

        // Если есть предпоследняя свеча, она должна быть левее
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
    // Тест монотонности позиций
    let visible_len = 20;
    let mut positions = Vec::new();

    for i in 0..visible_len {
        positions.push(candle_x_position(i, visible_len));
    }

    // Проверяем строгое возрастание
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

    // Проверяем равномерность интервалов
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
