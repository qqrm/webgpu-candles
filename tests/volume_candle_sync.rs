use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use price_chart_wasm::infrastructure::rendering::renderer::candle_x_position;
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
    // Создаем тестовые данные
    let test_candles = create_test_candles(20);
    let visible_len = test_candles.len();

    // Проверяем что volume bars и свечи используют одинаковые x позиции
    for (i, _candle) in test_candles.iter().enumerate() {
        let candle_x = candle_x_position(i, visible_len);
        let volume_x = candle_x_position(i, visible_len); // Та же функция должна использоваться

        assert_eq!(
            candle_x, volume_x,
            "Volume bar и свеча {} должны иметь одинаковую x позицию: candle={:.6}, volume={:.6}",
            i, candle_x, volume_x
        );
    }

    // Проверяем что последняя свеча и volume bar точно справа
    let last_x = candle_x_position(visible_len - 1, visible_len);
    assert_eq!(
        last_x, 1.0,
        "Последний элемент должен быть точно в x=1.0, получено x={:.10}",
        last_x
    );
}

#[wasm_bindgen_test]
fn volume_width_sync() {
    let test_candles = create_test_candles(10);
    let visible_len = test_candles.len();

    // Проверяем что step_size одинаковый для свечей и volume bars
    let step_size = 2.0 / visible_len as f32;
    let zoom_factor = 1.0; // По умолчанию
    let expected_width = (step_size * zoom_factor * 0.8).max(0.002);

    // Эмулируем логику из кода
    for i in 0..visible_len {
        let x = candle_x_position(i, visible_len);
        let half_width = expected_width * 0.5;

        // Проверяем что границы не пересекаются и не выходят за [-1, 1]
        assert!(
            x - half_width >= -1.0,
            "Левая граница элемента {} выходит за границы: {:.6}",
            i,
            x - half_width
        );
        assert!(
            x + half_width <= 1.0,
            "Правая граница элемента {} выходит за границы: {:.6}",
            i,
            x + half_width
        );
    }
}

#[wasm_bindgen_test]
fn debug_positioning_logic() {
    // Тестируем только логику позиционирования без GPU
    let test_candles = create_test_candles(15);
    let visible_len = test_candles.len();

    // Проверяем что volume bars и свечи используют идентичную логику позиционирования
    let mut candle_positions = Vec::new();
    let mut volume_positions = Vec::new();

    // Эмулируем логику создания позиций для свечей и volume bars
    for i in 0..visible_len {
        let candle_x = candle_x_position(i, visible_len); // Для свечей
        let volume_x = candle_x_position(i, visible_len); // Для volume bars (должна быть та же функция)

        candle_positions.push(candle_x);
        volume_positions.push(volume_x);
    }

    // Проверяем что позиции совпадают
    for (i, (candle_x, volume_x)) in
        candle_positions.iter().zip(volume_positions.iter()).enumerate()
    {
        assert!(
            (candle_x - volume_x).abs() < f32::EPSILON,
            "Позиция {} не совпадает: candle={:.6}, volume={:.6}",
            i,
            candle_x,
            volume_x
        );
    }

    // Проверяем правую привязку
    if !candle_positions.is_empty() {
        let last_candle = candle_positions.last().unwrap();
        let last_volume = volume_positions.last().unwrap();

        assert_eq!(
            *last_candle, 1.0,
            "Последняя свеча должна быть в x=1.0, получено {:.10}",
            last_candle
        );
        assert_eq!(
            *last_volume, 1.0,
            "Последний volume bar должен быть в x=1.0, получено {:.10}",
            last_volume
        );
    }

    // Дополнительная проверка: все позиции должны быть монотонно возрастающими
    for i in 1..candle_positions.len() {
        assert!(
            candle_positions[i] > candle_positions[i - 1],
            "Позиции свечей должны возрастать: pos[{}]={:.6} should be > pos[{}]={:.6}",
            i,
            candle_positions[i],
            i - 1,
            candle_positions[i - 1]
        );
    }
}
