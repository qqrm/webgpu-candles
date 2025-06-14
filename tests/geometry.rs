use insta::{assert_json_snapshot, with_settings};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use price_chart_wasm::infrastructure::rendering::gpu_structures::CandleGeometry;
use wasm_bindgen_test::*;

fn sample_candles() -> Vec<Candle> {
    vec![
        Candle::new(
            Timestamp::from_millis(0),
            OHLCV::new(
                Price::from(100.0),
                Price::from(110.0),
                Price::from(95.0),
                Price::from(105.0),
                Volume::from(1.0),
            ),
        ),
        Candle::new(
            Timestamp::from_millis(60000),
            OHLCV::new(
                Price::from(105.0),
                Price::from(115.0),
                Price::from(100.0),
                Price::from(108.0),
                Volume::from(1.5),
            ),
        ),
    ]
}

#[wasm_bindgen_test]
fn candle_geometry_snapshot() {
    let candles = sample_candles();
    let min_price = candles.iter().map(|c| c.ohlcv.low.value()).fold(f64::INFINITY, f64::min);
    let max_price = candles.iter().map(|c| c.ohlcv.high.value()).fold(f64::NEG_INFINITY, f64::max);
    let price_range = max_price - min_price;
    let normalize = |p: f64| ((p - min_price) / price_range * 2.0 - 1.0) as f32;

    let width = 0.1f32;
    let mut result = Vec::new();
    for (i, c) in candles.iter().enumerate() {
        let x = -1.0 + (i as f32 + 0.5) * width * 1.5;
        let verts = CandleGeometry::create_candle_vertices(
            c.timestamp.value() as f64,
            c.ohlcv.open.value() as f32,
            c.ohlcv.high.value() as f32,
            c.ohlcv.low.value() as f32,
            c.ohlcv.close.value() as f32,
            x,
            normalize(c.ohlcv.open.value()),
            normalize(c.ohlcv.high.value()),
            normalize(c.ohlcv.low.value()),
            normalize(c.ohlcv.close.value()),
            width,
        );
        result.extend(
            verts.into_iter().map(|v| [v.position_x, v.position_y, v.element_type, v.color_type]),
        );
    }

    with_settings!({snapshot_path => "tests/fixtures"}, {
        assert_json_snapshot!("candle_vertices", result);
    });
}

#[wasm_bindgen_test]
fn candle_color_logic() {
    let bullish = CandleGeometry::create_candle_vertices(
        0.0, 1.0, 1.2, 0.8, 1.1, 0.0, 0.0, 0.2, -0.2, 0.1, 0.2,
    );
    assert!((bullish[0].color_type - 1.0).abs() < f32::EPSILON);

    let bearish = CandleGeometry::create_candle_vertices(
        0.0, 1.1, 1.2, 0.9, 1.0, 0.0, 0.1, 0.2, -0.2, 0.0, 0.2,
    );
    assert!((bearish[0].color_type - 0.0).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn corner_segment_vertex_count() {
    let narrow = CandleGeometry::create_candle_vertices(
        0.0, 1.0, 1.1, 0.9, 1.05, 0.0, 0.0, 0.3, -0.3, 0.2, 0.02,
    );

    let wide = CandleGeometry::create_candle_vertices(
        0.0, 1.0, 1.1, 0.9, 1.05, 0.0, 0.0, 0.3, -0.3, 0.2, 0.05,
    );

    assert_eq!(narrow.len(), 114);
    assert_eq!(wide.len(), 186);
}

#[wasm_bindgen_test]
fn corner_radius_ratio() {
    let width = 0.1f32;
    let x = 0.0f32;
    let verts = CandleGeometry::create_candle_vertices(
        0.0, 1.0, 1.1, 0.9, 1.05, x, 0.0, 0.1, -0.1, 0.05, width,
    );

    let corner = width * 0.15;
    let expected_x = x - width * 0.5 + corner;
    assert!((verts[0].position_x - expected_x).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn very_low_candle_no_rounding() {
    let low = CandleGeometry::create_candle_vertices(
        0.0, 1.0, 1.05, 0.95, 1.0, 0.0, 0.0, 0.05, -0.05, 0.0, 0.05,
    );
    assert_eq!(low.len(), 18);
}
