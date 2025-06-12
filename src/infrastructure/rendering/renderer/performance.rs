use super::*;

impl WebGpuRenderer {
    /// Замерить средний FPS для заданного числа кадров
    pub fn measure_fps(&mut self, chart: &Chart, num_frames: u32) -> f64 {
        let window = web_sys::window().expect("no window");
        let performance = window.performance().expect("performance not available");
        let start = performance.now();
        for _ in 0..num_frames {
            let _ = self.render(chart);
        }
        let end = performance.now();
        let elapsed = (end - start) / 1000.0;
        if elapsed > 0.0 { num_frames as f64 / elapsed } else { 0.0 }
    }
}
