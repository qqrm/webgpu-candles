use price_chart_wasm::domain::market_data::services::MarketAnalysisService;
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use price_chart_wasm::infrastructure::rendering::gpu_structures::{CandleGeometry, IndicatorType};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn current_price_line_vertices() {
    let verts = CandleGeometry::create_current_price_line(0.5, 0.2);
    assert_eq!(verts.len(), 6);
    assert!((verts[0].position_x + 1.0).abs() < f32::EPSILON);
    assert!((verts[0].position_y - 0.4).abs() < f32::EPSILON);
    assert!((verts[0].color_type - 7.0).abs() < f32::EPSILON);
    let last = verts.last().unwrap();
    assert!((last.position_x - 1.0).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn indicator_line_vertex_count() {
    let points = [(-1.0, 0.0), (0.0, 0.5), (1.0, 0.0)];
    let verts = CandleGeometry::create_indicator_line_vertices(&points, IndicatorType::SMA20, 0.1);
    assert_eq!(verts.len(), (points.len() - 1) * 6);
    assert!((verts[0].color_type - 2.0).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn ichimoku_cloud_vertices() {
    let span_a = [(-1.0, 0.6), (0.0, 0.7), (1.0, 0.6)];
    let span_b = [(-1.0, 0.4), (0.0, 0.3), (1.0, 0.4)];
    let verts = CandleGeometry::create_ichimoku_cloud(&span_a, &span_b, 0.05);
    let expected = (span_a.len() - 1) * 6 + (span_a.len() - 1) * 6 * 2;
    assert_eq!(verts.len(), expected);
    assert!((verts[0].element_type - 6.0).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn ichimoku_calculation() {
    let candles: Vec<Candle> = (0..5)
        .map(|i| {
            Candle::new(
                Timestamp::from_millis(i as u64),
                OHLCV::new(
                    Price::from(10.0 + i as f64),
                    Price::from(11.0 + i as f64),
                    Price::from(9.0 + i as f64),
                    Price::from(10.5 + i as f64),
                    Volume::from(1.0),
                ),
            )
        })
        .collect();
    let svc = MarketAnalysisService::new();
    let tenkan = svc.calculate_tenkan_sen(&candles, 3);
    assert_eq!(tenkan.len(), 3);
    assert!((tenkan[0].value() - 10.5).abs() < f64::EPSILON);
}

#[wasm_bindgen_test]
fn indicator_line_preserves_out_of_range_y() {
    let points = [(-0.5, -1.2), (0.0, 0.0), (0.5, 1.3)];
    let verts = CandleGeometry::create_indicator_line_vertices(&points, IndicatorType::SMA20, 0.1);
    assert_eq!(verts.len(), (points.len() - 1) * 6);
    let min_y = verts.iter().map(|v| v.position_y).fold(f32::INFINITY, f32::min);
    let max_y = verts.iter().map(|v| v.position_y).fold(f32::NEG_INFINITY, f32::max);
    assert!(min_y < -1.1);
    assert!(max_y > 1.25);
}
