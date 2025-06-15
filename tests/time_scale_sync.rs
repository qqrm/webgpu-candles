use price_chart_wasm::app::{visible_range, visible_range_by_time};
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, TimeInterval, Timestamp, Volume,
};
use price_chart_wasm::time_utils::format_time_label;
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn make_candle(i: u64) -> Candle {
    Candle::new(
        Timestamp::from_millis(i * 60_000),
        OHLCV::new(
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Volume::from(1.0),
        ),
    )
}

fn time_labels(chart: &Chart, zoom: f64) -> Vec<String> {
    let interval = TimeInterval::OneMinute;
    let candles = chart.get_series(interval).unwrap().get_candles();
    if candles.is_empty() {
        return vec![];
    }

    let candle_vec: Vec<_> = candles.iter().cloned().collect();
    let (start_idx, visible) = visible_range_by_time(&candle_vec, &chart.viewport, zoom);
    let base_start = candle_vec.len().saturating_sub(visible);
    let pan = start_idx as isize - base_start as isize;
    let (start_idx, visible) = visible_range(candle_vec.len(), zoom, pan as f64);

    let num_labels = 5;
    let mut labels = Vec::new();
    for i in 0..num_labels {
        let index = (i * visible) / (num_labels - 1);
        if let Some(candle) =
            candle_vec.iter().skip(start_idx).nth(index.min(visible.saturating_sub(1)))
        {
            let ts = candle.timestamp.value();
            let label = format_time_label(ts, zoom);
            labels.push(label);
        }
    }
    labels
}

#[wasm_bindgen_test]
fn time_scale_updates_on_zoom_and_pan() {
    let candles: Vec<Candle> = (0..120).map(make_candle).collect();
    let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 200);
    chart.set_historical_data(candles);

    let mut zoom = 1.0f64;

    let labels_before = time_labels(&chart, zoom);

    chart.zoom(2.0, 0.5);
    zoom *= 2.0;
    let labels_after_zoom = time_labels(&chart, zoom);
    assert_ne!(labels_before, labels_after_zoom);

    chart.pan(-0.1, 0.0);
    let labels_after_pan = time_labels(&chart, zoom);
    assert_ne!(labels_after_zoom, labels_after_pan);
    assert_eq!(labels_after_pan.len(), 5);
}
