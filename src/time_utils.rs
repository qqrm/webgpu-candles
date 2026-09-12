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

/// Format a chart label from the actual visible time span.
pub fn format_time_label_for_span(timestamp: u64, span_ms: u64) -> String {
    let date = Date::new(&JsValue::from_f64(timestamp as f64));
    const TWO_DAYS_MS: u64 = 2 * 24 * 60 * 60 * 1000;
    const FOUR_MONTHS_MS: u64 = 120 * 24 * 60 * 60 * 1000;

    if span_ms <= TWO_DAYS_MS {
        format!("{:02}:{:02}", date.get_utc_hours(), date.get_utc_minutes())
    } else if span_ms <= FOUR_MONTHS_MS {
        format!("{:02}.{:02}", date.get_utc_date(), date.get_utc_month() + 1)
    } else {
        format!("{:02}.{}", date.get_utc_month() + 1, date.get_utc_full_year())
    }
}

/// Format a complete UTC timestamp for the chart tooltip.
pub fn format_timestamp_utc(timestamp: u64) -> String {
    let date = Date::new(&JsValue::from_f64(timestamp as f64));
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02} UTC",
        date.get_utc_full_year(),
        date.get_utc_month() + 1,
        date.get_utc_date(),
        date.get_utc_hours(),
        date.get_utc_minutes()
    )
}

#[cfg(test)]
mod tests {
    use super::{format_time_label, format_time_label_for_span, format_timestamp_utc};
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

    #[test]
    fn visible_span_selects_intraday_labels() {
        assert_eq!(format_time_label_for_span(0, 60 * 60 * 1000), "00:00");
        assert_eq!(format_timestamp_utc(0), "1970-01-01 00:00 UTC");
    }
}
