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

    // PNG encoders can produce different byte streams for identical pixels.
    // Assert the rendered RGBA value instead of an implementation-specific
    // compressed representation.
    let pixel = ctx.get_image_data(0.0, 0.0, 1.0, 1.0).unwrap().data().0;
    assert_eq!(&pixel[..4], &[255, 0, 0, 255]);
}
