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
