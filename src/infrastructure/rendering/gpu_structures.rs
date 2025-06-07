use bytemuck::{Pod, Zeroable};

/// GPU представление свечи для вершинного буфера
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CandleVertex {
    /// Позиция X (время в нормализованных координатах)
    pub position_x: f32,
    /// Позиция Y (цена в нормализованных координатах)  
    pub position_y: f32,
    /// Тип элемента свечи: 0 = тело, 1 = фитиль
    pub element_type: f32,
    /// Цвет: 0 = медвежья (красная), 1 = бычья (зеленая)
    pub color_type: f32,
}

impl CandleVertex {
    /// Создать vertex для тела свечи
    pub fn body_vertex(x: f32, y: f32, is_bullish: bool) -> Self {
        Self {
            position_x: x,
            position_y: y,
            element_type: 0.0, // тело
            color_type: if is_bullish { 1.0 } else { 0.0 },
        }
    }
    
    /// Создать vertex для фитиля свечи
    pub fn wick_vertex(x: f32, y: f32) -> Self {
        Self {
            position_x: x,
            position_y: y,
            element_type: 1.0, // фитиль
            color_type: 0.5,   // нейтральный цвет для фитиля
        }
    }
    
    /// Дескриптор вершинного буфера для wgpu
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CandleVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position_x
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32,
                },
                // position_y
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<f32>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32,
                },
                // element_type
                wgpu::VertexAttribute {
                    offset: (2 * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
                // color_type
                wgpu::VertexAttribute {
                    offset: (3 * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

/// Uniform буфер для глобальных параметров рендеринга
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ChartUniforms {
    /// Матрица преобразования viewport
    pub view_proj_matrix: [[f32; 4]; 4],
    /// Размеры viewport (width, height, min_price, max_price)
    pub viewport: [f32; 4],
    /// Временной диапазон (start_time, end_time, time_range, _padding)
    pub time_range: [f32; 4],
    /// Цвета (bullish_r, bullish_g, bullish_b, bullish_a)
    pub bullish_color: [f32; 4],
    /// Цвета (bearish_r, bearish_g, bearish_b, bearish_a)
    pub bearish_color: [f32; 4],
    /// Цвет фитиля (wick_r, wick_g, wick_b, wick_a)
    pub wick_color: [f32; 4],
    /// Параметры рендеринга (candle_width, spacing, line_width, _padding)
    pub render_params: [f32; 4],
}

impl ChartUniforms {
    pub fn new() -> Self {
        Self {
            view_proj_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            viewport: [800.0, 600.0, 0.0, 100.0],
            time_range: [0.0, 0.0, 0.0, 0.0],
            bullish_color: [0.0, 0.8, 0.0, 1.0],   // Зеленый
            bearish_color: [0.8, 0.0, 0.0, 1.0],   // Красный
            wick_color: [0.6, 0.6, 0.6, 1.0],      // Серый
            render_params: [8.0, 2.0, 1.0, 0.0],   // width, spacing, line_width, padding
        }
    }
}

/// Генератор геометрии для свечей
pub struct CandleGeometry;

impl CandleGeometry {
    /// Создать vertices для одной свечи
    pub fn create_candle_vertices(
        _timestamp: f64,
        open: f32,
        _high: f32,
        _low: f32,
        close: f32,
        x_normalized: f32,
        open_y: f32,
        high_y: f32,
        low_y: f32,
        close_y: f32,
        width: f32,
    ) -> Vec<CandleVertex> {
        let mut vertices = Vec::new();
        let is_bullish = close > open;
        let half_width = width * 0.5;
        
        // Определяем координаты тела свечи
        let body_top = if is_bullish { close_y } else { open_y };
        let body_bottom = if is_bullish { open_y } else { close_y };
        
        // Создаем прямоугольник для тела свечи (2 треугольника = 6 вершин)
        let body_vertices = [
            // Первый треугольник
            CandleVertex::body_vertex(x_normalized - half_width, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized + half_width, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized - half_width, body_top, is_bullish),
            
            // Второй треугольник
            CandleVertex::body_vertex(x_normalized + half_width, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized + half_width, body_top, is_bullish),
            CandleVertex::body_vertex(x_normalized - half_width, body_top, is_bullish),
        ];
        
        vertices.extend_from_slice(&body_vertices);
        
        // Создаем линии для фитилей (верхний и нижний)
        let wick_width = width * 0.1; // Фитиль тоньше тела
        let wick_half = wick_width * 0.5;
        
        // Верхний фитиль (если есть)
        if high_y > body_top {
            let upper_wick = [
                CandleVertex::wick_vertex(x_normalized - wick_half, body_top),
                CandleVertex::wick_vertex(x_normalized + wick_half, body_top),
                CandleVertex::wick_vertex(x_normalized - wick_half, high_y),
                
                CandleVertex::wick_vertex(x_normalized + wick_half, body_top),
                CandleVertex::wick_vertex(x_normalized + wick_half, high_y),
                CandleVertex::wick_vertex(x_normalized - wick_half, high_y),
            ];
            vertices.extend_from_slice(&upper_wick);
        }
        
        // Нижний фитиль (если есть)
        if low_y < body_bottom {
            let lower_wick = [
                CandleVertex::wick_vertex(x_normalized - wick_half, low_y),
                CandleVertex::wick_vertex(x_normalized + wick_half, low_y),
                CandleVertex::wick_vertex(x_normalized - wick_half, body_bottom),
                
                CandleVertex::wick_vertex(x_normalized + wick_half, low_y),
                CandleVertex::wick_vertex(x_normalized + wick_half, body_bottom),
                CandleVertex::wick_vertex(x_normalized - wick_half, body_bottom),
            ];
            vertices.extend_from_slice(&lower_wick);
        }
        
        vertices
    }
    
    /// Создать vertices для сетки графика
    pub fn create_grid_vertices(
        _viewport_width: f32,
        _viewport_height: f32,
        grid_lines_x: u32,
        grid_lines_y: u32,
    ) -> Vec<CandleVertex> {
        let mut vertices = Vec::new();
        let line_width = 0.002; // Тонкие линии сетки
        
        // Вертикальные линии
        for i in 0..=grid_lines_x {
            let x = i as f32 / grid_lines_x as f32 * 2.0 - 1.0; // Нормализация в [-1, 1]
            let half_width = line_width * 0.5;
            
            // Вертикальная линия как тонкий прямоугольник
            vertices.extend_from_slice(&[
                CandleVertex::wick_vertex(x - half_width, -1.0),
                CandleVertex::wick_vertex(x + half_width, -1.0),
                CandleVertex::wick_vertex(x - half_width, 1.0),
                
                CandleVertex::wick_vertex(x + half_width, -1.0),
                CandleVertex::wick_vertex(x + half_width, 1.0),
                CandleVertex::wick_vertex(x - half_width, 1.0),
            ]);
        }
        
        // Горизонтальные линии
        for i in 0..=grid_lines_y {
            let y = i as f32 / grid_lines_y as f32 * 2.0 - 1.0; // Нормализация в [-1, 1]
            let half_width = line_width * 0.5;
            
            // Горизонтальная линия как тонкий прямоугольник
            vertices.extend_from_slice(&[
                CandleVertex::wick_vertex(-1.0, y - half_width),
                CandleVertex::wick_vertex(1.0, y - half_width),
                CandleVertex::wick_vertex(-1.0, y + half_width),
                
                CandleVertex::wick_vertex(1.0, y - half_width),
                CandleVertex::wick_vertex(1.0, y + half_width),
                CandleVertex::wick_vertex(-1.0, y + half_width),
            ]);
        }
        
        vertices
    }
} 