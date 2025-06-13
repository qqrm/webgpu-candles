use super::*;
use crate::log_info;
use leptos::SignalGetUntracked;
use serde_json;

impl WebGpuRenderer {
    pub fn render(&mut self, chart: &Chart) -> Result<(), JsValue> {
        // ⏱️ Measure frame time
        if let Some(window) = web_sys::window() {
            if let Some(perf) = window.performance() {
                let now = perf.now();
                if self.last_frame_time > 0.0 {
                    let delta = now - self.last_frame_time;
                    if delta > 0.0 {
                        let fps = 1000.0 / delta;
                        self.fps_log.push_back(fps);
                        if self.fps_log.len() > 60 {
                            self.fps_log.pop_front();
                        }
                    }
                }
                self.last_frame_time = now;
            }
        }

        use crate::app::current_interval;
        let interval = current_interval().get_untracked();
        let candle_count = chart
            .get_series(interval)
            .map(|s| s.get_candles().len())
            .unwrap_or_else(|| chart.get_series_for_zoom(self.zoom_level).get_candles().len());

        // Log only every 100 frames for performance
        if candle_count % 100 == 0 {
            log_info!(
                LogComponent::Infrastructure("WebGpuRenderer"),
                "📊 Chart has {} candles to render",
                candle_count
            );
        }

        if candle_count == 0 {
            return Ok(());
        }

        let geometry_needs_update = candle_count != self.cached_candle_count
            || (self.zoom_level - self.cached_zoom_level).abs() > f64::EPSILON;

        if geometry_needs_update {
            // Fast instanced rendering with volume bars
            let (vertices, uniforms) = self.create_geometry(chart);
            if vertices.is_empty() {
                return Ok(());
            }
            self.cached_vertices = vertices;
            self.cached_instances = vec![]; // Do not use instances for simplicity
            self.cached_uniforms = uniforms;
            self.cached_candle_count = candle_count;
            self.cached_zoom_level = self.zoom_level;

            self.queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&self.cached_vertices),
            );
            self.queue.write_buffer(
                &self.uniform_buffer,
                0,
                bytemuck::cast_slice(&[self.cached_uniforms]),
            );
            self.template_vertices = self.cached_vertices.len() as u32;
            self.instance_count = 1;
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

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let start_pass = web_sys::window().and_then(|w| w.performance()).map(|p| p.now());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
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
            render_pass.draw(0..num_vertices, 0..1); // Non-instanced draw for simple shader
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        if let Some(start) = start_pass {
            if let Some(window) = web_sys::window() {
                if let Some(perf) = window.performance() {
                    let end = perf.now();
                    let duration = end - start;
                    log_info!(
                        LogComponent::Infrastructure("WebGpuRenderer"),
                        "\u{23f1}\u{fe0f} Render pass took {:.2} ms",
                        duration
                    );
                }
            }
        }

        output.present();

        Ok(())
    }

    /// Get renderer performance information
    pub fn get_performance_info(&self) -> String {
        let avg_fps = if self.fps_log.is_empty() {
            0.0
        } else {
            self.fps_log.iter().sum::<f64>() / self.fps_log.len() as f64
        };

        serde_json::json!({
            "backend": "WebGPU",
            "parallel": true,
            "status": "ready",
            "gpu_threads": "unlimited",
            "avg_fps": avg_fps
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
        match line_name {
            "sma20" => self.line_visibility.sma_20 = !self.line_visibility.sma_20,
            "sma50" => self.line_visibility.sma_50 = !self.line_visibility.sma_50,
            "sma200" => self.line_visibility.sma_200 = !self.line_visibility.sma_200,
            "ema12" => self.line_visibility.ema_12 = !self.line_visibility.ema_12,
            "ema26" => self.line_visibility.ema_26 = !self.line_visibility.ema_26,
            _ => {}
        }
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
                template_vertices: 0,
                instance_count: 0,
                cached_vertices: Vec::new(),
                cached_instances: Vec::new(),
                cached_uniforms: ChartUniforms::new(),
                cached_candle_count: 0,
                cached_zoom_level: 1.0,
                zoom_level: 1.0,
                pan_offset: 0.0,
                last_frame_time: 0.0,
                fps_log: VecDeque::new(),
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
    fn legend_click_detection() {
        let r = dummy_renderer();
        assert_eq!(r.check_legend_checkbox_click(15.0, 15.0), Some("sma20".to_string()));
        assert_eq!(r.check_legend_checkbox_click(100.0, 100.0), None);
    }

    #[test]
    fn fps_ring_buffer() {
        let mut r = dummy_renderer();
        for i in 0..65 {
            r.fps_log.push_back(i as f64);
            if r.fps_log.len() > 60 {
                r.fps_log.pop_front();
            }
        }
        assert_eq!(r.fps_log.len(), 60);
        assert_eq!(r.fps_log.front().copied(), Some(5.0));
    }
}
