use super::*;
use crate::domain::logging::LogComponent;
use crate::domain::market_data::TimeInterval;
#[cfg(debug_assertions)]
use crate::log_info;
use leptos::{SignalGetUntracked, SignalSet};
use serde_json;

impl WebGpuRenderer {
    pub fn data_revision(chart: &Chart) -> u64 {
        chart.revision()
    }

    fn update_cached_geometry(
        &mut self,
        vertices: Vec<CandleVertex>,
        _instances: Vec<CandleInstance>,
        uniforms: ChartUniforms,
    ) -> bool {
        self.cached_vertices = vertices;
        self.cached_uniforms = uniforms;
        self.cached_hash = self.cached_hash.wrapping_add(1);
        self.cached_line_visibility = self.line_visibility.clone();
        self.template_vertices = self.cached_vertices.len() as u32;

        #[cfg(not(test))]
        self.write_buffers();

        true
    }

    #[cfg(not(test))]
    fn write_buffers(&mut self) {
        let vertex_bytes = bytemuck::cast_slice(&self.cached_vertices);
        let required_size = vertex_bytes.len() as u64;
        if required_size > self.vertex_buffer.size() {
            let new_size = required_size.next_power_of_two();
            self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Resizable Vertex Buffer"),
                size: new_size,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        let uniform_copy = self.cached_uniforms;
        let uniform_bytes = bytemuck::bytes_of(&uniform_copy);
        self.queue.write_buffer(&self.vertex_buffer, 0, vertex_bytes);
        self.queue.write_buffer(&self.uniform_buffer, 0, uniform_bytes);
    }

    pub fn cache_geometry_for_test(&mut self, chart: &Chart) {
        let (inst, verts, uni) = self.create_geometry(chart);
        self.update_cached_geometry(verts, inst, uni);
        self.cached_data_revision = Self::data_revision(chart);
    }

    pub fn cached_hash_for_test(&self) -> u64 {
        self.cached_hash
    }

    pub fn render_width(&self) -> u32 {
        self.width
    }

    pub fn create_geometry_for_test(
        &self,
        chart: &Chart,
    ) -> (Vec<CandleInstance>, Vec<CandleVertex>, ChartUniforms) {
        self.create_geometry(chart)
    }

    pub fn render(&mut self, chart: &Chart) -> Result<(), JsValue> {
        use crate::app::current_interval;
        let interval = current_interval().get_untracked();
        let candle_count =
            chart.get_series(interval).map(|s| s.get_candles().len()).unwrap_or_else(|| {
                chart
                    .get_series(TimeInterval::TwoSeconds)
                    .expect("base series not found")
                    .get_candles()
                    .len()
            });

        if candle_count == 0 {
            return Ok(());
        }

        let data_revision = Self::data_revision(chart);
        let data_changed = data_revision != self.cached_data_revision;
        let visibility_changed = self.line_visibility != self.cached_line_visibility;

        let geometry_needs_update = candle_count != self.cached_candle_count
            || (self.zoom_level - self.cached_zoom_level).abs() > f64::EPSILON;

        if geometry_needs_update || data_changed || visibility_changed {
            let (instances, vertices, uniforms) = self.create_geometry(chart);
            if instances.is_empty() {
                return Ok(());
            }
            self.cached_candle_count = candle_count;
            self.cached_zoom_level = self.zoom_level;
            self.cached_data_revision = data_revision;
            self.update_cached_geometry(vertices, instances, uniforms);
        }

        // Skip empty check for simple shader - we don't use instances
        if self.cached_vertices.is_empty() {
            return Ok(());
        }

        let num_vertices = self.template_vertices;

        // Get surface texture and start rendering
        let output = self.surface.get_current_texture().map_err(|e| {
            let error_msg = format!("Failed to get surface texture: {:?}", e);
            get_logger().error(LogComponent::Infrastructure("WebGpuRenderer"), &error_msg);
            JsValue::from_str(&error_msg)
        })?;

        let surface_view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let start_pass = web_sys::window().and_then(|w| w.performance()).map(|p| p.now());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&surface_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.145,
                            g: 0.196,
                            b: 0.259,
                            a: 1.0, // Chart background color
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

        if let Some(start) = start_pass
            && let Some(window) = web_sys::window()
            && let Some(perf) = window.performance()
        {
            let end = perf.now();
            let duration = end - start;
            self.render_time_log.push_back(duration);
            if self.render_time_log.len() > 60 {
                self.render_time_log.pop_front();
            }
            crate::app::global_render_time_ms().set(duration);
        }

        output.present();

        Ok(())
    }

    /// Get renderer performance information
    pub fn get_performance_info(&self) -> String {
        let avg_render_ms = if self.render_time_log.is_empty() {
            0.0
        } else {
            self.render_time_log.iter().sum::<f64>() / self.render_time_log.len() as f64
        };
        let avg_fps = if avg_render_ms > 0.0 { 1000.0 / avg_render_ms } else { 0.0 };

        serde_json::json!({
            "backend": "WebGPU",
            "parallel": true,
            "status": "ready",
            "gpu_threads": "unlimited",
            "avg_fps": avg_fps,
            "avg_render_ms": avg_render_ms
        })
        .to_string()
    }

    /// Log GPU memory usage and return statistics as JSON
    pub fn log_gpu_memory_usage(&self) -> String {
        if let Some(report) = self.device.generate_allocator_report() {
            let reserved = report.total_reserved_bytes / 1024 / 1024;
            let allocated = report.total_allocated_bytes / 1024 / 1024;
            let msg = format!(
                "\u{1f4c8} GPU memory reserved: {} MB, allocated: {} MB",
                reserved, allocated
            );
            get_logger().info(LogComponent::Infrastructure("WebGpuRenderer"), &msg);
            serde_json::json!({
                "reserved_mb": reserved,
                "allocated_mb": allocated
            })
            .to_string()
        } else {
            get_logger().warn(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "\u{26a0}\u{fe0f} GPU memory report unavailable",
            );
            "{}".to_string()
        }
    }

    /// Toggle indicator line visibility
    pub fn toggle_line_visibility(&mut self, line_name: &str) {
        let state = match line_name {
            "sma20" => {
                self.line_visibility.sma_20 = !self.line_visibility.sma_20;
                Some(self.line_visibility.sma_20)
            }
            "sma50" => {
                self.line_visibility.sma_50 = !self.line_visibility.sma_50;
                Some(self.line_visibility.sma_50)
            }
            "sma200" => {
                self.line_visibility.sma_200 = !self.line_visibility.sma_200;
                Some(self.line_visibility.sma_200)
            }
            "ema12" => {
                self.line_visibility.ema_12 = !self.line_visibility.ema_12;
                Some(self.line_visibility.ema_12)
            }
            "ema26" => {
                self.line_visibility.ema_26 = !self.line_visibility.ema_26;
                Some(self.line_visibility.ema_26)
            }
            _ => None,
        };

        #[cfg(debug_assertions)]
        if let Some(state) = state {
            log_info!(
                LogComponent::Infrastructure("LegendToggle"),
                "Line {} visible: {}",
                line_name,
                state
            );
        }
        #[cfg(not(debug_assertions))]
        let _ = state;
        crate::app::global_line_visibility().set(self.line_visibility.clone());
    }

    pub fn line_visibility(&self) -> LineVisibility {
        self.line_visibility.clone()
    }

    /// Check if the legend checkbox was clicked
    pub fn check_legend_checkbox_click(&self, mouse_x: f32, mouse_y: f32) -> Option<String> {
        const LEGEND_LEFT: f32 = 10.0;
        const LEGEND_TOP: f32 = 10.0;
        const BOX_SIZE: f32 = 20.0;
        const BOX_GAP: f32 = 30.0;

        let lines = ["sma20", "sma50", "sma200", "ema12", "ema26"];

        for (i, name) in lines.iter().enumerate() {
            let x0 = LEGEND_LEFT;
            let y0 = LEGEND_TOP + i as f32 * BOX_GAP;
            let x1 = x0 + BOX_SIZE;
            let y1 = y0 + BOX_SIZE;
            if mouse_x >= x0 && mouse_x <= x1 && mouse_y >= y0 && mouse_y <= y1 {
                return Some((*name).to_string());
            }
        }

        None
    }

    /// Simplest test - clear the screen with a bright color (no geometry)
    pub fn test_clear_only(&self) -> Result<(), JsValue> {
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🌈 CLEAR-ONLY: Testing surface with bright yellow clear color...",
        );

        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Surface error: {:?}", e)))?;

        let surface_view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Only Encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Only Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&surface_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 1.0,
                            g: 1.0,
                            b: 0.0,
                            a: 1.0, // bright yellow
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            // Do not draw any geometry - clear only!
            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "🌈 Clear render pass completed",
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        get_logger()
            .info(LogComponent::Infrastructure("WebGpuRenderer"), "✅ CLEAR-ONLY TEST COMPLETED!");

        Ok(())
    }

    /// Ultra-simple test - red rectangle with fixed shader color
    pub fn test_simple_red_quad(&self) -> Result<(), JsValue> {
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🔴 ULTRA-SIMPLE: Drawing red quad with fixed shader color...",
        );

        // Create a basic rectangle with fixed coordinates
        let test_vertices = vec![
            // Triangle 1
            CandleVertex {
                position_x: -0.8,
                position_y: -0.8,
                element_type: 99.0,
                color_type: 99.0,
            },
            CandleVertex {
                position_x: 0.8,
                position_y: -0.8,
                element_type: 99.0,
                color_type: 99.0,
            },
            CandleVertex {
                position_x: -0.8,
                position_y: 0.8,
                element_type: 99.0,
                color_type: 99.0,
            },
            // Triangle 2
            CandleVertex {
                position_x: 0.8,
                position_y: -0.8,
                element_type: 99.0,
                color_type: 99.0,
            },
            CandleVertex { position_x: 0.8, position_y: 0.8, element_type: 99.0, color_type: 99.0 },
            CandleVertex {
                position_x: -0.8,
                position_y: 0.8,
                element_type: 99.0,
                color_type: 99.0,
            },
        ];

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🔴 Created {} ultra-simple vertices", test_vertices.len()),
        );

        // Write to buffer
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&test_vertices));

        // Basic uniforms
        let test_uniforms = ChartUniforms::default();
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[test_uniforms]));

        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Surface error: {:?}", e)))?;

        let surface_view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test Simple Quad Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Test Simple Quad Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&surface_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.2,
                            g: 0.0,
                            b: 0.5,
                            a: 1.0, // purple background for contrast
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
                "🎨 Drew ultra-simple quad with 6 vertices",
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        get_logger()
            .info(LogComponent::Infrastructure("WebGpuRenderer"), "✅ ULTRA-SIMPLE QUAD RENDERED!");

        Ok(())
    }

    /// Simple test - draw a large rectangle in the center
    pub fn test_big_rectangle(&self) -> Result<(), JsValue> {
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🟩 TESTING: Drawing big green rectangle in center...",
        );

        // Create a large rectangle in the center of the screen
        let test_vertices = vec![
            // First triangle
            CandleVertex::body_vertex(-0.5, -0.5, true), // left-bottom
            CandleVertex::body_vertex(0.5, -0.5, true),  // right-bottom
            CandleVertex::body_vertex(-0.5, 0.5, true),  // left-top
            // Second triangle
            CandleVertex::body_vertex(0.5, -0.5, true), // right-bottom
            CandleVertex::body_vertex(0.5, 0.5, true),  // right-top
            CandleVertex::body_vertex(-0.5, 0.5, true), // left-top
        ];

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🟩 Created {} test rectangle vertices", test_vertices.len()),
        );

        // Write to buffer
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&test_vertices));

        // Create test uniforms
        let test_uniforms = ChartUniforms::default();
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[test_uniforms]));

        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Surface error: {:?}", e)))?;

        let surface_view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test Rectangle Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Test Rectangle Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&surface_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.3,
                            a: 1.0, // dark blue background
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
            render_pass.draw(0..6, 0..1); // Draw 6 rectangle vertices

            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "🎨 Drew test rectangle with 6 vertices",
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "✅ TEST RECTANGLE RENDERED SUCCESSFULLY!",
        );

        Ok(())
    }

    /// Basic rendering test - draws a red triangle
    pub fn test_basic_triangle(&self) -> Result<(), JsValue> {
        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "🔴 TESTING: Drawing basic red triangle...",
        );

        // Create the simplest triangle vertices
        let test_vertices = vec![
            CandleVertex::body_vertex(0.0, 0.5, true),    // top (green)
            CandleVertex::body_vertex(-0.5, -0.5, false), // left-bottom (red)
            CandleVertex::body_vertex(0.5, -0.5, true),   // right-bottom (green)
        ];

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            &format!("🔺 Created {} test vertices", test_vertices.len()),
        );

        // Write to buffer
        self.queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&test_vertices));

        // Create test uniforms
        let test_uniforms = ChartUniforms::default();
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[test_uniforms]));

        let output = self
            .surface
            .get_current_texture()
            .map_err(|e| JsValue::from_str(&format!("Surface error: {:?}", e)))?;

        let surface_view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Test Triangle Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Test Triangle Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_view,
                    resolve_target: Some(&surface_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.3,
                            a: 1.0, // dark blue background
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
            render_pass.draw(0..3, 0..1); // Draw 3 triangle vertices

            get_logger().info(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "🎨 Drew test triangle with 3 vertices",
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        get_logger().info(
            LogComponent::Infrastructure("WebGpuRenderer"),
            "✅ TEST TRIANGLE RENDERED SUCCESSFULLY!",
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(invalid_value)]
    fn dummy_renderer() -> WebGpuRenderer {
        unsafe {
            WebGpuRenderer {
                _canvas_id: String::new(),
                width: 0,
                height: 0,
                surface: std::mem::MaybeUninit::zeroed().assume_init(),
                device: std::mem::MaybeUninit::zeroed().assume_init(),
                queue: std::mem::MaybeUninit::zeroed().assume_init(),
                config: std::mem::MaybeUninit::zeroed().assume_init(),
                render_pipeline: std::mem::MaybeUninit::zeroed().assume_init(),
                vertex_buffer: std::mem::MaybeUninit::zeroed().assume_init(),
                uniform_buffer: std::mem::MaybeUninit::zeroed().assume_init(),
                uniform_bind_group: std::mem::MaybeUninit::zeroed().assume_init(),
                msaa_texture: std::mem::MaybeUninit::zeroed().assume_init(),
                msaa_view: std::mem::MaybeUninit::zeroed().assume_init(),
                template_vertices: 0,
                cached_vertices: Vec::new(),
                cached_uniforms: ChartUniforms::new(),
                cached_candle_count: 0,
                cached_zoom_level: 1.0,
                cached_hash: 0,
                cached_data_revision: 0,
                cached_line_visibility: LineVisibility::default(),
                zoom_level: 1.0,
                pan_offset: 0.0,
                render_time_log: VecDeque::new(),
                line_visibility: LineVisibility::default(),
            }
        }
    }

    #[test]
    fn toggles_visibility() {
        let mut r = dummy_renderer();
        assert!(r.line_visibility.sma_20);
        r.toggle_line_visibility("sma20");
        assert!(!r.line_visibility.sma_20);
    }

    #[test]
    fn visibility_signal_updates() {
        use crate::app::global_line_visibility;

        global_line_visibility().set(LineVisibility::default());
        let mut r = dummy_renderer();
        r.toggle_line_visibility("sma20");
        assert!(!global_line_visibility().get_untracked().sma_20);
        r.toggle_line_visibility("sma20");
        assert!(global_line_visibility().get_untracked().sma_20);
    }

    #[test]
    fn legend_click_detection() {
        let r = dummy_renderer();
        assert_eq!(r.check_legend_checkbox_click(15.0, 15.0), Some("sma20".to_string()));
        assert_eq!(r.check_legend_checkbox_click(100.0, 100.0), None);
    }

    #[test]
    fn render_time_ring_buffer() {
        let mut r = dummy_renderer();
        for i in 0..65 {
            r.render_time_log.push_back(i as f64);
            if r.render_time_log.len() > 60 {
                r.render_time_log.pop_front();
            }
        }
        assert_eq!(r.render_time_log.len(), 60);
        assert_eq!(r.render_time_log.front().copied(), Some(5.0));
    }

    #[test]
    fn cache_generation_advances_on_geometry_upload() {
        let mut r = dummy_renderer();
        let verts = vec![CandleVertex::body_vertex(0.0, 0.0, true)];
        let inst = vec![CandleInstance {
            x: 0.0,
            width: 0.1,
            body_top: 0.5,
            body_bottom: 0.0,
            high: 0.6,
            low: -0.1,
            bullish: 1.0,
            _padding: 0.0,
        }];
        let uniforms = ChartUniforms::default();
        assert!(r.update_cached_geometry(verts.clone(), inst.clone(), uniforms));
        let cached = r.cached_hash;
        assert!(r.update_cached_geometry(verts, inst, ChartUniforms::default()));
        assert_eq!(r.cached_hash, cached.wrapping_add(1));
    }

    #[test]
    fn instance_count_matches_instances() {
        let mut r = dummy_renderer();
        let verts = vec![CandleVertex::body_vertex(0.0, 0.0, true)];
        let inst = vec![
            CandleInstance {
                x: 0.0,
                width: 0.1,
                body_top: 0.5,
                body_bottom: 0.0,
                high: 0.6,
                low: -0.1,
                bullish: 1.0,
                _padding: 0.0,
            },
            CandleInstance {
                x: 0.2,
                width: 0.1,
                body_top: 0.4,
                body_bottom: -0.1,
                high: 0.5,
                low: -0.2,
                bullish: 0.0,
                _padding: 0.0,
            },
        ];
        assert!(r.update_cached_geometry(verts, inst.clone(), ChartUniforms::default()));
    }

    #[test]
    fn geometry_updates_on_data_change() {
        use crate::domain::chart::{Chart, value_objects::ChartType};
        use crate::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};

        let mut chart = Chart::new("t".to_string(), ChartType::Candlestick, 10);
        chart.add_candle(Candle::new(
            Timestamp::from_millis(0),
            OHLCV::new(
                Price::from(1.0),
                Price::from(1.5),
                Price::from(0.5),
                Price::from(1.2),
                Volume::from(1.0),
            ),
        ));
        chart.add_candle(Candle::new(
            Timestamp::from_millis(60_000),
            OHLCV::new(
                Price::from(1.2),
                Price::from(1.7),
                Price::from(0.8),
                Price::from(1.4),
                Volume::from(1.0),
            ),
        ));

        let mut r = dummy_renderer();
        let (inst, verts, uni) = r.create_geometry(&chart);
        r.update_cached_geometry(verts, inst, uni);
        r.cached_data_revision = WebGpuRenderer::data_revision(&chart);
        let old = r.cached_hash;

        chart.add_candle(Candle::new(
            Timestamp::from_millis(60_000),
            OHLCV::new(
                Price::from(1.2),
                Price::from(1.9),
                Price::from(0.8),
                Price::from(1.6),
                Volume::from(1.0),
            ),
        ));

        assert_eq!(chart.get_candle_count(), 2);
        let new_revision = WebGpuRenderer::data_revision(&chart);
        assert_ne!(new_revision, r.cached_data_revision);
        let (inst2, verts2, uni2) = r.create_geometry(&chart);
        assert!(r.update_cached_geometry(verts2, inst2, uni2));
        r.cached_data_revision = new_revision;
        assert_ne!(r.cached_hash, old);
    }

    #[test]
    fn line_visibility_toggle_updates_geometry() {
        use crate::domain::chart::{Chart, value_objects::ChartType};
        use crate::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};

        let mut chart = Chart::new("t".to_string(), ChartType::Candlestick, 10);
        chart.add_candle(Candle::new(
            Timestamp::from_millis(0),
            OHLCV::new(
                Price::from(1.0),
                Price::from(1.5),
                Price::from(0.5),
                Price::from(1.2),
                Volume::from(1.0),
            ),
        ));
        chart.add_candle(Candle::new(
            Timestamp::from_millis(60_000),
            OHLCV::new(
                Price::from(1.2),
                Price::from(1.7),
                Price::from(0.8),
                Price::from(1.4),
                Volume::from(1.0),
            ),
        ));

        let mut r = dummy_renderer();
        let (inst, verts, uni) = r.create_geometry(&chart);
        r.update_cached_geometry(verts.clone(), inst.clone(), uni);
        r.cached_candle_count = chart.get_candle_count();
        r.cached_zoom_level = r.zoom_level;
        r.cached_data_revision = WebGpuRenderer::data_revision(&chart);
        let cached = r.cached_hash;

        r.toggle_line_visibility("sma20");
        let _ = r.render(&chart);
        assert_ne!(r.cached_hash, cached);
    }
}
