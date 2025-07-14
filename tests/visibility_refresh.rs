use price_chart_wasm::domain::{
    chart::{Chart, value_objects::ChartType},
    market_data::{Candle, OHLCV, Price, Timestamp, Volume},
};
use price_chart_wasm::infrastructure::rendering::renderer::dummy_renderer;
use wasm_bindgen_test::*;

fn sample_chart() -> Chart {
    let mut chart = Chart::new("vis".to_string(), ChartType::Candlestick, 100);
    for i in 0..30u64 {
        chart.add_candle(Candle::new(
            Timestamp::from_millis(i * 60_000),
            OHLCV::new(
                Price::from(100.0 + i as f64),
                Price::from(101.0 + i as f64),
                Price::from(99.0 + i as f64),
                Price::from(100.0 + i as f64),
                Volume::from(1.0),
            ),
        ));
    }
    chart
}

#[wasm_bindgen_test]
fn visibility_refreshes_cached_geometry() {
    let chart = sample_chart();
    let mut renderer = dummy_renderer();
    renderer.cache_geometry_for_test(&chart);
    let initial = renderer.cached_hash_for_test();

    renderer.toggle_line_visibility("sma20");
    let _ = renderer.render(&chart);

    assert_ne!(renderer.cached_hash_for_test(), initial);
}
