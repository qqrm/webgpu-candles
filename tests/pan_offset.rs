use price_chart_wasm::app::{HISTORY_PRELOAD_THRESHOLD, should_fetch_history, visible_range};

#[test]
fn pan_direction_and_history_activation() {
    let len = 200;
    let zoom = 1.0;
    let (_, visible) = visible_range(len, zoom, 0.0);
    let base_start = len - visible;
    let target_start = HISTORY_PRELOAD_THRESHOLD - 1;
    let pan = (target_start as isize - base_start as isize) as f64;
    let (start, _) = visible_range(len, zoom, pan);
    assert_eq!(start, target_start);
    assert!(should_fetch_history(start));
}
