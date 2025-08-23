#![cfg(feature = "render")]
use price_chart_wasm::domain::{
    chart::{Chart, ChartType},
    market_data::{Candle, OHLCV, Price, Timestamp, Volume},
};
use price_chart_wasm::infrastructure::rendering::renderer::WebGpuRenderer;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

fn setup_canvas(id: &str, width: u32, height: u32) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    canvas.set_id(id);
    canvas.set_width(width);
    canvas.set_height(height);
    document.body().unwrap().append_child(&canvas).unwrap();
}

fn sample_chart(count: usize) -> Chart {
    let mut chart = Chart::new("perf".to_string(), ChartType::Candlestick, count + 10);
    for i in 0..count {
        let ts = Timestamp::from_millis(i as u64);
        let base = 10000.0 + i as f64;
        let ohlcv = OHLCV::new(
            Price::from(base),
            Price::from(base + 10.0),
            Price::from(base - 10.0),
            Price::from(base + 5.0),
            Volume::from(1.0),
        );
        chart.add_candle(Candle::new(ts, ohlcv));
    }
    chart
}

#[wasm_bindgen_test(async)]
async fn fps_degradation_logging() {
    if !WebGpuRenderer::is_webgpu_supported().await {
        web_sys::console::log_1(&"Skipping test: WebGPU not supported".into());
        return;
    }
    setup_canvas("perf-canvas", 800, 600);
    let mut renderer = match WebGpuRenderer::new("perf-canvas", 800, 600).await {
        Ok(r) => r,
        Err(e) => {
            web_sys::console::log_1(&format!("Skipping test: {e:?}").into());
            return;
        }
    };

    let counts = [1000usize, 5000, 10000, 20000, 50000];
    for &count in &counts {
        let chart = sample_chart(count);
        let fps = renderer.measure_fps(&chart, 30);
        if fps < 30.0 {
            web_sys::console::log_1(&format!("⚠ {count} candles: {fps:.2} FPS").into());
        } else {
            web_sys::console::log_1(&format!("{count} candles: {fps:.2} FPS").into());
        }
    }
}
