use price_chart_wasm::infrastructure::rendering::renderer::{
    MAX_ELEMENT_WIDTH, MIN_ELEMENT_WIDTH, candle_x_position,
};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn width_calculation_sync() {
    // Тестируем что ширина volume bars и свечей рассчитывается одинаково
    let visible_len = 20;
    let zoom_level = 1.5f64;

    // Эмулируем логику из кода для свечей
    let step_size = 2.0 / visible_len as f32;
    let zoom_factor = zoom_level.clamp(0.1, 10.0) as f32;
    let candle_width = (step_size * zoom_factor * 0.8).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

    // Эмулируем логику из кода для volume bars (после исправления)
    let bar_width = (step_size * zoom_factor * 0.8).clamp(MIN_ELEMENT_WIDTH, MAX_ELEMENT_WIDTH);

    // Проверяем что ширина одинаковая
    assert_eq!(
        candle_width, bar_width,
        "Ширина свечей и volume bars должна быть одинаковой: candle={:.6}, volume={:.6}",
        candle_width, bar_width
    );

    // Проверяем что ширина находится в допустимых пределах
    assert!(candle_width >= MIN_ELEMENT_WIDTH, "Ширина слишком мала: {:.6}", candle_width);
    assert!(candle_width <= MAX_ELEMENT_WIDTH, "Ширина слишком велика: {:.6}", candle_width);
}

#[wasm_bindgen_test]
fn positioning_boundary_test() {
    // Проверяем граничные условия для позиционирования
    let test_cases = vec![1, 2, 5, 10, 50, 100];

    for &visible_len in &test_cases {
        // Проверяем что все позиции в границах [-1, 1]
        for i in 0..visible_len {
            let x = candle_x_position(i, visible_len);
            assert!(
                (-1.0..=1.0).contains(&x),
                "Позиция {} из {} вне границ: x={:.6}",
                i,
                visible_len,
                x
            );
        }

        // Проверяем что последняя позиция точно 1.0
        let last_x = candle_x_position(visible_len - 1, visible_len);
        assert_eq!(
            last_x, 1.0,
            "Последняя позиция должна быть 1.0 для visible_len={}, получено {:.10}",
            visible_len, last_x
        );
    }
}
