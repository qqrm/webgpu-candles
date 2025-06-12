use price_chart_wasm::infrastructure::rendering::renderer::candle_x_position;
use wasm_bindgen_test::*;

/// Регрессионный тест: проверяем что новая логика не сломала базовые принципы
#[wasm_bindgen_test]
fn positioning_regression_basic() {
    // Эти значения должны быть стабильными между версиями

    // Тест для 10 свечей
    let visible = 10;

    // Последняя свеча точно справа
    assert_eq!(candle_x_position(9, visible), 1.0);

    // Предпоследняя свеча левее последней
    assert!(candle_x_position(8, visible) < candle_x_position(9, visible));

    // Первая свеча левее всех остальных
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

/// Проверяем что изменения не сломали математику
#[wasm_bindgen_test]
fn positioning_regression_math() {
    let test_cases = vec![
        (1, vec![1.0]),                          // Одна свеча
        (2, vec![0.0, 1.0]),                     // Две свечи
        (3, vec![-0.33333334, 0.33333334, 1.0]), // Три свечи (с погрешностью float)
        (4, vec![-0.5, 0.0, 0.5, 1.0]),          // Четыре свечи
    ];

    for (visible_len, expected_positions) in test_cases {
        for (i, expected) in expected_positions.iter().enumerate() {
            let actual = candle_x_position(i, visible_len);
            assert!(
                (actual - expected).abs() < 1e-6,
                "Position mismatch for visible_len={}, index={}: expected {:.6}, got {:.6}",
                visible_len,
                i,
                expected,
                actual
            );
        }
    }
}

/// Тест совместимости tooltip логики
#[wasm_bindgen_test]
fn tooltip_compatibility_regression() {
    // Проверяем что tooltip логика работает с новым позиционированием
    let visible_len = 5;
    let step_size = 2.0 / visible_len as f64;

    // Для каждой позиции проверяем обратную конверсию
    for expected_index in 0..visible_len {
        let x = candle_x_position(expected_index, visible_len);

        // Применяем tooltip логику из app.rs
        let index_float = visible_len as f64 - (1.0 - x as f64) / step_size - 1.0;
        let calculated_index = index_float.round() as i32;

        assert!(calculated_index >= 0, "Calculated index should be non-negative");
        assert!((calculated_index as usize) < visible_len, "Calculated index should be in bounds");

        assert_eq!(
            calculated_index as usize, expected_index,
            "Tooltip should find correct candle for index {}: got {}",
            expected_index, calculated_index
        );
    }
}

/// Проверяем границы viewport
#[wasm_bindgen_test]
fn viewport_bounds_regression() {
    let test_sizes = vec![1, 2, 5, 10, 20, 50, 100, 300];

    for &size in &test_sizes {
        // Первая позиция не должна быть левее -1.0
        let first = candle_x_position(0, size);

        assert!(first >= -1.0, "First position {:.6} should be >= -1.0 for size {}", first, size);

        // Последняя позиция должна быть точно 1.0
        let last = candle_x_position(size - 1, size);
        assert_eq!(
            last, 1.0,
            "Last position should be exactly 1.0 for size {}, got {:.10}",
            size, last
        );

        // Все промежуточные позиции в границах
        for i in 0..size {
            let pos = candle_x_position(i, size);
            assert!(
                (-1.0..=1.0).contains(&pos),
                "Position {:.6} out of bounds [-1, 1] for size {} index {}",
                pos,
                size,
                i
            );
        }

        // Проверяем что используем viewport оптимально
        if size > 1 {
            let total_span = last - first;
            let expected_span = 2.0 * (size - 1) as f32 / size as f32;
            assert!(
                (total_span - expected_span).abs() < 1e-6,
                "Total span should be {:.6}, got {:.6} for size {}",
                expected_span,
                total_span,
                size
            );
        }
    }
}

/// Проверяем что спейсинг равномерный
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
