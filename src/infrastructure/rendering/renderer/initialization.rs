use super::*;
use leptos::SignalGetUntracked;
use std::collections::VecDeque;

impl WebGpuRenderer {
    pub async fn is_webgpu_supported() -> bool {
        if let Some(window) = web_sys::window() {
            let navigator = window.navigator();
            let has_gpu = js_sys::Reflect::has(&navigator, &"gpu".into()).unwrap_or(false);
            if !has_gpu {
                return false;
            }

            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    compatible_surface: None,
                    force_fallback_adapter: false,
                })
                .await
                .is_ok()
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
            &format!(
                "🎯 Canvas found: {}x{} -> setting to {}x{}",
                canvas.width(),
                canvas.height(),
                width,
                height
            ),
        );

        canvas.set_width(width);
        canvas.set_height(height);

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🎯 Canvas configured: {}x{}", canvas.width(), canvas.height()),
        );

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {}", e)))?;

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🎯 WebGPU surface created successfully",
        );

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to find adapter: {:?}", e)))?;

        let adapter_info = adapter.get_info();
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!(
                "\u{1f5a5}\u{fe0f} Adapter: {}, driver: {}, type: {:?}",
                adapter_info.name, adapter_info.driver_info, adapter_info.device_type
            ),
        );

        // Get the adapter's supported limits to ensure compatibility
        let supported_limits = adapter.limits();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Device"),
                required_features: wgpu::Features::empty(),
                // Use the adapter's own supported limits
                required_limits: supported_limits,
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
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
            &format!(
                "🎯 Surface config: {}x{}, format: {:?}, present_mode: {:?}, alpha: {:?}",
                config.width, config.height, config.format, config.present_mode, config.alpha_mode
            ),
        );

        surface.configure(&device, &config);

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🎯 Surface configured successfully",
        );

        let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("MSAA Texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: MSAA_SAMPLE_COUNT,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_view = msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

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
            label: Some("Simple Candle Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../simple_shader.wgsl").into()),
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
                count: MSAA_SAMPLE_COUNT,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: (std::mem::size_of::<CandleVertex>() * 100000) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "✅ Full WebGPU renderer initialized successfully.",
        );

        let renderer = Self {
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
            msaa_texture,
            msaa_view,
            template_vertices: 0,
            cached_vertices: Vec::new(),
            cached_uniforms: ChartUniforms::new(),
            cached_candle_count: 0,
            cached_zoom_level: 1.0,
            cached_hash: 0,
            cached_data_hash: 0,
            cached_line_visibility: LineVisibility::default(),
            zoom_level: 1.0,
            pan_offset: 0.0,
            last_frame_time: 0.0,
            fps_log: VecDeque::new(),
            line_visibility: LineVisibility::default(),
        };

        renderer.log_gpu_memory_usage();

        Ok(renderer)
    }

    pub fn resize(&mut self, new_width: u32, new_height: u32) {
        if new_width > 0 && new_height > 0 {
            self.width = new_width;
            self.height = new_height;
            self.config.width = new_width;
            self.config.height = new_height;
            self.surface.configure(&self.device, &self.config);
            self.msaa_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("MSAA Texture"),
                size: wgpu::Extent3d {
                    width: new_width,
                    height: new_height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: MSAA_SAMPLE_COUNT,
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.msaa_view = self.msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());
        }
    }

    pub fn update(&mut self, chart: &Chart) {
        // Simplified update method - just store vertex count for debugging
        use crate::app::current_interval;
        let interval = current_interval().get_untracked();
        let candles = chart
            .get_series(interval)
            .map(|s| s.get_candles())
            .unwrap_or_else(|| chart.get_series_for_zoom(self.zoom_level).get_candles());
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("📊 Updated chart data: {} candles", candles.len()),
        );
    }

    /// 🔍 Set zoom and pan parameters
    pub fn set_zoom_params(&mut self, zoom_level: f64, pan_offset: f64) {
        self.zoom_level = zoom_level;
        self.pan_offset = pan_offset;
        // Force geometry refresh on next render
        self.cached_zoom_level = f64::MAX;
    }
}
