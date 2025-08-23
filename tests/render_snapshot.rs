use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn red_canvas_snapshot() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .create_element("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    canvas.set_width(10);
    canvas.set_height(10);
    document.body().unwrap().append_child(&canvas).unwrap();

    let ctx = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();
    ctx.set_fill_style_str("#ff0000");
    ctx.fill_rect(0.0, 0.0, 10.0, 10.0);

    let data_url = canvas.to_data_url().unwrap();
    insta::assert_snapshot!(data_url);
}
