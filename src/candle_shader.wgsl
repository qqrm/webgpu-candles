// Uniform буфер с параметрами графика
struct ChartUniforms {
    view_proj_matrix: mat4x4<f32>,
    viewport: vec4<f32>,          // width, height, min_price, max_price
    time_range: vec4<f32>,        // start_time, end_time, time_range, _padding
    bullish_color: vec4<f32>,     // Цвет бычьих свечей (зеленый)
    bearish_color: vec4<f32>,     // Цвет медвежьих свечей (красный)
    wick_color: vec4<f32>,        // Цвет фитилей (серый)
    render_params: vec4<f32>,     // candle_width, spacing, line_width, _padding
}

@group(0) @binding(0)
var<uniform> uniforms: ChartUniforms;

// Вершинные атрибуты
struct VertexInput {
    @location(0) position_x: f32,    // X позиция в нормализованных координатах
    @location(1) position_y: f32,    // Y позиция в нормализованных координатах
    @location(2) element_type: f32,  // 0.0 = тело свечи, 1.0 = фитиль
    @location(3) color_type: f32,    // 0.0 = медвежья, 1.0 = бычья, 0.5 = фитиль
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
    
    // Простое преобразование позиции (уже в NDC координатах)
    out.clip_position = vec4<f32>(vertex.position_x, vertex.position_y, 0.0, 1.0);
    
    // Определяем цвет на основе типа элемента и цвета
    if (vertex.element_type < 0.5) {
        // Тело свечи
        if (vertex.color_type > 0.5) {
            out.color = uniforms.bullish_color; // Бычья свеча - зеленая
        } else {
            out.color = uniforms.bearish_color; // Медвежья свеча - красная
        }
    } else {
        // Фитиль
        out.color = uniforms.wick_color; // Серый цвет для фитилей
    }
    
    out.element_type = vertex.element_type;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Можно добавить дополнительные эффекты на основе element_type
    var final_color = in.color;
    
    // Легкое затемнение для фитилей
    if (in.element_type > 0.5) {
        final_color.a = 0.8; // Фитили чуть прозрачнее
    }
    
    return final_color;
} 