//! Lazily initialized global reactive signals.
//!
//! This module stores shared application state such as the current price and
//! UI flags. `OnceCell` is used to ensure the globals are created only on first
//! access.

use crate::app::TooltipData;
use crate::domain::{
    chart::{Chart, value_objects::ChartType},
    market_data::{Candle, Symbol, TimeInterval},
};
use crate::ecs::EcsWorld;
use futures::future::AbortHandle;
use leptos::*;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct Globals {
    pub current_price: RwSignal<f64>,
    pub candle_count: RwSignal<usize>,
    pub is_streaming: RwSignal<bool>,
    pub max_volume: RwSignal<f64>,
    pub loading_more: RwSignal<bool>,
    pub tooltip_data: RwSignal<Option<TooltipData>>,
    pub tooltip_visible: RwSignal<bool>,
    pub zoom_level: RwSignal<f64>,
    pub pan_offset: RwSignal<f64>,
    pub is_dragging: RwSignal<bool>,
    pub last_mouse_x: RwSignal<f64>,
    pub current_interval: RwSignal<TimeInterval>,
    pub current_symbol: RwSignal<Symbol>,
    pub charts: RwSignal<HashMap<Symbol, RwSignal<Chart>>>,
    pub stream_abort_handles: RwSignal<HashMap<Symbol, AbortHandle>>,
    pub line_visibility: RwSignal<crate::infrastructure::rendering::renderer::LineVisibility>,
}

// The `OnceCell` ensures this state is created at most once on demand.
static GLOBALS: OnceCell<Globals> = OnceCell::new();
static ECS_WORLD: OnceCell<Mutex<EcsWorld>> = OnceCell::new();

pub fn globals() -> &'static Globals {
    GLOBALS.get_or_init(|| Globals {
        current_price: create_rw_signal(0.0),
        candle_count: create_rw_signal(0),
        is_streaming: create_rw_signal(false),
        max_volume: create_rw_signal(0.0),
        loading_more: create_rw_signal(false),
        tooltip_data: create_rw_signal(None),
        tooltip_visible: create_rw_signal(false),
        zoom_level: create_rw_signal(0.32),
        pan_offset: create_rw_signal(0.0),
        is_dragging: create_rw_signal(false),
        last_mouse_x: create_rw_signal(0.0),
        current_interval: create_rw_signal(TimeInterval::OneMinute),
        current_symbol: create_rw_signal(Symbol::from("BTCUSDT")),
        charts: create_rw_signal(HashMap::new()),
        stream_abort_handles: create_rw_signal(HashMap::new()),
        line_visibility: create_rw_signal(
            crate::infrastructure::rendering::renderer::LineVisibility::default(),
        ),
    })
}

/// Access the global ECS world.
pub fn ecs_world() -> &'static Mutex<EcsWorld> {
    ECS_WORLD.get_or_init(|| Mutex::new(EcsWorld::new()))
}

pub fn ensure_chart(symbol: &Symbol) -> RwSignal<Chart> {
    let charts = &globals().charts;
    charts.update(|map| {
        map.entry(symbol.clone()).or_insert_with(|| {
            let chart = Chart::new(symbol.value().to_string(), ChartType::Candlestick, 1000);
            ecs_world().lock().unwrap().spawn_chart(chart.clone());
            create_rw_signal(chart)
        });
    });
    charts.with(|map| map.get(symbol).copied().unwrap())
}

pub fn global_charts() -> RwSignal<HashMap<Symbol, RwSignal<Chart>>> {
    globals().charts
}

pub fn stream_abort_handles() -> RwSignal<HashMap<Symbol, AbortHandle>> {
    globals().stream_abort_handles
}

/// Add a candle to the ECS world and process systems.
pub fn push_realtime_candle(candle: Candle) {
    use crate::ecs::components::CandleComponent;
    {
        let mut world = ecs_world().lock().unwrap();
        world.world.spawn((CandleComponent(candle),));
        world.run_candle_system();
    }
    sync_charts_from_ecs();
}

/// Replace or spawn a chart entity in the ECS world.
pub fn set_chart_in_ecs(symbol: &Symbol, chart: Chart) {
    use crate::ecs::components::ChartComponent;
    {
        let mut world = ecs_world().lock().unwrap();
        let mut found = false;
        for (_, comp) in world.world.query::<&mut ChartComponent>().iter() {
            if comp.0.id == symbol.value() {
                comp.0 = chart.clone();
                found = true;
                break;
            }
        }
        if !found {
            world.world.spawn((ChartComponent(chart),));
        }
    }
    sync_charts_from_ecs();
}

/// Synchronize chart signals from the ECS world.
pub fn sync_charts_from_ecs() {
    use crate::domain::market_data::Symbol;
    use crate::ecs::components::ChartComponent;

    let world = ecs_world().lock().unwrap();
    let updates: Vec<(Symbol, Chart)> = world
        .world
        .query::<&ChartComponent>()
        .iter()
        .map(|(_, comp)| (Symbol::from(comp.0.id.as_str()), comp.0.clone()))
        .collect();
    drop(world);

    globals().charts.update(|map| {
        for (symbol, chart) in updates {
            if let Some(signal) = map.get(&symbol) {
                signal.set(chart);
            }
        }
    });
}
