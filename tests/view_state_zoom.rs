#![cfg(feature = "render")]
mod support;
use price_chart_wasm::view_state::ViewState;
use quickcheck::{QuickCheck, TestResult};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
const WIDTH: f32 = 800.0;

fn zoom_roundtrip_property(cursor: f32, delta: f32) -> TestResult {
    // This property covers finite UI coordinates/deltas, not invalid floating
    // point inputs. QuickCheck generates NaN too; clamp does not sanitize it.
    if !cursor.is_finite() || !delta.is_finite() {
        return TestResult::discard();
    }
    let cursor = cursor.clamp(0.0, 1.0);
    let delta = delta.clamp(-5.0, 5.0);
    let mut view = ViewState::new(5.0, 1.0, 20.0);
    let cursor_px = WIDTH * cursor;
    let before = (cursor_px - view.pan_offset_px) / view.pixels_per_candle;
    view.zoom_at(delta, cursor, WIDTH);
    view.zoom_at(-delta, cursor, WIDTH);
    let after = (cursor_px - view.pan_offset_px) / view.pixels_per_candle;
    TestResult::from_bool((after - before).abs() <= 0.5)
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn zoom_roundtrip_preserves_index() {
    support::with_app_context(|| {
        for cursor in [0.0, 0.5, 1.0] {
            for delta in [-5.0, -4.0, 0.0, 5.0] {
                assert!(!zoom_roundtrip_property(cursor, delta).is_failure());
            }
        }
        QuickCheck::new()
            .tests(256)
            .min_tests_passed(256)
            .max_tests(10_000)
            .quickcheck(zoom_roundtrip_property as fn(f32, f32) -> TestResult);
    });
}
