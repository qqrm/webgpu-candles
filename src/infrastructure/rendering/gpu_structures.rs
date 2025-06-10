use bytemuck::{Pod, Zeroable};

/// Типы индикаторов для GPU рендеринга
#[derive(Debug, Clone, Copy)]
pub enum IndicatorType {
    SMA20,
    SMA50,
    SMA200,
    EMA12,
    EMA26,
}

/// GPU представление свечи для вершинного буфера
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CandleVertex {
    /// Позиция X (время в нормализованных координатах)
    pub position_x: f32,
    /// Позиция Y (цена в нормализованных координатах)  
    pub position_y: f32,
    /// Тип элемента: 0 = тело свечи, 1 = фитиль, 2 = линия индикатора, 3 = сетка, 4 = current price line
    pub element_type: f32,
    /// Цвет/индикатор: для свечей 0/1, для индикаторов: 2=SMA20, 3=SMA50, 4=SMA200, 5=EMA12, 6=EMA26, 7 = current price
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

    /// Создать vertex для линии индикатора
    pub fn indicator_vertex(x: f32, y: f32, indicator_type: IndicatorType) -> Self {
        let color_type = match indicator_type {
            IndicatorType::SMA20 => 2.0,
            IndicatorType::SMA50 => 3.0,
            IndicatorType::SMA200 => 4.0,
            IndicatorType::EMA12 => 5.0,
            IndicatorType::EMA26 => 6.0,
        };

        Self {
            position_x: x,
            position_y: y,
            element_type: 2.0, // линия индикатора
            color_type,
        }
    }

    /// Создать vertex для сетки графика
    pub fn grid_vertex(x: f32, y: f32) -> Self {
        Self {
            position_x: x,
            position_y: y,
            element_type: 3.0, // сетка
            color_type: 0.2,   // очень светлый серый цвет
        }
    }

    /// 💰 Создать vertex для линии текущей цены
    pub fn current_price_vertex(x: f32, y: f32) -> Self {
        Self {
            position_x: x,
            position_y: y,
            element_type: 4.0, // current price line
            color_type: 7.0,   // специальный цвет для current price
        }
    }

    /// 📊 Создать vertex для volume bars
    pub fn volume_vertex(x: f32, y: f32, is_bullish: bool) -> Self {
        Self {
            position_x: x,
            position_y: y,
            element_type: 5.0,                              // volume bar
            color_type: if is_bullish { 1.0 } else { 0.0 }, // тот же цвет что и у свечей
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
    /// Цвет SMA 20 (sma20_r, sma20_g, sma20_b, sma20_a)
    pub sma20_color: [f32; 4],
    /// Цвет SMA 50 (sma50_r, sma50_g, sma50_b, sma50_a)
    pub sma50_color: [f32; 4],
    /// Цвет SMA 200 (sma200_r, sma200_g, sma200_b, sma200_a)
    pub sma200_color: [f32; 4],
    /// Цвет EMA 12 (ema12_r, ema12_g, ema12_b, ema12_a)
    pub ema12_color: [f32; 4],
    /// Цвет EMA 26 (ema26_r, ema26_g, ema26_b, ema26_a)
    pub ema26_color: [f32; 4],
    /// 💰 Цвет текущей цены (current_price_r, current_price_g, current_price_b, current_price_a)
    pub current_price_color: [f32; 4],
    /// Параметры рендеринга (candle_width, spacing, line_width, _padding)
    pub render_params: [f32; 4],
}

impl Default for ChartUniforms {
    fn default() -> Self {
        Self::new()
    }
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
            bullish_color: [0.455, 0.780, 0.529, 1.0], // #74c787 - buy
            bearish_color: [0.882, 0.424, 0.282, 1.0], // #e16c48 - sell
            wick_color: [0.6, 0.6, 0.6, 1.0],          // Серый
            sma20_color: [1.0, 0.0, 0.0, 1.0],         // Ярко-красный
            sma50_color: [1.0, 0.8, 0.0, 1.0],         // Желтый
            sma200_color: [0.2, 0.4, 0.8, 1.0],        // Синий
            ema12_color: [0.8, 0.2, 0.8, 1.0],         // Фиолетовый
            ema26_color: [0.0, 0.8, 0.8, 1.0],         // Голубой
            current_price_color: [1.0, 1.0, 0.0, 0.8], // 💰 Ярко-желтый с прозрачностью
            render_params: [8.0, 2.0, 1.0, 0.0],       // width, spacing, line_width, padding
        }
    }
}

/// Генератор геометрии для свечей
pub struct CandleGeometry;

impl CandleGeometry {
    /// Создать vertices для одной свечи
    #[allow(clippy::too_many_arguments)]
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

    /// 💰 Создать vertices для линии текущей цены
    pub fn create_current_price_line(current_price_y: f32, line_width: f32) -> Vec<CandleVertex> {
        let half_width = line_width * 0.5;

        // Горизонтальная линия через весь экран
        vec![
            CandleVertex::current_price_vertex(-1.0, current_price_y - half_width),
            CandleVertex::current_price_vertex(1.0, current_price_y - half_width),
            CandleVertex::current_price_vertex(-1.0, current_price_y + half_width),
            CandleVertex::current_price_vertex(1.0, current_price_y - half_width),
            CandleVertex::current_price_vertex(1.0, current_price_y + half_width),
            CandleVertex::current_price_vertex(-1.0, current_price_y + half_width),
        ]
    }

    /// Создать vertices для линии индикатора - улучшенный алгоритм для сплошных линий
    pub fn create_indicator_line_vertices(
        points: &[(f32, f32)], // (x_normalized, y_normalized) точки
        indicator_type: IndicatorType,
        line_width: f32,
    ) -> Vec<CandleVertex> {
        if points.len() < 2 {
            return Vec::new();
        }

        let mut vertices = Vec::new();
        let half_width = (line_width * 0.3).max(0.001); // Тоньше линии для лучшего вида

        // Создаем непрерывную линию как triangle strip
        for i in 0..(points.len() - 1) {
            let (x1, y1) = points[i];
            let (x2, y2) = points[i + 1];

            // Вычисляем перпендикулярный вектор для правильной толщины линии
            let dx = x2 - x1;
            let dy = y2 - y1;
            let length = (dx * dx + dy * dy).sqrt();

            // Нормализованный перпендикулярный вектор
            let (perp_x, perp_y) = if length > 0.0001 {
                (-dy / length * half_width, dx / length * half_width)
            } else {
                (0.0, half_width) // Вертикальная линия
            };

            // Создаем прямоугольник как два треугольника без зазоров
            let segment_vertices = [
                // Первый треугольник
                CandleVertex::indicator_vertex(x1 - perp_x, y1 - perp_y, indicator_type),
                CandleVertex::indicator_vertex(x1 + perp_x, y1 + perp_y, indicator_type),
                CandleVertex::indicator_vertex(x2 - perp_x, y2 - perp_y, indicator_type),
                // Второй треугольник
                CandleVertex::indicator_vertex(x1 + perp_x, y1 + perp_y, indicator_type),
                CandleVertex::indicator_vertex(x2 + perp_x, y2 + perp_y, indicator_type),
                CandleVertex::indicator_vertex(x2 - perp_x, y2 - perp_y, indicator_type),
            ];

            vertices.extend_from_slice(&segment_vertices);
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

    /// Создать умную ценовую сетку с красивыми уровнями
    pub fn create_price_grid(
        min_price: f32,
        max_price: f32,
        chart_width: f32,
        chart_height: f32,
        time_lines: u32,
        price_lines: u32,
    ) -> Vec<CandleVertex> {
        let mut vertices = Vec::new();
        let grid_line_width = 0.001; // Очень тонкие линии сетки
        let half_width = grid_line_width * 0.5;

        // Вертикальные линии (временная сетка)
        for i in 1..time_lines {
            // Пропускаем крайние линии
            let x = (i as f32 / time_lines as f32) * chart_width - 1.0;

            // Вертикальная линия
            vertices.extend_from_slice(&[
                CandleVertex::grid_vertex(x - half_width, -1.0),
                CandleVertex::grid_vertex(x + half_width, -1.0),
                CandleVertex::grid_vertex(x - half_width, 1.0),
                CandleVertex::grid_vertex(x + half_width, -1.0),
                CandleVertex::grid_vertex(x + half_width, 1.0),
                CandleVertex::grid_vertex(x - half_width, 1.0),
            ]);
        }

        // Горизонтальные линии (ценовая сетка)
        let price_range = max_price - min_price;
        let nice_step = Self::calculate_nice_price_step(price_range, price_lines);

        // Находим первый красивый уровень цены
        let first_price = ((min_price / nice_step).ceil() * nice_step).max(min_price);

        let mut current_price = first_price;
        while current_price <= max_price {
            // Преобразуем цену в координату Y
            let y = -1.0 + ((current_price - min_price) / price_range) * chart_height;

            // Горизонтальная линия
            vertices.extend_from_slice(&[
                CandleVertex::grid_vertex(-1.0, y - half_width),
                CandleVertex::grid_vertex(1.0, y - half_width),
                CandleVertex::grid_vertex(-1.0, y + half_width),
                CandleVertex::grid_vertex(1.0, y - half_width),
                CandleVertex::grid_vertex(1.0, y + half_width),
                CandleVertex::grid_vertex(-1.0, y + half_width),
            ]);

            current_price += nice_step;
        }

        vertices
    }

    /// Вычисляет красивый шаг для ценовой сетки
    fn calculate_nice_price_step(price_range: f32, target_lines: u32) -> f32 {
        let raw_step = price_range / target_lines as f32;

        // Находим порядок величины
        let magnitude = 10.0_f32.powf(raw_step.log10().floor());

        // Нормализуем к диапазону [1, 10)
        let normalized = raw_step / magnitude;

        // Выбираем красивое значение
        let nice_normalized = if normalized <= 1.0 {
            1.0
        } else if normalized <= 2.0 {
            2.0
        } else if normalized <= 5.0 {
            5.0
        } else {
            10.0
        };

        nice_normalized * magnitude
    }
}
