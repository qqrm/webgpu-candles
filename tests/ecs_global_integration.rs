#![cfg(feature = "render")]
use leptos::prelude::*;
use price_chart_wasm::domain::market_data::{Candle, OHLCV, Price, Symbol, Timestamp, Volume};
use price_chart_wasm::ecs::components::ChartComponent;
use price_chart_wasm::global_state::{ecs_world, ensure_chart, push_realtime_candle};

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
#[wasm_bindgen_test::wasm_bindgen_test]
fn ensure_chart_spawns_entity() {
    let owner = leptos::prelude::Owner::new();
    owner.with(|| {
        ecs_world().lock().unwrap().world = hecs::World::new();
        let symbol = Symbol::from("TEST");
        ensure_chart(&symbol);
        let world_handle = ecs_world();
        let world_ref = world_handle.lock().unwrap();
        let mut query = world_ref.world.query::<(hecs::Entity, &ChartComponent)>();
        let count = query.iter().filter(|(_, c)| c.0.with(|ch| ch.id == symbol.value())).count();
        assert_eq!(count, 1);
    });
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn push_candle_updates_world() {
    let owner = leptos::prelude::Owner::new();
    owner.with(|| {
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
        let world_handle = ecs_world();
        let world_ref = world_handle.lock().unwrap();
        let mut query = world_ref.world.query::<(hecs::Entity, &ChartComponent)>();
        let chart_comp = query.iter().next().expect("chart component").1;
        assert_eq!(chart_comp.0.with_untracked(|c| c.get_candle_count()), 1);
    });
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn ensure_chart_returns_same_signal() {
    let owner = leptos::prelude::Owner::new();
    owner.with(|| {
        ecs_world().lock().unwrap().world = hecs::World::new();
        let symbol = Symbol::from("DUP");
        let first = ensure_chart(&symbol);
        let second = ensure_chart(&symbol);
        assert_eq!(first.with_untracked(|c| c.id.clone()), second.with_untracked(|c| c.id.clone()));
        let world_handle = ecs_world();
        let world_ref = world_handle.lock().unwrap();
        let mut query = world_ref.world.query::<(hecs::Entity, &ChartComponent)>();
        assert_eq!(query.iter().count(), 1);
    });
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn push_candle_without_charts() {
    let owner = leptos::prelude::Owner::new();
    owner.with(|| {
        ecs_world().lock().unwrap().world = hecs::World::new();
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
        let world_handle = ecs_world();
        let world_ref = world_handle.lock().unwrap();
        assert_eq!(world_ref.world.len(), 0);
    });
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn push_candle_updates_multiple_charts() {
    let owner = leptos::prelude::Owner::new();
    owner.with(|| {
        ecs_world().lock().unwrap().world = hecs::World::new();
        let sym_a = Symbol::from("AAA");
        let sym_b = Symbol::from("BBB");
        let chart_a = ensure_chart(&sym_a);
        let chart_b = ensure_chart(&sym_b);
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
        assert_eq!(chart_a.with_untracked(|c| c.get_candle_count()), 1);
        assert_eq!(chart_b.with_untracked(|c| c.get_candle_count()), 1);
    });
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn ensure_chart_creates_new_for_new_symbol() {
    let owner = leptos::prelude::Owner::new();
    owner.with(|| {
        ecs_world().lock().unwrap().world = hecs::World::new();
        let sym_a = Symbol::from("AAA");
        let sym_b = Symbol::from("BBB");
        ensure_chart(&sym_a);
        ensure_chart(&sym_b);
        let world_handle = ecs_world();
        let world_ref = world_handle.lock().unwrap();
        let count = world_ref.world.query::<(hecs::Entity, &ChartComponent)>().iter().count();
        assert_eq!(count, 2);
    });
}
