use price_chart_wasm::domain::chart::Chart;
use price_chart_wasm::domain::chart::value_objects::ChartType;
use price_chart_wasm::domain::chart::value_objects::Viewport;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn horizontal_pan_moves_viewport() {
    let mut chart = Chart {
        id: "test".to_string(),
        chart_type: ChartType::Candlestick,
        series: Default::default(),
        viewport: Viewport {
            start_time: 0.0,
            end_time: 100.0,
            min_price: 0.0,
            max_price: 100.0,
            width: 800,
            height: 600,
        },
        indicators: Vec::new(),
        ichimoku: Default::default(),
    };
    chart.pan(0.1, 0.0);
    assert!((chart.viewport.start_time - 10.0).abs() < 1e-6);
    assert!((chart.viewport.end_time - 110.0).abs() < 1e-6);
}
