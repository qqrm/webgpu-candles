use wasm_bindgen::prelude::*;
use crate::domain::{
    chart::Chart,
    logging::{LogComponent, get_logger},

};
use wgpu::util::DeviceExt;
use crate::infrastructure::rendering::gpu_structures::{CandleVertex, ChartUniforms};
use gloo::utils::document;
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;
use js_sys;

/// Настоящий WebGPU рендерер для свечей
pub struct WebGpuRenderer {
    _canvas_id: String,
    width: u32,
    height: u32,
    
    // WGPU state
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    
    // Rendering pipeline
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    num_vertices: u32,
}

/// Состояние видимости линий индикаторов
#[derive(Debug, Clone)]
pub struct LineVisibility {
    pub sma_20: bool,
    pub sma_50: bool,
    pub sma_200: bool,
    pub ema_12: bool,
    pub ema_26: bool,
}

impl Default for LineVisibility {
    fn default() -> Self {
        Self {
            sma_20: true,
            sma_50: true,
            sma_200: true,
            ema_12: true,
            ema_26: true,
        }
    }
}

impl WebGpuRenderer {
    pub async fn is_webgpu_supported() -> bool {
        if let Some(window) = web_sys::window() {
            unsafe {
                let navigator = window.navigator();
                js_sys::Reflect::has(&navigator, &"gpu".into()).unwrap_or(false)
            }
        } else {
            false
        }
    }

    pub async fn new(canvas_id: &str, width: u32, height: u32) -> Result<Self, JsValue> {
        let canvas = document()
            .get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str(&format!("Canvas with id '{}' not found", canvas_id)))?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("Element is not a canvas"))?;
        
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🎯 Canvas found: {}x{} -> setting to {}x{}", 
                canvas.width(), canvas.height(), width, height)
        );
        
        canvas.set_width(width);
        canvas.set_height(height);
        
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🎯 Canvas configured: {}x{}", canvas.width(), canvas.height())
        );

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {}", e)))?;
            
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🎯 WebGPU surface created successfully"
        );

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to find adapter: {:?}", e)))?;

        // Get the adapter's supported limits to ensure compatibility
        let supported_limits = adapter.limits();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: wgpu::Features::empty(),
                    // Use the adapter's own supported limits
                    required_limits: supported_limits,
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                },
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create device: {:?}", e)))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🎯 Surface config: {}x{}, format: {:?}, present_mode: {:?}, alpha: {:?}", 
                config.width, config.height, config.format, config.present_mode, config.alpha_mode)
        );
        
        surface.configure(&device, &config);
        
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🎯 Surface configured successfully"
        );

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[ChartUniforms::new()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("uniform_bind_group_layout"),
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
            label: Some("uniform_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../candle_shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&uniform_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[CandleVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (std::mem::size_of::<CandleVertex>() * 100000) as u64, // 100k вершин = 1.6MB буфер
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "✅ Full WebGPU renderer initialized successfully."
        );
        
        Ok(Self {
            _canvas_id: canvas.id(),
            width,
            height,
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            uniform_buffer,
            uniform_bind_group,
            num_vertices: 0,
        })
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width > 0 && new_height > 0 {
            self.width = new_width;
            self.height = new_height;
            self.config.width = new_width;
            self.config.height = new_height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn update(&mut self, chart: &Chart) {
        // Simplified update method - just store vertex count for debugging
        let candles = chart.data.get_candles();
        self.num_vertices = if candles.is_empty() { 
            0 
        } else {
            // Estimate vertex count: ~18 vertices per candle + indicators + grid
            (candles.len() * 18 + candles.len() * 6 + 100) as u32
        };
        
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("📊 Updated chart data: {} candles, estimated {} vertices", 
                candles.len(), self.num_vertices)
        );
    }

    pub fn render(&self, chart: &Chart) -> Result<(), JsValue> {
        let candle_count = chart.data.get_candles().len();
        
        // Логируем только каждые 100 кадров для производительности
        if candle_count % 100 == 0 {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!("📊 Chart has {} candles to render", candle_count)
            );
        }

        if candle_count == 0 {
            return Ok(());
        }

        // Create geometry and uniforms
        let (vertices, uniforms) = self.create_geometry(chart);
        
        if vertices.is_empty() {
            return Ok(());
        }

        // Update buffers with new data
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        let num_vertices = vertices.len() as u32;

        // Get surface texture and start rendering
        let output = self.surface
            .get_current_texture()
            .map_err(|e| {
                let error_msg = format!("Failed to get surface texture: {:?}", e);
                get_logger().error(
                    LogComponent::Infrastructure("WebGpuRenderer"),
                    &error_msg
                );
                JsValue::from_str(&error_msg)
            })?;
            
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,  // Темно-серый фон для контраста
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..num_vertices, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn create_geometry(&self, chart: &Chart) -> (Vec<CandleVertex>, ChartUniforms) {
        let candles = chart.data.get_candles();
        if candles.is_empty() {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "⚠️ No candles to render"
            );
            return (vec![], ChartUniforms::new());
        }

        // Реже логируем для производительности
        if candles.len() % 50 == 0 {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!("🔧 Creating geometry for {} candles", candles.len())
            );
        }

        let mut vertices = vec![];
        let candle_count = candles.len();
        let chart_width = 2.0; // NDC width (-1 to 1)
        let _chart_height = 2.0; // NDC height (-1 to 1)

        // Find price range
        let mut min_price = f32::MAX;
        let mut max_price = f32::MIN;
        for candle in candles {
            min_price = min_price.min(candle.ohlcv.low.value() as f32);
            max_price = max_price.max(candle.ohlcv.high.value() as f32);
        }

        // Add some padding
        let price_range = max_price - min_price;
        min_price -= price_range * 0.05;
        max_price += price_range * 0.05;

        // Calculate visible candle width and spacing
        let spacing_ratio = 0.2; // 20% spacing between candles  
        let step_size = chart_width / candle_count as f64;
        let max_candle_width = step_size * (1.0 - spacing_ratio);
        let candle_width = max_candle_width.max(0.01).min(0.06); // Reasonable width limits

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("📏 Price range: {:.2} - {:.2}, Candle width: {:.4}, step: {:.4}", 
                min_price, max_price, candle_width, step_size)
        );

        // Ensure we have a valid price range
        if (max_price - min_price).abs() < 0.01 {
            get_logger().error(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "❌ Invalid price range!"
            );
            return (vec![], ChartUniforms::new());
        }

        // Рендерим последние 300 свечей (как в реальном тикере)
        let max_visible_candles = 300;
        let start_index = if candles.len() > max_visible_candles {
            candles.len() - max_visible_candles
        } else {
            0
        };
        let visible_candles = &candles[start_index..];
        
        // Логируем реже для производительности  
        if visible_candles.len() % 50 == 0 {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!("🔧 Rendering {} candles (showing last {} of {})", 
                    visible_candles.len(), max_visible_candles, candles.len())
            );
        }

        // Create vertices for each visible candle
        let visible_count = visible_candles.len();
        let chart_width = 2.0; // NDC width (-1 to 1)
        let step_size = chart_width / visible_count as f32; // Размер одной свечи
        let candle_width = (step_size * 0.8).max(0.002).min(0.02); // 80% от step_size, но не больше 0.02 и не меньше 0.002
        
        for (i, candle) in visible_candles.iter().enumerate() {
            // Position X в NDC space [-1, 1] - новые свечи справа
            let x = -1.0 + (i as f32 + 0.5) * step_size;

            // Нормализация Y - используем почти весь экран [-0.8, 0.8]
            let price_range = max_price - min_price;
            let price_norm = |price: f64| -> f32 {
                let normalized = (price as f32 - min_price) / price_range;
                -0.8 + normalized * 1.6 // Map to [-0.8, 0.8]
            };

            let open_y = price_norm(candle.ohlcv.open.value());
            let high_y = price_norm(candle.ohlcv.high.value());
            let low_y = price_norm(candle.ohlcv.low.value());
            let close_y = price_norm(candle.ohlcv.close.value());

            // Логируем только первые 3 и последние 3 свечи
            if i < 3 || i >= visible_count - 3 {
                get_logger().info(
                    LogComponent::Infrastructure("WebGpuRenderer"),
                    &format!("🕯️ Candle {}: x={:.3}, Y=({:.3},{:.3},{:.3},{:.3}) width={:.4}", 
                        i, x, open_y, high_y, low_y, close_y, candle_width)
                );
            }

            let half_width = candle_width * 0.5;
            let body_top = open_y.max(close_y);
            let body_bottom = open_y.min(close_y);
            
            // Минимальная высота для видимости
            let min_height = 0.005;
            let actual_body_top = if (body_top - body_bottom).abs() < min_height {
                body_bottom + min_height
            } else {
                body_top
            };
            
            let is_bullish = close_y >= open_y;

            // Тело свечи
            let body_vertices = vec![
                CandleVertex::body_vertex(x - half_width, body_bottom, is_bullish),
                CandleVertex::body_vertex(x + half_width, body_bottom, is_bullish),
                CandleVertex::body_vertex(x - half_width, actual_body_top, is_bullish),
                
                CandleVertex::body_vertex(x + half_width, body_bottom, is_bullish),
                CandleVertex::body_vertex(x + half_width, actual_body_top, is_bullish),
                CandleVertex::body_vertex(x - half_width, actual_body_top, is_bullish),
            ];
            vertices.extend_from_slice(&body_vertices);
            
            // Добавляем фитили (верхний и нижний)
            let wick_width = candle_width * 0.1; // Тонкие фитили
            let wick_half = wick_width * 0.5;
            
            // Верхний фитиль
            if high_y > actual_body_top {
                let upper_wick = vec![
                    CandleVertex::wick_vertex(x - wick_half, actual_body_top),
                    CandleVertex::wick_vertex(x + wick_half, actual_body_top),
                    CandleVertex::wick_vertex(x - wick_half, high_y),
                    
                    CandleVertex::wick_vertex(x + wick_half, actual_body_top),
                    CandleVertex::wick_vertex(x + wick_half, high_y),
                    CandleVertex::wick_vertex(x - wick_half, high_y),
                ];
                vertices.extend_from_slice(&upper_wick);
            }
            
            // Нижний фитиль
            if low_y < body_bottom {
                let lower_wick = vec![
                    CandleVertex::wick_vertex(x - wick_half, low_y),
                    CandleVertex::wick_vertex(x + wick_half, low_y),
                    CandleVertex::wick_vertex(x - wick_half, body_bottom),
                    
                    CandleVertex::wick_vertex(x + wick_half, low_y),
                    CandleVertex::wick_vertex(x + wick_half, body_bottom),
                    CandleVertex::wick_vertex(x - wick_half, body_bottom),
                ];
                vertices.extend_from_slice(&lower_wick);
            }
        }

        // Добавляем сплошную линию текущей цены
        if let Some(last_candle) = visible_candles.last() {
            let current_price = last_candle.ohlcv.close.value() as f32;
            let price_range = max_price - min_price;
            let price_y = -0.8 + ((current_price - min_price) / price_range) * 1.6;
            
            // Сплошная горизонтальная линия через весь экран
            let line_thickness = 0.002;
            let price_line = vec![
                CandleVertex::current_price_vertex(-1.0, price_y - line_thickness),
                CandleVertex::current_price_vertex(1.0, price_y - line_thickness),
                CandleVertex::current_price_vertex(-1.0, price_y + line_thickness),
                
                CandleVertex::current_price_vertex(1.0, price_y - line_thickness),
                CandleVertex::current_price_vertex(1.0, price_y + line_thickness),
                CandleVertex::current_price_vertex(-1.0, price_y + line_thickness),
            ];
            vertices.extend_from_slice(&price_line);
        }

        // 📈 Добавляем скользящие средние (SMA20 и EMA12)
        vertices.extend(self.create_moving_averages(visible_candles, min_price, max_price));

        // Логируем только если много вершин
        if vertices.len() > 1000 {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!("✅ Generated {} vertices for {} visible candles + indicators", vertices.len(), visible_candles.len())
            );
        }

        // Identity matrix - vertices are already in NDC coordinates [-1, 1]
        let view_proj_matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];

        // Create uniforms with corrected parameters
        let uniforms = ChartUniforms {
            view_proj_matrix,
            viewport: [self.width as f32, self.height as f32, min_price, max_price],
            time_range: [0.0, visible_candles.len() as f32, visible_candles.len() as f32, 0.0],
            bullish_color: [0.447, 0.776, 0.522, 1.0],   // #72c685 - зеленый
            bearish_color: [0.882, 0.420, 0.282, 1.0],   // #e16b48 - красный
            wick_color: [0.6, 0.6, 0.6, 0.9],            // Светло-серый
            sma20_color: [1.0, 0.2, 0.2, 0.9],           // Ярко-красный
            sma50_color: [1.0, 0.8, 0.0, 0.9],           // Желтый
            sma200_color: [0.2, 0.4, 0.8, 0.9],          // Синий
            ema12_color: [0.8, 0.2, 0.8, 0.9],           // Фиолетовый
            ema26_color: [0.0, 0.8, 0.8, 0.9],           // Голубой
            current_price_color: [1.0, 1.0, 0.0, 0.8],   // 💰 Ярко-желтый
            render_params: [candle_width as f32, spacing_ratio as f32, 0.004, 0.0],
        };

        (vertices, uniforms)
    }

    /// 📈 Создать геометрию для скользящих средних
    fn create_moving_averages(&self, candles: &[crate::domain::market_data::Candle], min_price: f32, max_price: f32) -> Vec<CandleVertex> {
        use crate::infrastructure::rendering::gpu_structures::{CandleGeometry, IndicatorType};
        
        if candles.len() < 20 {
            return Vec::new(); // Недостаточно данных для SMA20
        }

        let mut vertices = Vec::new();
        let candle_count = candles.len();
        let step_size = 2.0 / candle_count as f32;
        let price_range = max_price - min_price;

        // Функция для нормализации цены в NDC координаты
        let price_to_ndc = |price: f32| -> f32 {
            -0.8 + ((price - min_price) / price_range) * 1.6
        };

        // Расчёт SMA20 (Simple Moving Average 20)
        let mut sma20_points = Vec::new();
        for i in 19..candle_count { // Начинаем с 20-й свечи
            let sum: f32 = candles[i-19..=i].iter()
                .map(|c| c.ohlcv.close.value() as f32)
                .sum();
            let sma20 = sum / 20.0;
            
            let x = -1.0 + (i as f32 + 0.5) * step_size;
            let y = price_to_ndc(sma20);
            sma20_points.push((x, y));
        }

        // Расчёт EMA12 (Exponential Moving Average 12)
        let mut ema12_points = Vec::new();
        if candle_count >= 12 {
            let multiplier = 2.0 / (12.0 + 1.0); // EMA multiplier
            let mut ema = candles[0].ohlcv.close.value() as f32; // Начальное значение
            
            for i in 1..candle_count {
                let close = candles[i].ohlcv.close.value() as f32;
                ema = (close * multiplier) + (ema * (1.0 - multiplier));
                
                if i >= 11 { // Показываем EMA только после 12 свечей
                    let x = -1.0 + (i as f32 + 0.5) * step_size;
                    let y = price_to_ndc(ema);
                    ema12_points.push((x, y));
                }
            }
        }

        // Создаём геометрию для линий
        if !sma20_points.is_empty() {
            let sma20_vertices = CandleGeometry::create_indicator_line_vertices(
                &sma20_points, 
                IndicatorType::SMA20, 
                0.003 // Толщина линии
            );
            vertices.extend(sma20_vertices);
        }

        if !ema12_points.is_empty() {
            let ema12_vertices = CandleGeometry::create_indicator_line_vertices(
                &ema12_points, 
                IndicatorType::EMA12, 
                0.003 // Толщина линии
            );
            vertices.extend(ema12_vertices);
        }

        if !vertices.is_empty() {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!("📈 Generated {} SMA20 points, {} EMA12 points, {} total MA vertices", 
                    sma20_points.len(), ema12_points.len(), vertices.len())
            );
        }

        vertices
    }

    /// Получить информацию о производительности
    pub fn get_performance_info(&self) -> String {
        "{\"backend\":\"WebGPU\",\"parallel\":true,\"status\":\"ready\",\"gpu_threads\":\"unlimited\"}".to_string()
    }

    /// Переключить видимость линии индикатора
    pub fn toggle_line_visibility(&mut self, _line_name: &str) {
        // Implementation needed
    }

    /// Проверить попадание в область чекбокса легенды
    pub fn check_legend_checkbox_click(&self, _mouse_x: f32, _mouse_y: f32) -> Option<String> {
        // Implementation needed
        None
    }

    /// Самый простой тест - только очистка в яркий цвет (без геометрии)
    pub fn test_clear_only(&self) -> Result<(), JsValue> {
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🌈 CLEAR-ONLY: Testing surface with bright yellow clear color..."
        );

        let output = self.surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Surface error: {:?}", e)))?;
            
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Only Encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Only Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0, g: 1.0, b: 0.0, a: 1.0, // ЯРКО-ЖЕЛТЫЙ
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // НЕ рисуем никакой геометрии - только очистка!
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "🌈 Clear render pass completed"
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "✅ CLEAR-ONLY TEST COMPLETED!"
        );

        Ok(())
    }

    /// Ультра-простой тест - красный прямоугольник с фиксированным цветом в шейдере
    pub fn test_simple_red_quad(&self) -> Result<(), JsValue> {
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🔴 ULTRA-SIMPLE: Drawing red quad with fixed shader color..."
        );

        // Создаем простейший четырехугольник с фиксированными координатами
        let test_vertices = vec![
            // Треугольник 1
            CandleVertex { position_x: -0.8, position_y: -0.8, element_type: 99.0, color_type: 99.0 },
            CandleVertex { position_x:  0.8, position_y: -0.8, element_type: 99.0, color_type: 99.0 },
            CandleVertex { position_x: -0.8, position_y:  0.8, element_type: 99.0, color_type: 99.0 },
            
            // Треугольник 2  
            CandleVertex { position_x:  0.8, position_y: -0.8, element_type: 99.0, color_type: 99.0 },
            CandleVertex { position_x:  0.8, position_y:  0.8, element_type: 99.0, color_type: 99.0 },
            CandleVertex { position_x: -0.8, position_y:  0.8, element_type: 99.0, color_type: 99.0 },
        ];

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🔴 Created {} ultra-simple vertices", test_vertices.len())
        );

        // Записываем в буфер
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&test_vertices));
        
        // Простейшие uniforms
        let test_uniforms = ChartUniforms::default();
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[test_uniforms]));

        let output = self.surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Surface error: {:?}", e)))?;
            
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test Simple Quad Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Test Simple Quad Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.2, g: 0.0, b: 0.5, a: 1.0, // Фиолетовый фон для контраста
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1);

            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "🎨 Drew ultra-simple quad with 6 vertices"
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "✅ ULTRA-SIMPLE QUAD RENDERED!"
        );

        Ok(())
    }

    /// Простой тест - рисует большой прямоугольник в центре
    pub fn test_big_rectangle(&self) -> Result<(), JsValue> {
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🟩 TESTING: Drawing big green rectangle in center..."
        );

        // Создаем большой прямоугольник в центре экрана
        let test_vertices = vec![
            // Первый треугольник
            CandleVertex::body_vertex(-0.5, -0.5, true),  // Лево-низ
            CandleVertex::body_vertex(0.5, -0.5, true),   // Право-низ
            CandleVertex::body_vertex(-0.5, 0.5, true),   // Лево-верх
            
            // Второй треугольник
            CandleVertex::body_vertex(0.5, -0.5, true),   // Право-низ
            CandleVertex::body_vertex(0.5, 0.5, true),    // Право-верх
            CandleVertex::body_vertex(-0.5, 0.5, true),   // Лево-верх
        ];

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🟩 Created {} test rectangle vertices", test_vertices.len())
        );

        // Записываем в буфер
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&test_vertices));
        
        // Создаем тестовые uniforms
        let test_uniforms = ChartUniforms::default();
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[test_uniforms]));

        let output = self.surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Surface error: {:?}", e)))?;
            
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test Rectangle Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Test Rectangle Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1, g: 0.1, b: 0.3, a: 1.0, // Темно-синий фон
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1); // Рисуем 6 вершин прямоугольника

            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "🎨 Drew test rectangle with 6 vertices"
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "✅ TEST RECTANGLE RENDERED SUCCESSFULLY!"
        );

        Ok(())
    }

    /// Базовый тест рендеринга - рисует красный треугольник
    pub fn test_basic_triangle(&self) -> Result<(), JsValue> {
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🔴 TESTING: Drawing basic red triangle..."
        );

        // Создаем простейшие вершины треугольника
        let test_vertices = vec![
            CandleVertex::body_vertex(0.0, 0.5, true),   // Верх (зеленый)
            CandleVertex::body_vertex(-0.5, -0.5, false), // Лево-низ (красный)
            CandleVertex::body_vertex(0.5, -0.5, true),  // Право-низ (зеленый)
        ];

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🔺 Created {} test vertices", test_vertices.len())
        );

        // Записываем в буфер
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&test_vertices));
        
        // Создаем тестовые uniforms
        let test_uniforms = ChartUniforms::default();
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[test_uniforms]));

        let output = self.surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Surface error: {:?}", e)))?;
            
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test Triangle Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Test Triangle Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0, g: 0.0, b: 0.3, a: 1.0, // Темно-синий фон
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..3, 0..1); // Рисуем 3 вершины треугольника

            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "🎨 Drew test triangle with 3 vertices"
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "✅ TEST TRIANGLE RENDERED SUCCESSFULLY!"
        );

        Ok(())
    }
}

// Future expansion: Complete WebGPU pipeline implementation
// with advanced shaders, complex buffers and enhanced GPU rendering 