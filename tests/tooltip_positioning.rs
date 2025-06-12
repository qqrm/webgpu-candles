use price_chart_wasm::infrastructure::rendering::renderer::candle_x_position;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn tooltip_reverse_positioning() {
    // Тестируем что обратная формула правильно находит индекс свечи по координатам мыши
    let visible_len = 10;

    for expected_index in 0..visible_len {
        // Получаем x позицию для свечи
        let x = candle_x_position(expected_index, visible_len);

        // Применяем обратную формулу (как в tooltip логике)
        let step_size = 2.0 / visible_len as f64;
        let calculated_index = visible_len as f64 - (1.0 - x as f64) / step_size - 1.0;
        let rounded_index = calculated_index.round() as usize;

        // Проверяем что получили тот же индекс
        assert_eq!(
            rounded_index, expected_index,
            "For index {}, x={:.6}, calculated_index={:.6}, rounded={}",
            expected_index, x, calculated_index, rounded_index
        );
    }
}

#[wasm_bindgen_test]
fn tooltip_mouse_boundaries() {
    let visible_len = 5;
    let step_size = 2.0 / visible_len as f64;

    // Тест крайних координат

    // Левая граница - должна дать индекс 0 или отрицательный
    let left_boundary = -1.0;
    let left_index = visible_len as f64 - (1.0 - left_boundary) / step_size - 1.0;
    assert!(left_index <= 0.0, "Left boundary should give index <= 0, got {}", left_index);

    // Правая граница - должна дать последний индекс или больше
    let right_boundary = 1.0;
    let right_index = visible_len as f64 - (1.0 - right_boundary) / step_size - 1.0;
    assert!(
        right_index >= (visible_len - 1) as f64,
        "Right boundary should give index >= {}, got {}",
        visible_len - 1,
        right_index
    );
}

#[wasm_bindgen_test]
fn tooltip_positioning_consistency() {
    // Проверяем что позиционирование tooltip согласовано с позиционированием свечей
    let test_cases = vec![1, 2, 5, 10, 50, 100, 300];

    for &visible_len in &test_cases {
        let step_size = 2.0 / visible_len as f64;

        // Для каждой свечи проверяем что tooltip найдет правильный индекс
        for expected_index in 0..visible_len {
            let candle_x = candle_x_position(expected_index, visible_len);

            // Конвертируем в NDC координаты (как в реальном коде)
            let ndc_x = candle_x as f64;

            // Применяем логику из app.rs
            let index_float = visible_len as f64 - (1.0 - ndc_x) / step_size - 1.0;
            let calculated_index = index_float.round() as i32;

            // Проверяем что индекс в допустимых границах и корректный
            assert!(
                calculated_index >= 0,
                "Index should be non-negative for visible_len={}, expected_index={}, got {}",
                visible_len,
                expected_index,
                calculated_index
            );
            assert!(
                (calculated_index as usize) < visible_len,
                "Index should be less than visible_len for visible_len={}, expected_index={}, got {}",
                visible_len,
                expected_index,
                calculated_index
            );
            assert_eq!(
                calculated_index as usize, expected_index,
                "Should find correct index for visible_len={}, expected_index={}, got {}",
                visible_len, expected_index, calculated_index
            );
        }
    }
}
