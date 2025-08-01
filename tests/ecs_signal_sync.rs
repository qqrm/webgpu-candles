use leptos::*;
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Symbol, Timestamp, Volume};
use price_chart_wasm::global_state::{
    ecs_world, ensure_chart, push_realtime_candle, set_chart_in_ecs,
};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

#[test]
fn push_candle_syncs_signal() {
    ecs_world().lock().unwrap().world = hecs::World::new();
    let symbol = Symbol::from("SYNC");
    let chart = ensure_chart(&symbol);
    let candle = Candle::new(
        Timestamp::from_millis(0),
        OHLCV::new(
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Volume::from(1.0),
        ),
    );
    push_realtime_candle(candle);
    assert_eq!(chart.with_untracked(|c| c.get_candle_count()), 1);
}

#[test]
fn set_chart_updates_signal() {
    ecs_world().lock().unwrap().world = hecs::World::new();
    let symbol = Symbol::from("SYNC2");
    let chart_signal = ensure_chart(&symbol);
    let mut chart = chart_signal.with_untracked(|c| c.clone());
    let candle = Candle::new(
        Timestamp::from_millis(0),
        OHLCV::new(
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Price::from(1.0),
            Volume::from(1.0),
        ),
    );
    chart.add_candle(candle);
    set_chart_in_ecs(&symbol, chart);
    assert_eq!(chart_signal.with_untracked(|c| c.get_candle_count()), 1);
}
