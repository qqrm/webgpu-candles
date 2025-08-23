#![cfg(feature = "render")]
use leptos::*;
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use price_chart_wasm::ecs::EcsWorld;
use price_chart_wasm::ecs::components::CandleComponent;
use price_chart_wasm::ecs::components::{ChartComponent, ViewportComponent};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
#[test]
fn world_starts_empty() {
    let world = EcsWorld::new();
    assert_eq!(world.world.len(), 0);
}

#[test]
fn spawn_chart_entity() {
    let mut world = EcsWorld::new();
    let chart = Chart::new("test".into(), ChartType::Candlestick, 100);
    let entity = world.spawn_chart(chart.clone());
    let stored = world.world.get::<&ChartComponent>(entity).expect("chart component exists");
    assert_eq!(stored.0.with_untracked(|c| c.id.clone()), chart.id);
}

#[test]
fn spawn_chart_adds_viewport_component() {
    let mut world = EcsWorld::new();
    let chart = Chart::new("test_vp".into(), ChartType::Candlestick, 50);
    let entity = world.spawn_chart(chart.clone());

    let vp = world.world.get::<&ViewportComponent>(entity).expect("viewport component exists");
    assert_eq!(vp.0, chart.viewport);
}

#[test]
fn candle_system_applies_candles() {
    let mut world = EcsWorld::new();
    let chart = Chart::new("test".into(), ChartType::Candlestick, 10);
    world.spawn_chart(chart.clone());

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
    world.world.spawn((CandleComponent(candle.clone()),));

    world.run_candle_system();

    let mut query = world.world.query::<&ChartComponent>();
    let chart_comp = query.iter().next().expect("chart component").1;
    assert_eq!(chart_comp.0.with_untracked(|c| c.get_candle_count()), 1);
    assert_eq!(world.world.len(), 1);
}

#[test]
fn candle_system_parallel_matches_sequential() {
    let mut world_seq = EcsWorld::new();
    let mut world_par = EcsWorld::new();
    let chart_seq = Chart::new("par".into(), ChartType::Candlestick, 10);
    let chart_par = chart_seq.clone();
    world_seq.spawn_chart(chart_seq);
    world_par.spawn_chart(chart_par);

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
    world_seq.world.spawn((CandleComponent(candle.clone()),));
    world_par.world.spawn((CandleComponent(candle.clone()),));

    world_seq.run_candle_system();
    world_par.run_candle_system_parallel();

    let count_seq = world_seq
        .world
        .query::<&ChartComponent>()
        .iter()
        .next()
        .expect("chart component")
        .1
        .0
        .with_untracked(|c| c.get_candle_count());
    let count_par = world_par
        .world
        .query::<&ChartComponent>()
        .iter()
        .next()
        .expect("chart component")
        .1
        .0
        .with_untracked(|c| c.get_candle_count());
    assert_eq!(count_seq, count_par);
}

#[test]
fn viewport_component_updates() {
    let mut world = EcsWorld::new();
    let mut chart = Chart::new("test".into(), ChartType::Candlestick, 10);
    let entity = world.spawn_chart(chart.clone());

    chart.zoom(2.0, 0.5);
    chart.pan(0.1, 0.0);

    {
        let comp = world.world.get::<&mut ChartComponent>(entity).unwrap();
        comp.0.set(chart.clone());
    }

    world.run_viewport_system();

    let vp = world.world.get::<&ViewportComponent>(entity).unwrap();
    assert_eq!(vp.0, chart.viewport);
}
#[test]
fn candle_system_no_candles() {
    let mut world = EcsWorld::new();
    world.spawn_chart(Chart::new("noop".into(), ChartType::Candlestick, 10));
    world.run_candle_system();
    let count = world
        .world
        .query::<&ChartComponent>()
        .iter()
        .next()
        .unwrap()
        .1
        .0
        .with_untracked(|c| c.get_candle_count());
    assert_eq!(count, 0);
    assert_eq!(world.world.len(), 1);
}

#[test]
fn candle_system_no_charts() {
    let mut world = EcsWorld::new();
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
    world.world.spawn((CandleComponent(candle),));
    world.run_candle_system();
    assert_eq!(world.world.len(), 0);
}

#[test]
fn viewport_system_multiple_charts() {
    let mut world = EcsWorld::new();
    let mut chart_a = Chart::new("A".into(), ChartType::Candlestick, 10);
    let mut chart_b = Chart::new("B".into(), ChartType::Candlestick, 10);
    let entity_a = world.spawn_chart(chart_a.clone());
    let entity_b = world.spawn_chart(chart_b.clone());

    chart_a.pan(0.1, 0.0);
    chart_b.zoom(2.0, 0.5);

    {
        let comp = world.world.get::<&mut ChartComponent>(entity_a).unwrap();
        comp.0.set(chart_a.clone());
    }
    {
        let comp = world.world.get::<&mut ChartComponent>(entity_b).unwrap();
        comp.0.set(chart_b.clone());
    }

    world.run_viewport_system();

    let vp_a = world.world.get::<&ViewportComponent>(entity_a).unwrap();
    assert_eq!(vp_a.0, chart_a.viewport);
    let vp_b = world.world.get::<&ViewportComponent>(entity_b).unwrap();
    assert_eq!(vp_b.0, chart_b.viewport);
}
