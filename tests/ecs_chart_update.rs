#![cfg(feature = "render")]
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

#[test]
fn set_chart_updates_only_target() {
    ecs_world().lock().unwrap().world = hecs::World::new();
    let sym_a = Symbol::from("A");
    let sym_b = Symbol::from("B");
    let chart_a = Chart::new(sym_a.value().to_string(), ChartType::Candlestick, 5);
    let mut chart_b = Chart::new(sym_b.value().to_string(), ChartType::Candlestick, 5);
    {
        let mut world = ecs_world().lock().unwrap();
        world.spawn_chart(chart_a);
        world.spawn_chart(chart_b.clone());
    }
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
    chart_b.add_candle(candle);
    set_chart_in_ecs(&sym_b, chart_b);
    let world_ref = ecs_world().lock().unwrap();
    let mut query = world_ref.world.query::<&ChartComponent>();
    let mut count_a = 0;
    let mut count_b = 0;
    for (_, comp) in query.iter() {
        let id = comp.0.with_untracked(|c| c.id.clone());
        let count = comp.0.with_untracked(|c| c.get_candle_count());
        if id == sym_a.value() {
            count_a = count;
        } else if id == sym_b.value() {
            count_b = count;
        }
    }
    assert_eq!(count_a, 0);
    assert_eq!(count_b, 1);
}
