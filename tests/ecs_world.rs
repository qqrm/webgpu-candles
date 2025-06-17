use price_chart_wasm::domain::chart::{Chart, value_objects::ChartType};
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Timestamp, Volume};
use price_chart_wasm::ecs::EcsWorld;
use price_chart_wasm::ecs::components::CandleComponent;
use price_chart_wasm::ecs::components::ChartComponent;

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
    assert_eq!(stored.0.id, chart.id);
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
    assert_eq!(chart_comp.0.get_candle_count(), 1);
    assert_eq!(world.world.len(), 1);
}
