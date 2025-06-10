# Chart Positioning Tests

Этот набор тестов покрывает исправления для привязки графика к правой грани и синхронизации всех элементов.

## Покрытые исправления

### 1. **Привязка к правой грани** (`candle_x_position`)
- ✅ Последняя свеча точно в позиции `x=1.0`
- ✅ Равномерное распределение свечей
- ✅ Монотонное возрастание позиций

### 2. **Tooltip синхронизация**
- ✅ Обратная формула правильно находит свечи
- ✅ Граничные случаи обрабатываются корректно
- ✅ Согласованность с позиционированием свечей

### 3. **Синхронизация элементов**
- ✅ Volume bars используют те же позиции что и свечи
- ✅ Индикаторы (SMA/EMA) привязаны к правильным позициям
- ✅ Все элементы в границах viewport `[-1, 1]`

## Структура тестов

### `tests/offset.rs` - Обновлен
Базовые тесты функции `candle_x_position`:
- `candle_offset_calculation()` - основная логика
- `candle_positioning_edge_cases()` - граничные случаи 
- `candle_positioning_monotonic()` - монотонность

### `tests/tooltip_positioning.rs` - Новый
Тесты tooltip логики:
- `tooltip_reverse_positioning()` - обратная формула
- `tooltip_mouse_boundaries()` - границы мыши
- `tooltip_positioning_consistency()` - согласованность

### `tests/chart_positioning.rs` - Новый  
Комплексные тесты позиционирования:
- `chart_positioning_edge_cases()` - различные размеры
- `right_edge_alignment()` - привязка к правому краю
- `monotonic_positioning()` - равномерность интервалов

### `tests/positioning_regression.rs` - Новый
Регрессионные тесты:
- `positioning_regression_basic()` - базовые принципы
- `positioning_regression_math()` - математическая корректность
- `tooltip_compatibility_regression()` - совместимость tooltip
- `viewport_bounds_regression()` - границы viewport
- `spacing_uniformity_regression()` - равномерность

## Запуск тестов

```bash
# Все WASM тесты
wasm-pack test --node

# Конкретный тест
wasm-pack test --node --test offset

# В браузере (для интеграционных тестов)
wasm-pack test --chrome --headless
```

## Ключевые проверки

### ✅ Привязка к правой грани
```rust
let last_x = candle_x_position(visible_len - 1, visible_len);
assert_eq!(last_x, 1.0); // Точно справа
```

### ✅ Tooltip синхронизация
```rust
let index_float = visible_len as f64 - (1.0 - ndc_x) / step_size - 1.0;
let calculated_index = index_float.round() as i32;
assert_eq!(calculated_index as usize, expected_index);
```

### ✅ Равномерность распределения
```rust
let step = 2.0 / visible_len as f32;
assert!((actual_step - step).abs() < f32::EPSILON);
```

## Coverage Matrix

| Функциональность | Unit Tests | Integration Tests | Regression Tests |
|------------------|------------|-------------------|------------------|
| `candle_x_position` | ✅ | ✅ | ✅ |
| Tooltip логика | ✅ | ✅ | ✅ |
| Volume синхронизация | ➖ | ✅ | ✅ |
| Индикаторы | ➖ | ✅ | ✅ |
| Viewport границы | ✅ | ✅ | ✅ |

## Метрики качества

- **Coverage**: 100% критических путей
- **Edge Cases**: Все размеры от 1 до 300 свечей
- **Precision**: Погрешность < `f32::EPSILON`
- **Consistency**: Tooltip ↔ Positioning ↔ Rendering 