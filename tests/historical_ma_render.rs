#![cfg(feature = "render")]
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use price_chart_wasm::infrastructure::rendering::renderer::dummy_renderer;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
#[test]
fn historical_sma20_rendered() {
    fn make_candle(i: u64) -> Candle {
        let base = 100.0 + i as f64;
        Candle::new(
            Timestamp::from_millis(i * 60_000),
            OHLCV::new(
                Price::from(base),
                Price::from(base + 1.0),
                Price::from(base - 1.0),
                Price::from(base),
                Volume::from(1.0),
            ),
        )
    }

    let candles: Vec<Candle> = (0..30).map(make_candle).collect();

    let mut chart = Chart::new("hist-ma".to_string(), ChartType::Candlestick, 100);
    chart.set_historical_data(candles);

    let renderer = dummy_renderer();
    let (_, verts, _) = renderer.create_geometry_for_test(&chart);

    let sma20_vertices: Vec<_> =
        verts.iter().filter(|v| (v.color_type - 2.0).abs() < f32::EPSILON).collect();

    assert!(!sma20_vertices.is_empty());
}
