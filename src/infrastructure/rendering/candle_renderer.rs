use wgpu::{Device, Queue, RenderPass, Buffer, BindGroup};
use crate::domain::chart::Chart;
use super::gpu_structures::{CandleVertex, ChartUniforms, CandleGeometry};

/// Рендерер свечей - управляет GPU буферами и отрисовкой
pub struct CandleRenderer {
    /// Система double buffering - два вершинных буфера
    vertex_buffers: [Buffer; 2],
    /// Текущий активный буфер (0 или 1)
    current_buffer: usize,
    /// Uniform буфер для параметров рендеринга
    uniform_buffer: Buffer,
    /// Bind group для uniform буфера
    uniform_bind_group: BindGroup,
    /// Текущие uniform данные
    uniforms: ChartUniforms,
    /// Максимальное количество вершин в буфере
    max_vertices: usize,
    /// Текущее количество вершин для отрисовки в каждом буфере
    vertex_counts: [u32; 2],
    /// Bind group layout
    bind_group_layout: wgpu::BindGroupLayout,
    /// Кэшированное состояние viewport для отслеживания изменений
    cached_viewport: ViewportState,
    /// Буфер для переиспользования vertices (оптимизация аллокаций)
    vertex_cache: Vec<CandleVertex>,
    /// Статистика использования буферов
    buffer_stats: BufferStats,
    /// Флаг для переключения буферов при следующем рендере
    swap_buffers_next_frame: bool,
}

/// Кэшированное состояние viewport для оптимизации обновлений
#[derive(Debug, Clone, PartialEq)]
struct ViewportState {
    width: u32,
    height: u32,
    min_price: f32,
    max_price: f32,
    start_time: f64,
    end_time: f64,
    candle_count: usize,
}

/// Статистика использования буферов
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub vertex_count: u32,
    pub max_vertices: u32,
    pub buffer_usage_percent: f32,
    pub uniform_updates: u32,
    pub geometry_regenerations: u32,
    pub viewport_changes: u32,
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
        let vertex_buffers = [
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Candle Vertex Buffer 0"),
                size: (max_vertices * std::mem::size_of::<CandleVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Candle Vertex Buffer 1"),
                size: (max_vertices * std::mem::size_of::<CandleVertex>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        ];

        Self {
            vertex_buffers,
            current_buffer: 0,
            uniform_buffer,
            uniform_bind_group,
            uniforms,
            max_vertices,
            vertex_counts: [0; 2],
            bind_group_layout,
            cached_viewport: ViewportState {
                width: 0,
                height: 0,
                min_price: 0.0,
                max_price: 0.0,
                start_time: 0.0,
                end_time: 0.0,
                candle_count: 0,
            },
            vertex_cache: Vec::new(),
            buffer_stats: BufferStats {
                vertex_count: 0,
                max_vertices: 0,
                buffer_usage_percent: 0.0,
                uniform_updates: 0,
                geometry_regenerations: 0,
                viewport_changes: 0,
            },
            swap_buffers_next_frame: false,
        }
    }

    /// Обновить данные свечей из ChartState
    pub fn update_from_chart(&mut self, chart: &Chart, _device: &Device, queue: &Queue) {
        let current_viewport = self.extract_viewport_state(chart);
        let viewport_changed = current_viewport != self.cached_viewport;
        
        // Обновляем uniform буфер только при изменении viewport
        if viewport_changed {
            self.update_uniforms_from_chart(chart, queue);
            self.cached_viewport = current_viewport;
            self.buffer_stats.viewport_changes += 1;
            self.buffer_stats.uniform_updates += 1;
            
            #[allow(unused_unsafe)]
            unsafe {
                web_sys::console::log_1(&format!(
                    "🔄 Viewport changed: {}x{}, price: {:.2}-{:.2}, time: {:.0}-{:.0}",
                    self.cached_viewport.width,
                    self.cached_viewport.height,
                    self.cached_viewport.min_price,
                    self.cached_viewport.max_price,
                    self.cached_viewport.start_time,
                    self.cached_viewport.end_time
                ).into());
            }
        }
        
        // Генерируем vertices только если viewport изменился или есть новые данные
        if viewport_changed || chart.data.count() != self.cached_viewport.candle_count {
            self.regenerate_geometry(chart, queue);
            self.buffer_stats.geometry_regenerations += 1;
        }
        
        // Обновляем статистику
        self.update_buffer_stats();
    }
    
    /// Извлечь состояние viewport для сравнения
    fn extract_viewport_state(&self, chart: &Chart) -> ViewportState {
        let viewport = &chart.viewport;
        ViewportState {
            width: viewport.width,
            height: viewport.height,
            min_price: viewport.min_price,
            max_price: viewport.max_price,
            start_time: viewport.start_time,
            end_time: viewport.end_time,
            candle_count: chart.data.count(),
        }
    }
    
    /// Регенерировать геометрию с оптимизацией аллокаций
    fn regenerate_geometry(&mut self, chart: &Chart, queue: &Queue) {
        // Очищаем кэш вершин, но сохраняем capacity
        self.vertex_cache.clear();
        
        // Оценочное количество вершин: ~20 на свечу + сетка
        let estimated_vertices = chart.data.count() * 20 + 400; // 400 для сетки
        if self.vertex_cache.capacity() < estimated_vertices {
            self.vertex_cache.reserve(estimated_vertices - self.vertex_cache.capacity());
        }
        
        // Генерируем vertices с переиспользованием буфера
        self.generate_vertices_optimized(chart);
        
        // Проверяем вместимость буфера
        if self.vertex_cache.len() <= self.max_vertices {
            queue.write_buffer(
                &self.vertex_buffers[self.current_buffer],
                0,
                bytemuck::cast_slice(&self.vertex_cache),
            );
            self.vertex_counts[self.current_buffer] = self.vertex_cache.len() as u32;
            
            #[allow(unused_unsafe)]
            unsafe {
                web_sys::console::log_1(&format!(
                    "🎨 Geometry regenerated: {} vertices for {} candles ({:.1}% buffer usage)",
                    self.vertex_cache.len(),
                    chart.data.count(),
                    (self.vertex_cache.len() as f32 / self.max_vertices as f32 * 100.0)
                ).into());
            }
        } else {
            self.handle_buffer_overflow();
        }
    }
    
    /// Обработка переполнения буфера
    fn handle_buffer_overflow(&mut self) {
        #[allow(unused_unsafe)]
        unsafe {
            web_sys::console::error_1(&format!(
                "❌ Buffer overflow: {} vertices > {} max capacity. Rendering {} vertices only.",
                self.vertex_cache.len(),
                self.max_vertices,
                self.max_vertices
            ).into());
        }
        
        // Обрезаем до максимального размера буфера
        self.vertex_cache.truncate(self.max_vertices);
        self.vertex_counts[self.current_buffer] = self.max_vertices as u32;
    }
    
    /// Обновить статистику буферов
    fn update_buffer_stats(&mut self) {
        self.buffer_stats.vertex_count = self.vertex_counts[self.current_buffer];
        self.buffer_stats.max_vertices = self.max_vertices as u32;
        self.buffer_stats.buffer_usage_percent = 
            (self.vertex_counts[self.current_buffer] as f32 / self.max_vertices as f32 * 100.0);
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

    /// Оптимизированная генерация vertices с переиспользованием буфера
    fn generate_vertices_optimized(&mut self, chart: &Chart) {
        let candles = chart.data.get_candles();
        
        if candles.is_empty() {
            return;
        }
        
        let viewport = &chart.viewport;
        let candle_count = candles.len();
        
        // Вычисляем ширину свечи на основе доступного пространства
        let available_width = 2.0; // NDC координаты от -1 до 1
        let spacing_factor = 0.8;  // 80% для свечей, 20% для промежутков
        let candle_width = (available_width * spacing_factor) / candle_count as f32;
        let candle_width = candle_width.min(0.05); // Максимальная ширина свечи
        
        // Генерируем vertices для свечей
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
                
                // Генерируем vertices для этой свечи напрямую в кэш
                self.add_candle_vertices_to_cache(
                    candle.ohlcv.open.value(),
                    candle.ohlcv.close.value(),
                    x_normalized,
                    open_y,
                    high_y,
                    low_y,
                    close_y,
                    candle_width,
                );
            }
        }
        
        // Добавляем сетку в конце
        self.add_grid_vertices_to_cache(10, 8); // 10 вертикальных, 8 горизонтальных линий
    }
    
    /// Добавить vertices одной свечи напрямую в кэш (избегаем промежуточных аллокаций)
    fn add_candle_vertices_to_cache(
        &mut self,
        open: f32,
        close: f32,
        x_normalized: f32,
        open_y: f32,
        high_y: f32,
        low_y: f32,
        close_y: f32,
        width: f32,
    ) {
        let is_bullish = close > open;
        let half_width = width * 0.5;
        
        // Определяем координаты тела свечи
        let body_top = if is_bullish { close_y } else { open_y };
        let body_bottom = if is_bullish { open_y } else { close_y };
        
        // Добавляем прямоугольник для тела свечи (2 треугольника = 6 вершин)
        self.vertex_cache.extend_from_slice(&[
            // Первый треугольник
            CandleVertex::body_vertex(x_normalized - half_width, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized + half_width, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized - half_width, body_top, is_bullish),
            
            // Второй треугольник
            CandleVertex::body_vertex(x_normalized + half_width, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized + half_width, body_top, is_bullish),
            CandleVertex::body_vertex(x_normalized - half_width, body_top, is_bullish),
        ]);
        
        // Добавляем фитили
        let wick_width = width * 0.1;
        let wick_half = wick_width * 0.5;
        
        // Верхний фитиль (если есть)
        if high_y > body_top {
            self.vertex_cache.extend_from_slice(&[
                CandleVertex::wick_vertex(x_normalized - wick_half, body_top),
                CandleVertex::wick_vertex(x_normalized + wick_half, body_top),
                CandleVertex::wick_vertex(x_normalized - wick_half, high_y),
                
                CandleVertex::wick_vertex(x_normalized + wick_half, body_top),
                CandleVertex::wick_vertex(x_normalized + wick_half, high_y),
                CandleVertex::wick_vertex(x_normalized - wick_half, high_y),
            ]);
        }
        
        // Нижний фитиль (если есть)
        if low_y < body_bottom {
            self.vertex_cache.extend_from_slice(&[
                CandleVertex::wick_vertex(x_normalized - wick_half, low_y),
                CandleVertex::wick_vertex(x_normalized + wick_half, low_y),
                CandleVertex::wick_vertex(x_normalized - wick_half, body_bottom),
                
                CandleVertex::wick_vertex(x_normalized + wick_half, low_y),
                CandleVertex::wick_vertex(x_normalized + wick_half, body_bottom),
                CandleVertex::wick_vertex(x_normalized - wick_half, body_bottom),
            ]);
        }
    }
    
    /// Добавить vertices сетки напрямую в кэш
    fn add_grid_vertices_to_cache(&mut self, grid_lines_x: u32, grid_lines_y: u32) {
        let line_width = 0.002;
        
        // Вертикальные линии
        for i in 0..=grid_lines_x {
            let x = i as f32 / grid_lines_x as f32 * 2.0 - 1.0;
            let half_width = line_width * 0.5;
            
            self.vertex_cache.extend_from_slice(&[
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
            let y = i as f32 / grid_lines_y as f32 * 2.0 - 1.0;
            let half_width = line_width * 0.5;
            
            self.vertex_cache.extend_from_slice(&[
                CandleVertex::wick_vertex(-1.0, y - half_width),
                CandleVertex::wick_vertex(1.0, y - half_width),
                CandleVertex::wick_vertex(-1.0, y + half_width),
                
                CandleVertex::wick_vertex(1.0, y - half_width),
                CandleVertex::wick_vertex(1.0, y + half_width),
                CandleVertex::wick_vertex(-1.0, y + half_width),
            ]);
        }
    }

    /// Отрисовать свечи
    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
        if self.vertex_counts[self.current_buffer] > 0 {
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffers[self.current_buffer].slice(..));
            render_pass.draw(0..self.vertex_counts[self.current_buffer], 0..1);
        }
    }

    /// Получить bind group layout для создания render pipeline
    pub fn get_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    /// Получить статистику рендеринга
    pub fn get_stats(&self) -> &BufferStats {
        &self.buffer_stats
    }
    
    /// Получить детальную статистику использования
    pub fn get_detailed_stats(&self) -> DetailedStats {
        DetailedStats {
            buffer_stats: self.buffer_stats.clone(),
            vertex_cache_capacity: self.vertex_cache.capacity(),
            vertex_cache_len: self.vertex_cache.len(),
            is_near_capacity: self.buffer_stats.buffer_usage_percent > 80.0,
            candles_capacity: self.max_vertices / 20, // ~20 vertices per candle
            current_candles: self.cached_viewport.candle_count,
        }
    }
    
    /// Сбросить статистику
    pub fn reset_stats(&mut self) {
        self.buffer_stats.uniform_updates = 0;
        self.buffer_stats.geometry_regenerations = 0;
        self.buffer_stats.viewport_changes = 0;
    }

    /// Переключить буферы для плавного рендеринга (double buffering)
    pub fn swap_buffers(&mut self) {
        if self.swap_buffers_next_frame {
            let old_buffer = self.current_buffer;
            self.current_buffer = 1 - self.current_buffer; // Переключаем между 0 и 1
            self.swap_buffers_next_frame = false;
            
            #[allow(unused_unsafe)]
            unsafe {
                web_sys::console::log_1(&format!(
                    "🔄 Buffer swapped: {} -> {} ({} vertices)",
                    old_buffer,
                    self.current_buffer,
                    self.vertex_counts[self.current_buffer]
                ).into());
            }
        }
    }
    
    /// Подготовить следующий буфер в фоне (для double buffering)
    pub fn prepare_next_buffer(&mut self, chart: &Chart, queue: &Queue) {
        let next_buffer = 1 - self.current_buffer; // Следующий буфер
        
        // Генерируем геометрию в vertex_cache
        self.vertex_cache.clear();
        let estimated_vertices = chart.data.count() * 20 + 400;
        if self.vertex_cache.capacity() < estimated_vertices {
            self.vertex_cache.reserve(estimated_vertices - self.vertex_cache.capacity());
        }
        
        self.generate_vertices_optimized(chart);
        
        // Записываем в следующий буфер
        if self.vertex_cache.len() <= self.max_vertices {
            queue.write_buffer(
                &self.vertex_buffers[next_buffer],
                0,
                bytemuck::cast_slice(&self.vertex_cache),
            );
            self.vertex_counts[next_buffer] = self.vertex_cache.len() as u32;
            self.swap_buffers_next_frame = true; // Готов к переключению
            
            #[allow(unused_unsafe)]
            unsafe {
                web_sys::console::log_1(&format!(
                    "📦 Next buffer prepared: buffer {} with {} vertices (ready for swap)",
                    next_buffer,
                    self.vertex_counts[next_buffer]
                ).into());
            }
        } else {
            // Если не помещается, используем текущий буфер
            self.vertex_cache.truncate(self.max_vertices);
            self.vertex_counts[next_buffer] = self.max_vertices as u32;
            
            #[allow(unused_unsafe)]
            unsafe {
                web_sys::console::warn_1(&format!(
                    "⚠️ Next buffer overflow, truncated to {} vertices",
                    self.max_vertices
                ).into());
            }
        }
    }
    
    /// Оптимизированное обновление с double buffering
    pub fn update_with_double_buffering(&mut self, chart: &Chart, _device: &Device, queue: &Queue) {
        let current_viewport = self.extract_viewport_state(chart);
        let viewport_changed = current_viewport != self.cached_viewport;
        
        // Переключаем буферы если готов
        self.swap_buffers();
        
        // Обновляем uniform буфер только при изменении viewport
        if viewport_changed {
            self.update_uniforms_from_chart(chart, queue);
            self.cached_viewport = current_viewport;
            self.buffer_stats.viewport_changes += 1;
            self.buffer_stats.uniform_updates += 1;
        }
        
        // Подготавливаем следующий буфер если viewport изменился или есть новые данные
        if viewport_changed || chart.data.count() != self.cached_viewport.candle_count {
            self.prepare_next_buffer(chart, queue);
            self.buffer_stats.geometry_regenerations += 1;
        }
        
        // Обновляем статистику
        self.update_buffer_stats();
    }
    
    /// Получить информацию о состоянии буферов
    pub fn get_buffer_info(&self) -> BufferInfo {
        BufferInfo {
            current_buffer: self.current_buffer,
            vertex_counts: self.vertex_counts,
            swap_ready: self.swap_buffers_next_frame,
            total_capacity: self.max_vertices,
        }
    }
}

/// Детальная статистика системы рендеринга
#[derive(Debug, Clone)]
pub struct DetailedStats {
    pub buffer_stats: BufferStats,
    pub vertex_cache_capacity: usize,
    pub vertex_cache_len: usize,
    pub is_near_capacity: bool,
    pub candles_capacity: usize,
    pub current_candles: usize,
}

/// Информация о состоянии буферов
#[derive(Debug, Clone)]
pub struct BufferInfo {
    pub current_buffer: usize,
    pub vertex_counts: [u32; 2],
    pub swap_ready: bool,
    pub total_capacity: usize,
} 