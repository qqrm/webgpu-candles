use leptos::*;
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Symbol, Timestamp, Volume};
use price_chart_wasm::ecs::components::ChartComponent;
use price_chart_wasm::global_state::{ecs_world, set_chart_in_ecs};

#[test]
fn set_chart_spawns_when_missing() {
    ecs_world().lock().unwrap().world = hecs::World::new();
    let symbol = Symbol::from("TEST");
    let chart = Chart::new(symbol.value().to_string(), ChartType::Candlestick, 10);
    set_chart_in_ecs(&symbol, chart.clone());
    let world_ref = ecs_world().lock().unwrap();
    let mut query = world_ref.world.query::<&ChartComponent>();
    assert_eq!(query.iter().count(), 1);
    let stored = query.iter().next().unwrap().1;
    assert_eq!(stored.0.with_untracked(|c| c.id.clone()), chart.id);
}

#[test]
fn set_chart_replaces_existing() {
    ecs_world().lock().unwrap().world = hecs::World::new();
    let symbol = Symbol::from("TEST2");
    let mut chart = Chart::new(symbol.value().to_string(), ChartType::Candlestick, 10);
    ecs_world().lock().unwrap().spawn_chart(chart.clone());
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
    set_chart_in_ecs(&symbol, chart.clone());
    let world_ref = ecs_world().lock().unwrap();
    let mut query = world_ref.world.query::<&ChartComponent>();
    let stored = query.iter().next().unwrap().1;
    assert_eq!(stored.0.with_untracked(|c| c.get_candle_count()), 1);
}
