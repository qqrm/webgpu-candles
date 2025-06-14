use js_sys::Date;
use wasm_bindgen::JsValue;

/// Format timestamp according to zoom level using UTC components.
///
/// - `zoom >= 2.0` -> `HH:MM`
/// - `1.0 <= zoom < 2.0` -> `DD.MM`
/// - `zoom < 1.0` -> `MM.YYYY`
pub fn format_time_label(timestamp: u64, zoom: f64) -> String {
    let date = Date::new(&JsValue::from_f64(timestamp as f64));
    if zoom >= 2.0 {
        format!("{:02}:{:02}", date.get_utc_hours(), date.get_utc_minutes())
    } else if zoom >= 1.0 {
        format!("{:02}.{:02}", date.get_utc_date(), date.get_utc_month() + 1)
    } else {
        format!("{:02}.{}", date.get_utc_month() + 1, date.get_utc_full_year())
    }
}

#[cfg(test)]
mod tests {
    use super::format_time_label;
    use js_sys::Date;
    use wasm_bindgen::JsValue;

    #[test]
    fn format_consistent_with_utc() {
        let ts = 0u64;
        let date = Date::new(&JsValue::from_f64(ts as f64));
        assert_eq!(
            format_time_label(ts, 2.0),
            format!("{:02}:{:02}", date.get_utc_hours(), date.get_utc_minutes())
        );
        assert_eq!(
            format_time_label(ts, 1.5),
            format!("{:02}.{:02}", date.get_utc_date(), date.get_utc_month() + 1)
        );
        assert_eq!(
            format_time_label(ts, 0.5),
            format!("{:02}.{}", date.get_utc_month() + 1, date.get_utc_full_year())
        );
    }
}
