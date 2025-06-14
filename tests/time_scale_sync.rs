use price_chart_wasm::app::visible_range;
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{
    Candle, OHLCV, Price, TimeInterval, Timestamp, Volume,
};
use wasm_bindgen_test::*;

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

fn generate_labels(chart: &Chart, zoom: f64, pan: f64) -> Vec<String> {
    let interval = TimeInterval::OneMinute;
    let candles = chart.get_series(interval).unwrap().get_candles();
    if candles.is_empty() {
        return vec![];
    }
    let (start_idx, visible) = visible_range(candles.len(), zoom, pan);
    let num_labels = 5;
    let mut labels = Vec::new();
    for i in 0..num_labels {
        let index = (i * visible) / (num_labels - 1);
        let idx = index.min(visible.saturating_sub(1));
        if let Some(candle) = candles.iter().skip(start_idx).nth(idx) {
            let ts = candle.timestamp.value() / 1000 / 60; // minutes
            let h = (ts / 60) % 24;
            let m = ts % 60;
            labels.push(format!("{:02}:{:02}", h, m));
        }
    }
    labels
}

#[wasm_bindgen_test]
fn time_scale_updates_with_zoom_and_pan() {
    let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 100);
    for i in 0..30 {
        chart.add_candle(make_candle(i as u64));
    }

    let initial = generate_labels(&chart, 1.0, 0.0);
    chart.zoom(2.0, 0.5);
    let zoomed = generate_labels(&chart, 2.0, 0.0);
    assert_ne!(initial, zoomed);
    assert_eq!(
        zoomed,
        vec![
            "00:05".to_string(),
            "00:11".to_string(),
            "00:17".to_string(),
            "00:23".to_string(),
            "00:29".to_string(),
        ]
    );

    let pan_offset = -2.0;
    let time_range = chart.viewport.time_range();
    let delta = (pan_offset * 60_000.0) / time_range;
    chart.pan(delta as f32, 0.0);
    let panned = generate_labels(&chart, 2.0, pan_offset);
    assert_ne!(zoomed, panned);
    assert_eq!(
        panned,
        vec![
            "00:03".to_string(),
            "00:09".to_string(),
            "00:15".to_string(),
            "00:21".to_string(),
            "00:27".to_string(),
        ]
    );
}
