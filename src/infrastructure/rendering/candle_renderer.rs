use wgpu::{Device, Queue, RenderPass, Buffer, BindGroup};
use crate::domain::chart::Chart;
use super::gpu_structures::{CandleVertex, ChartUniforms, CandleGeometry};

/// Рендерер свечей - управляет GPU буферами и отрисовкой
pub struct CandleRenderer {
    /// Вершинный буфер для свечей
    vertex_buffer: Buffer,
    /// Индексный буфер (если нужен)
    index_buffer: Option<Buffer>,
    /// Uniform буфер для параметров рендеринга
    uniform_buffer: Buffer,
    /// Bind group для uniform буфера
    uniform_bind_group: BindGroup,
    /// Текущие uniform данные
    uniforms: ChartUniforms,
    /// Максимальное количество вершин в буфере
    max_vertices: usize,
    /// Текущее количество вершин для отрисовки
    vertex_count: u32,
    /// Bind group layout
    bind_group_layout: wgpu::BindGroupLayout,
}

impl CandleRenderer {
    /// Создать новый рендерер свечей
    pub fn new(device: &Device, queue: &Queue, _surface_format: wgpu::TextureFormat) -> Self {
        // Создаем uniform буфер
        let uniforms = ChartUniforms::new();
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chart Uniform Buffer"),
            size: std::mem::size_of::<ChartUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Записываем начальные данные в uniform буфер
        queue.write_buffer(
            &uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        // Создаем bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Chart Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Создаем bind group
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Chart Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Создаем большой вершинный буфер для множества свечей
        let max_vertices = 10000; // Достаточно для ~500 свечей (20 вершин на свечу)
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Candle Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<CandleVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer: None,
            uniform_buffer,
            uniform_bind_group,
            uniforms,
            max_vertices,
            vertex_count: 0,
            bind_group_layout,
        }
    }

    /// Обновить данные свечей из ChartState
    pub fn update_from_chart(&mut self, chart: &Chart, _device: &Device, queue: &Queue) {
        // Обновляем uniform буфер с параметрами viewport
        self.update_uniforms_from_chart(chart, queue);
        
        // Генерируем vertices для всех свечей
        let vertices = self.generate_vertices_from_candles(chart);
        
        // Обновляем вершинный буфер
        if vertices.len() <= self.max_vertices {
            queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&vertices),
            );
            self.vertex_count = vertices.len() as u32;
            
            #[allow(unused_unsafe)]
            unsafe {
                web_sys::console::log_1(&format!(
                    "🎨 CandleRenderer: Updated {} vertices for {} candles",
                    vertices.len(),
                    chart.data.count()
                ).into());
            }
        } else {
            #[allow(unused_unsafe)]
            unsafe {
                web_sys::console::warn_1(&format!(
                    "⚠️ CandleRenderer: Too many vertices ({}) for buffer size ({})",
                    vertices.len(),
                    self.max_vertices
                ).into());
            }
        }
    }

    /// Обновить uniform буфер из данных графика
    fn update_uniforms_from_chart(&mut self, chart: &Chart, queue: &Queue) {
        let viewport = &chart.viewport;
        
        // Обновляем viewport данные
        self.uniforms.viewport = [
            viewport.width as f32,
            viewport.height as f32,
            viewport.min_price,
            viewport.max_price,
        ];
        
        self.uniforms.time_range = [
            viewport.start_time as f32,
            viewport.end_time as f32,
            viewport.time_range() as f32,
            0.0, // padding
        ];
        
        // Обновляем цвета из стиля графика
        let _style = &chart.style;
        self.uniforms.bullish_color = [0.0, 0.8, 0.0, 1.0];  // Зеленый
        self.uniforms.bearish_color = [0.8, 0.0, 0.0, 1.0];  // Красный
        self.uniforms.wick_color = [0.6, 0.6, 0.6, 1.0];     // Серый
        
        // Записываем обновления в GPU
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.uniforms]),
        );
    }

    /// Сгенерировать vertices для всех свечей в графике
    fn generate_vertices_from_candles(&self, chart: &Chart) -> Vec<CandleVertex> {
        let mut all_vertices = Vec::new();
        let candles = chart.data.get_candles();
        
        if candles.is_empty() {
            return all_vertices;
        }
        
        let viewport = &chart.viewport;
        let candle_count = candles.len();
        
        // Вычисляем ширину свечи на основе доступного пространства
        let available_width = 2.0; // NDC координаты от -1 до 1
        let spacing_factor = 0.8;  // 80% для свечей, 20% для промежутков
        let candle_width = (available_width * spacing_factor) / candle_count as f32;
        let candle_width = candle_width.min(0.05); // Максимальная ширина свечи
        
        for (i, candle) in candles.iter().enumerate() {
            // Нормализация X координаты (время)
            let x_normalized = if candle_count > 1 {
                (i as f32 / (candle_count - 1) as f32) * 2.0 - 1.0 // [-1, 1]
            } else {
                0.0
            };
            
            // Нормализация Y координат (цены)
            let price_range = viewport.max_price - viewport.min_price;
            if price_range > 0.0 {
                let open_y = ((candle.ohlcv.open.value() - viewport.min_price) / price_range) * 2.0 - 1.0;
                let high_y = ((candle.ohlcv.high.value() - viewport.min_price) / price_range) * 2.0 - 1.0;
                let low_y = ((candle.ohlcv.low.value() - viewport.min_price) / price_range) * 2.0 - 1.0;
                let close_y = ((candle.ohlcv.close.value() - viewport.min_price) / price_range) * 2.0 - 1.0;
                
                // Генерируем vertices для этой свечи
                let candle_vertices = CandleGeometry::create_candle_vertices(
                    candle.timestamp.as_f64(),
                    candle.ohlcv.open.value(),
                    candle.ohlcv.high.value(),
                    candle.ohlcv.low.value(),
                    candle.ohlcv.close.value(),
                    x_normalized,
                    open_y,
                    high_y,
                    low_y,
                    close_y,
                    candle_width,
                );
                
                all_vertices.extend(candle_vertices);
            }
        }
        
        // Добавляем сетку
        let grid_vertices = CandleGeometry::create_grid_vertices(
            viewport.width as f32,
            viewport.height as f32,
            10, // 10 вертикальных линий
            8,  // 8 горизонтальных линий
        );
        all_vertices.extend(grid_vertices);
        
        all_vertices
    }

    /// Отрисовать свечи
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        if self.vertex_count > 0 {
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.vertex_count, 0..1);
        }
    }

    /// Получить bind group layout для создания render pipeline
    pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Получить статистику рендеринга
    pub fn get_stats(&self) -> CandleRendererStats {
        CandleRendererStats {
            vertex_count: self.vertex_count,
            max_vertices: self.max_vertices as u32,
            buffer_usage_percent: (self.vertex_count as f32 / self.max_vertices as f32 * 100.0),
        }
    }
}

/// Статистика рендеринга свечей
#[derive(Debug, Clone)]
pub struct CandleRendererStats {
    pub vertex_count: u32,
    pub max_vertices: u32,
    pub buffer_usage_percent: f32,
} 