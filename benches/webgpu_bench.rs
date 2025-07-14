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
    let mut chart = Chart::new("bench".to_string(), ChartType::Candlestick, count + 10);
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
async fn benchmark_fps() {
    setup_canvas("bench-canvas", 800, 600);
    let mut renderer = WebGpuRenderer::new("bench-canvas", 800, 600).await.unwrap();

    for &count in &[100usize, 1000usize] {
        let chart = sample_chart(count);
        let fps = renderer.measure_fps(&chart, 30);
        web_sys::console::log_1(&format!("{count} candles: {fps:.2} FPS").into());
    }
}
