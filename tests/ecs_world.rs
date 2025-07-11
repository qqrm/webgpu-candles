use leptos::*;
use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use price_chart_wasm::ecs::EcsWorld;
use price_chart_wasm::ecs::components::CandleComponent;
use price_chart_wasm::ecs::components::{ChartComponent, ViewportComponent};

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
    assert_eq!(stored.0.with(|c| c.id.clone()), chart.id);
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
    assert_eq!(chart_comp.0.with(|c| c.get_candle_count()), 1);
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
        .with(|c| c.get_candle_count());
    let count_par = world_par
        .world
        .query::<&ChartComponent>()
        .iter()
        .next()
        .expect("chart component")
        .1
        .0
        .with(|c| c.get_candle_count());
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
