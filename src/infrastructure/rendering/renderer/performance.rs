use super::*;

impl WebGpuRenderer {
    /// Measure completed frames, including CPU submission and GPU queue latency.
    /// This is serialized throughput, not display refresh rate or a GPU timestamp.
    pub async fn measure_fps(&mut self, chart: &Chart, num_frames: u32) -> Result<f64, JsValue> {
        if num_frames == 0 || self.gpu.is_none() || chart.get_candle_count() == 0 {
            return Err(JsValue::from_str("benchmark needs frames, candles and a real GPU"));
        }
        let window = web_sys::window().expect("no window");
        let performance = window.performance().expect("performance not available");
        let start = performance.now();
        for _ in 0..num_frames {
            self.render(chart)?;
            if self.cached_vertices.is_empty() {
                return Err(JsValue::from_str("benchmark submitted no chart geometry"));
            }
            let (done, completed) = futures::channel::oneshot::channel();
            self.gpu().queue.on_submitted_work_done(move || {
                let _ = done.send(());
            });
            completed.await.map_err(|_| JsValue::from_str("GPU completion callback lost"))?;
        }
        let end = performance.now();
        let elapsed = (end - start) / 1000.0;
        if !elapsed.is_finite() || elapsed <= 0.0 {
            return Err(JsValue::from_str("invalid benchmark elapsed time"));
        }
        Ok(num_frames as f64 / elapsed)
    }
}
