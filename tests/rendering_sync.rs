use price_chart_wasm::domain::{
    chart::{Chart, value_objects::ChartType},
    market_data::{Candle, OHLCV, Price, Timestamp, Volume},
};
use price_chart_wasm::infrastructure::rendering::renderer::dummy_renderer;
use wasm_bindgen_test::*;
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn sample_chart() -> Chart {
    let mut chart = Chart::new("sync".into(), ChartType::Candlestick, 10);
    let c1 = Candle::new(
        Timestamp::from_millis(0),
        OHLCV::new(
            Price::from(100.0),
            Price::from(110.0),
            Price::from(95.0),
            Price::from(105.0),
            Volume::from(1.0),
        ),
    );
    let c2 = Candle::new(
        Timestamp::from_millis(60_000),
        OHLCV::new(
            Price::from(105.0),
            Price::from(115.0),
            Price::from(95.0),
            Price::from(100.0),
            Volume::from(1.2),
        ),
    );
    chart.set_historical_data(vec![c1, c2]);
    chart
}

#[wasm_bindgen_test]
fn body_positions_within_bounds() {
    let chart = sample_chart();
    let renderer = dummy_renderer();
    let (_instances, vertices, _uniforms) = renderer.create_geometry_for_test(&chart);

    let body: Vec<_> = vertices.iter().filter(|v| v.element_type == 0.0).collect();

    assert!(body.len() >= 36); // two candles * 18 vertices each

    let first_body_x = body[0].position_x;
    let second_body_x = body[18].position_x;

    assert_ne!(first_body_x, second_body_x);
    for &x in &[first_body_x, second_body_x] {
        assert!((-1.0..=1.0).contains(&x));
    }
}
