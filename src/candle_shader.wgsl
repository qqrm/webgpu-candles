// Uniform буфер с параметрами графика
struct ChartUniforms {
    view_proj_matrix: mat4x4<f32>,
    viewport: vec4<f32>,          // width, height, min_price, max_price
    time_range: vec4<f32>,        // start_time, end_time, time_range, _padding
    bullish_color: vec4<f32>,     // Цвет бычьих свечей (зеленый)
    bearish_color: vec4<f32>,     // Цвет медвежьих свечей (красный)
    wick_color: vec4<f32>,        // Цвет фитилей (серый)
    sma20_color: vec4<f32>,       // Цвет SMA 20 (красный)
    sma50_color: vec4<f32>,       // Цвет SMA 50 (желтый)
    sma200_color: vec4<f32>,      // Цвет SMA 200 (синий)
    ema12_color: vec4<f32>,       // Цвет EMA 12 (фиолетовый)
    ema26_color: vec4<f32>,       // Цвет EMA 26 (голубой)
    current_price_color: vec4<f32>, // 💰 Цвет текущей цены (ярко-желтый)
    render_params: vec4<f32>,     // candle_width, spacing, line_width, _padding
}

@group(0) @binding(0)
var<uniform> uniforms: ChartUniforms;

// Вершинные атрибуты
struct VertexInput {
    @location(0) position_x: f32,    // X позиция в нормализованных координатах
    @location(1) position_y: f32,    // Y позиция в нормализованных координатах
    @location(2) element_type: f32,  // 0.0 = тело свечи, 1.0 = фитиль, 2.0 = индикатор, 3.0 = сетка, 4.0 = current price
    @location(3) color_type: f32,    // 0.0 = медвежья, 1.0 = бычья, 0.5 = фитиль, 2-6 = индикаторы, 7.0 = current price
}

// Выходные данные вершинного шейдера
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) element_type: f32,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Применяем матрицу преобразования к позиции
    let position = vec4<f32>(vertex.position_x, vertex.position_y, 0.0, 1.0);
    out.clip_position = uniforms.view_proj_matrix * position;
    
    // Определяем цвет на основе типа элемента и цвета
    if (vertex.element_type < 0.5) {
        // Тело свечи
        if (vertex.color_type > 0.5) {
            out.color = uniforms.bullish_color; // Бычья свеча - зеленая
        } else {
            out.color = uniforms.bearish_color; // Медвежья свеча - красная
        }
    } else if (vertex.element_type < 1.5) {
        // Фитиль
        out.color = uniforms.wick_color; // Серый цвет для фитилей
    } else if (vertex.element_type < 2.5) {
        // Линии индикаторов
        if (vertex.color_type < 2.5) {
            out.color = uniforms.sma20_color; // SMA 20 - красный
        } else if (vertex.color_type < 3.5) {
            out.color = uniforms.sma50_color; // SMA 50 - желтый
        } else if (vertex.color_type < 4.5) {
            out.color = uniforms.sma200_color; // SMA 200 - синий
        } else if (vertex.color_type < 5.5) {
            out.color = uniforms.ema12_color; // EMA 12 - фиолетовый
        } else {
            out.color = uniforms.ema26_color; // EMA 26 - голубой
        }
    } else if (vertex.element_type < 3.5) {
        // Сетка графика
        out.color = vec4<f32>(0.3, 0.3, 0.3, 0.3); // Очень светло-серый, полупрозрачный
    } else if (vertex.element_type < 4.5) {
        // 💰 Линия текущей цены
        out.color = uniforms.current_price_color; // Ярко-желтый
    } else if (vertex.element_type < 5.5) {
        // 📊 Volume bars
        if (vertex.color_type > 0.5) {
            // Бычий volume - зеленый с пониженной яркостью
            out.color = vec4<f32>(uniforms.bullish_color.rgb * 0.6, 0.8);
        } else {
            // Медвежий volume - красный с пониженной яркостью
            out.color = vec4<f32>(uniforms.bearish_color.rgb * 0.6, 0.8);
        }
    } else if (vertex.element_type > 98.0) {
        // УЛЬТРА-ПРОСТОЙ ТЕСТ - яркий красный цвет
        out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0); // Красный
    } else {
        // Неизвестный элемент - белый
        out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }
    
    out.element_type = vertex.element_type;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Упрощенный fragment shader - просто возвращаем цвет от vertex shader
    return vec4<f32>(in.color.rgb, 1.0); // Используем цвет от vertex shader, но принудительно альфа = 1.0
} 