use js_sys::Date;
use price_chart_wasm::time_utils::format_time_label;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn format_time_label_utc() {
    let ts = 0u64; // 1970-01-01 00:00:00 UTC
    let date = Date::new(&JsValue::from_f64(ts as f64));
    assert_eq!(
        format_time_label(ts, 2.0),
        format!("{:02}:{:02}", date.get_utc_hours(), date.get_utc_minutes())
    );
    assert_eq!(
        format_time_label(ts, 1.0),
        format!("{:02}.{:02}", date.get_utc_date(), date.get_utc_month() + 1)
    );
    assert_eq!(
        format_time_label(ts, 0.5),
        format!("{:02}.{}", date.get_utc_month() + 1, date.get_utc_full_year())
    );
}
