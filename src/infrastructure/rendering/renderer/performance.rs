use super::*;

impl WebGpuRenderer {
    /// Measure completed frames, including CPU submission and GPU queue latency.
    /// This is serialized throughput, not display refresh rate or a GPU timestamp.
    pub async fn measure_fps(&mut self, chart: &Chart, num_frames: u32) -> Result<f64, JsValue> {
        if num_frames == 0 || self.gpu.is_none() || chart.get_candle_count() == 0 {
            return Err(JsValue::from_str("benchmark needs frames, candles and a real GPU"));
        }
        let error_scope = self.gpu().device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = self.measure_completed_frames(chart, num_frames).await;
        if let Some(error) = error_scope.pop().await {
            return Err(JsValue::from_str(&format!("benchmark GPU validation: {error}")));
        }
        result
    }

    async fn measure_completed_frames(
        &mut self,
        chart: &Chart,
        num_frames: u32,
    ) -> Result<f64, JsValue> {
        let window = web_sys::window().expect("no window");
        let performance = window.performance().expect("performance not available");
        // A mapped readback after a same-queue buffer clear is a completion fence:
        // the mapping resolves only after that submission and prior render work.
        let fence = self.gpu().device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Benchmark completion fence"),
            size: 4,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let start = performance.now();
        for _ in 0..num_frames {
            let before = self.submitted_frames;
            self.render(chart)?;
            if self.submitted_frames == before {
                return Err(JsValue::from_str("benchmark submitted no chart geometry"));
            }
            let mut encoder = self.gpu().device.create_command_encoder(&Default::default());
            encoder.clear_buffer(&fence, 0, None);
            self.gpu().queue.submit([encoder.finish()]);
            let (done, completed) = futures::channel::oneshot::channel();
            fence.slice(..).map_async(wgpu::MapMode::Read, move |result| {
                let _ = done.send(result);
            });
            completed
                .await
                .map_err(|_| JsValue::from_str("GPU completion callback lost"))?
                .map_err(|_| JsValue::from_str("GPU completion readback failed"))?;
            fence.unmap();
        }
        let end = performance.now();
        let elapsed = (end - start) / 1000.0;
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return Err(JsValue::from_str("invalid benchmark elapsed time"));
        }
        Ok(num_frames as f64 / elapsed)
    }
}
