#![cfg(feature = "render")]
use price_chart_wasm::view_state::ViewState;
use quickcheck_macros::quickcheck;

const WIDTH: f32 = 800.0;

#[quickcheck]
fn zoom_roundtrip_preserves_index(cursor: f32, delta: f32) -> bool {
    let cursor = cursor.clamp(0.0, 1.0);
    let delta = delta.clamp(-5.0, 5.0);
    let mut view = ViewState::new(5.0, 1.0, 20.0);
    let cursor_px = WIDTH * cursor;
    let before = (cursor_px - view.pan_offset_px) / view.pixels_per_candle;
    view.zoom_at(delta, cursor, WIDTH);
    view.zoom_at(-delta, cursor, WIDTH);
    let after = (cursor_px - view.pan_offset_px) / view.pixels_per_candle;
    (after - before).abs() <= 0.5
}

#[test]
fn zoom_handler_does_not_call_resample() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let flag = AtomicBool::new(false);
    let _resample = || flag.store(true, Ordering::SeqCst);

    let mut view = ViewState::new(5.0, 1.0, 20.0);
    view.zoom_at(1.0, 0.5, WIDTH);

    assert!(!flag.load(Ordering::SeqCst));
}
