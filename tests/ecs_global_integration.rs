use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Symbol, Timestamp, Volume};
use price_chart_wasm::ecs::components::ChartComponent;
use price_chart_wasm::global_state::{ecs_world, ensure_chart, push_realtime_candle};

#[test]
fn ensure_chart_spawns_entity() {
    ecs_world().lock().unwrap().world = hecs::World::new();
    let symbol = Symbol::from("TEST");
    ensure_chart(&symbol);
    let world_ref = ecs_world().lock().unwrap();
    let mut query = world_ref.world.query::<&ChartComponent>();
    let count = query.iter().filter(|(_, c)| c.0.id == symbol.value()).count();
    assert_eq!(count, 1);
}

#[test]
fn push_candle_updates_world() {
    ecs_world().lock().unwrap().world = hecs::World::new();
    let symbol = Symbol::from("TEST2");
    ensure_chart(&symbol);
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
    let world_ref = ecs_world().lock().unwrap();
    let mut query = world_ref.world.query::<&ChartComponent>();
    let chart_comp = query.iter().next().expect("chart component").1;
    assert_eq!(chart_comp.0.get_candle_count(), 1);
}
