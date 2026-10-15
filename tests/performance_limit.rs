#![cfg(feature = "performance")]
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
use leptos::SignalSet;
use price_chart_wasm::domain::market_data::TimeInterval;
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
    let candles = (0..count)
        .map(|i| {
            let ts = Timestamp::from_millis(i as u64);
            let base = 10_000.0 + (i % 10_000) as f64;
            Candle::new(
                ts,
                OHLCV::new(
                    Price::from(base),
                    Price::from(base + 10.0),
                    Price::from(base - 10.0),
                    Price::from(base + 5.0),
                    Volume::from(1.0),
                ),
            )
        })
        .collect();
    chart.set_base_series(candles);
    chart
}

#[wasm_bindgen_test(async)]
async fn completed_frame_throughput() {
    price_chart_wasm::app::current_interval().set(TimeInterval::TwoSeconds);
    assert!(WebGpuRenderer::is_webgpu_supported().await, "WebGPU is required");
    setup_canvas("perf-canvas", 800, 600);
    let mut renderer =
        WebGpuRenderer::new("perf-canvas", 800, 600).await.expect("benchmark GPU initialization");
    let counts = [1_000usize, 50_000, 1_000_000];
    for &count in &counts {
        let chart = sample_chart(count);
        renderer.measure_fps(&chart, 10).await.expect("warmup frames");
        for sample in 0..5 {
            let fps = renderer.measure_fps(&chart, 30).await.expect("measured frames");
            assert!(fps.is_finite() && fps > 0.0);
            // Structured output is consumed by parse_perf_log.py. Each dataset
            // retains its own samples; averaging different workloads hides regressions.
            console_log!(
                "PERF_SAMPLE {}",
                serde_json::json!({"candles": count, "sample": sample, "frames": 30, "fps": fps})
            );
        }
    }
}

#[wasm_bindgen_test(async)]
async fn invalid_benchmark_inputs_fail() {
    let mut renderer = price_chart_wasm::infrastructure::rendering::renderer::dummy_renderer();
    assert!(renderer.measure_fps(&sample_chart(10), 30).await.is_err());
    setup_canvas("invalid-perf-canvas", 800, 600);
    let mut renderer = WebGpuRenderer::new("invalid-perf-canvas", 800, 600).await.unwrap();
    assert!(renderer.measure_fps(&sample_chart(10), 0).await.is_err());
    assert!(renderer.measure_fps(&sample_chart(0), 30).await.is_err());
    price_chart_wasm::app::current_interval().set(TimeInterval::TwoSeconds);
    let chart = sample_chart(10);
    renderer.measure_fps(&chart, 1).await.unwrap();
    // The base series is populated, but the selected derived series is empty.
    // A previous cached frame must not make a no-op render count as a new frame.
    price_chart_wasm::app::current_interval().set(TimeInterval::OneMinute);
    assert!(renderer.measure_fps(&chart, 1).await.is_err());
}
