use wasm_bindgen::prelude::*;
use crate::domain::{
    chart::Chart,
    logging::{LogComponent, get_logger},
    market_data::services::MarketAnalysisService,
};
use wgpu::util::DeviceExt;
use crate::infrastructure::rendering::gpu_structures::{CandleVertex, ChartUniforms, CandleGeometry, IndicatorType};
use gloo::utils::{document, window};
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
                match js_sys::Reflect::has(&navigator, &"gpu".into()) {
                    Ok(has_gpu) => has_gpu,
                    Err(_) => false,
                }
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
        
        canvas.set_width(width);
        canvas.set_height(height);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {}", e)))?;

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
        surface.configure(&device, &config);

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
        // ... update vertex and uniform buffers based on chart data ...
        let candles = chart.data.get_candles();
        if candles.is_empty() {
            self.num_vertices = 0;
            return;
        }

        let mut vertices = vec![];
        for candle in candles {
            // This is a simplified conversion, real logic would use viewport to normalize
             vertices.extend_from_slice(&CandleGeometry::create_candle_vertices(
                 candle.timestamp.as_f64(),
                 candle.ohlcv.open.value(),
                 candle.ohlcv.high.value(),
                 candle.ohlcv.low.value(),
                 candle.ohlcv.close.value(),
                 0.0, 0.0, 0.0, 0.0, 0.0, 0.0 // Placeholder values, need real normalization
             ));
        }

        self.num_vertices = vertices.len() as u32;
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        // Update uniforms
        let uniforms = ChartUniforms::new(); // placeholder
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn render(&self, chart: &Chart) -> Result<(), JsValue> {
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🎨 Starting WebGPU render..."
        );

        let candle_count = chart.data.get_candles().len();
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("📊 Chart has {} candles to render", candle_count)
        );

        if candle_count == 0 {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "⚠️ No candles to render, skipping..."
            );
            return Ok(());
        }

        // Update buffers
        let (vertices, uniforms) = self.create_geometry(chart);
        
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("📊 Created {} vertices for {} candles", vertices.len(), chart.data.get_candles().len())
        );

        if vertices.is_empty() {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "⚠️ No vertices to render, skipping..."
            );
            return Ok(());
        }

        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        let num_vertices = vertices.len() as u32;

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "📝 Updated vertex and uniform buffers"
        );

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
            
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🖼️ Got surface texture"
        );

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🎬 Created command encoder"
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,  // Красивый синий фон
                            g: 0.2,
                            b: 0.4,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "🎭 Started render pass with blue background"
            );

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..num_vertices, 0..1);

            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!("🖌️ Drew {} vertices", num_vertices)
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "✅ WebGPU render completed successfully!"
        );

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

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🔧 Creating geometry for {} candles", candles.len())
        );

        // Логируем информацию о данных для отладки
        if let (Some(first), Some(last)) = (candles.first(), candles.last()) {
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!("📅 Data span: {} to {} ({:.1} hours)", 
                    first.timestamp.value(),
                    last.timestamp.value(),
                    (last.timestamp.value() - first.timestamp.value()) as f64 / 3600000.0
                )
            );
        }

        let mut vertices = vec![];
        let candle_count = candles.len();
        let chart_width = 2.0; // NDC width (-1 to 1)
        let chart_height = 2.0; // NDC height (-1 to 1)

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
        let spacing_ratio = 0.3; // 30% spacing between candles  
        let step_size = chart_width / candle_count as f64;
        let max_candle_width = step_size * (1.0 - spacing_ratio); // Width with proper spacing
        let candle_width = max_candle_width.max(0.02).min(0.08); // Min 0.02, Max 0.08 NDC units for visibility

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("📏 Candle width: {:.6}, step size: {:.6}", candle_width, step_size)
        );

        // Create vertices for each candle
        for (i, candle) in candles.iter().enumerate() {
            // Better X positioning with proper spacing
            let x = -1.0 + (i as f64 + 0.5) * step_size; // Center each candle in its slot

            // Normalize prices to [-1, 1] range
            let open_y = -1.0 + ((candle.ohlcv.open.value() as f32 - min_price) / (max_price - min_price)) * chart_height as f32;
            let high_y = -1.0 + ((candle.ohlcv.high.value() as f32 - min_price) / (max_price - min_price)) * chart_height as f32;
            let low_y = -1.0 + ((candle.ohlcv.low.value() as f32 - min_price) / (max_price - min_price)) * chart_height as f32;
            let close_y = -1.0 + ((candle.ohlcv.close.value() as f32 - min_price) / (max_price - min_price)) * chart_height as f32;

            // Create vertices using the CandleGeometry helper
            let candle_vertices = CandleGeometry::create_candle_vertices(
                candle.timestamp.as_f64(), // timestamp
                candle.ohlcv.open.value() as f32, // open  
                candle.ohlcv.high.value() as f32, // high
                candle.ohlcv.low.value() as f32,  // low
                candle.ohlcv.close.value() as f32, // close
                x as f32, // x_normalized
                open_y,   // open_y
                high_y,   // high_y  
                low_y,    // low_y
                close_y,  // close_y
                candle_width as f32, // width
            );
            vertices.extend_from_slice(&candle_vertices);
        }

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("✅ Generated {} vertices for {} candles", vertices.len(), candle_count)
        );

        // Вычисляем и добавляем технические индикаторы
        let analysis_service = MarketAnalysisService::new();
        let mas = analysis_service.calculate_multiple_mas(candles);
        
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🔍 Indicators calculated: SMA20({} pts), SMA50({} pts), EMA12({} pts)", 
                mas.sma_20.len(), mas.sma_50.len(), mas.ema_12.len())
        );
        
        // Добавляем SMA 20
        if !mas.sma_20.is_empty() {
            let sma20_points: Vec<(f32, f32)> = mas.sma_20.iter().enumerate().map(|(i, price)| {
                let x = -1.0 + ((i + 20) as f64 + 0.5) * step_size; // Смещение на период SMA
                let y = -1.0 + ((price.value() - min_price) / (max_price - min_price)) * chart_height as f32;
                (x as f32, y)
            }).collect();
            
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!("📍 SMA20 sample points: start=({:.3}, {:.3}), end=({:.3}, {:.3})", 
                    sma20_points[0].0, sma20_points[0].1,
                    sma20_points[sma20_points.len()-1].0, sma20_points[sma20_points.len()-1].1)
            );
            
            let sma20_vertices = CandleGeometry::create_indicator_line_vertices(
                &sma20_points, IndicatorType::SMA20, 0.004  // Тонкие линии для индикаторов
            );
            vertices.extend_from_slice(&sma20_vertices);
            
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                &format!("📈 SMA20: added {} vertices (width: {:.4})", sma20_vertices.len(), candle_width)
            );
        }
        
        // Добавляем SMA 50
        if !mas.sma_50.is_empty() {
            let sma50_points: Vec<(f32, f32)> = mas.sma_50.iter().enumerate().map(|(i, price)| {
                let x = -1.0 + ((i + 50) as f64 + 0.5) * step_size; // Смещение на период SMA
                let y = -1.0 + ((price.value() - min_price) / (max_price - min_price)) * chart_height as f32;
                (x as f32, y)
            }).collect();
            
            let sma50_vertices = CandleGeometry::create_indicator_line_vertices(
                &sma50_points, IndicatorType::SMA50, 0.004  // Тонкие линии для индикаторов
            );
            vertices.extend_from_slice(&sma50_vertices);
        }
        
        // Добавляем EMA 12
        if !mas.ema_12.is_empty() {
            let ema12_points: Vec<(f32, f32)> = mas.ema_12.iter().enumerate().map(|(i, price)| {
                let x = -1.0 + ((i + 12) as f64 + 0.5) * step_size; // Смещение на период EMA
                let y = -1.0 + ((price.value() - min_price) / (max_price - min_price)) * chart_height as f32;
                (x as f32, y)
            }).collect();
            
            let ema12_vertices = CandleGeometry::create_indicator_line_vertices(
                &ema12_points, IndicatorType::EMA12, 0.004  // Тонкие линии для индикаторов
            );
            vertices.extend_from_slice(&ema12_vertices);
        }
        
        // Добавляем сетку цен
        let grid_vertices = CandleGeometry::create_price_grid(
            min_price, max_price, chart_width as f32, chart_height as f32, 8, 5
        );
        vertices.extend_from_slice(&grid_vertices);
        
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("📈 Added: SMA20({} pts), SMA50({} pts), EMA12({} pts), Grid({} lines)", 
                mas.sma_20.len(), mas.sma_50.len(), mas.ema_12.len(), grid_vertices.len() / 6)
        );

        // Create uniforms
        let uniforms = ChartUniforms {
            view_proj_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            viewport: [self.width as f32, self.height as f32, min_price as f32, max_price as f32],
            time_range: [0.0, candle_count as f32, candle_count as f32, 0.0],
            bullish_color: [0.447, 0.776, 0.522, 1.0],   // #72c685 - ваш зеленый
            bearish_color: [0.882, 0.420, 0.282, 1.0],   // #e16b48 - ваш красный
            wick_color: [0.5, 0.5, 0.5, 0.8],            // Gray
            sma20_color: [1.0, 0.0, 0.0, 1.0],           // Bright Red
            sma50_color: [1.0, 0.8, 0.0, 1.0],           // Yellow
            sma200_color: [0.2, 0.4, 0.8, 1.0],          // Blue
            ema12_color: [0.8, 0.2, 0.8, 1.0],           // Purple
            ema26_color: [0.0, 0.8, 0.8, 1.0],           // Cyan
            render_params: [candle_width as f32, 1.0, 1.0, 0.0],
        };

        (vertices, uniforms)
    }

    /// Получить информацию о производительности
    pub fn get_performance_info(&self) -> String {
        "{\"backend\":\"WebGPU\",\"parallel\":true,\"status\":\"ready\",\"gpu_threads\":\"unlimited\"}".to_string()
    }

    /// Переключить видимость линии индикатора
    pub fn toggle_line_visibility(&mut self, line_name: &str) {
        // Implementation needed
    }

    /// Проверить попадание в область чекбокса легенды
    pub fn check_legend_checkbox_click(&self, mouse_x: f32, mouse_y: f32) -> Option<String> {
        // Implementation needed
        None
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

// TODO: В будущем здесь будет полная реализация WebGPU pipeline
// с настоящими шейдерами, буферами и рендерингом на GPU 