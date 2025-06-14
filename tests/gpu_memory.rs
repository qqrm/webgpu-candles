use price_chart_wasm::infrastructure::rendering::renderer::WebGpuRenderer;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

fn setup_canvas(id: &str) {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    canvas.set_id(id);
    canvas.set_width(10);
    canvas.set_height(10);
    document.body().unwrap().append_child(&canvas).unwrap();
}

#[wasm_bindgen_test(async)]
async fn memory_usage_returns_string() {
    if !WebGpuRenderer::is_webgpu_supported().await {
        web_sys::console::log_1(&"Skipping test: WebGPU not supported".into());
        return;
    }
    setup_canvas("mem-canvas");
    let renderer = WebGpuRenderer::new("mem-canvas", 10, 10).await.unwrap();
    let stats = renderer.log_gpu_memory_usage();
    assert!(!stats.is_empty());
}
