use price_chart_wasm::infrastructure::rendering::renderer::WebGpuRenderer;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

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
    let renderer = match WebGpuRenderer::new("mem-canvas", 10, 10).await {
        Ok(r) => r,
        Err(e) => {
            web_sys::console::log_1(&format!("Skipping test: {e:?}").into());
            return;
        }
    };
    let stats = renderer.log_gpu_memory_usage();
    assert!(!stats.is_empty());
}
