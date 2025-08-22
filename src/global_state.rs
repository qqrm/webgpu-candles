//! Lazily initialized global reactive signals.
//!
//! This module stores shared application state such as the current price and
//! UI flags. `OnceCell` is used to ensure the globals are created only on first
//! access.

use crate::app::TooltipData;
use crate::domain::{
    DomainState,
    chart::{Chart, value_objects::ChartType},
    market_data::{Candle, Symbol, TimeInterval},
};
use crate::ecs::{EcsWorld, components::ChartComponent};
use crate::view_state::ViewState;
use futures::future::AbortHandle;
use leptos::*;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct Globals {
    pub current_price: RwSignal<f64>,
    pub candle_count: RwSignal<usize>,
    pub is_streaming: RwSignal<bool>,
    pub max_volume: RwSignal<f64>,
    pub loading_more: RwSignal<bool>,
    pub tooltip_data: RwSignal<Option<TooltipData>>,
    pub tooltip_visible: RwSignal<bool>,
    pub is_dragging: RwSignal<bool>,
    pub last_mouse_x: RwSignal<f64>,
    pub current_interval: RwSignal<TimeInterval>,
    pub current_symbol: RwSignal<Symbol>,
    pub stream_abort_handles: RwSignal<HashMap<Symbol, AbortHandle>>,
    pub line_visibility: RwSignal<crate::infrastructure::rendering::renderer::LineVisibility>,
    pub domain_state: RwSignal<DomainState>,
    pub view_state: RwSignal<ViewState>,
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
        is_dragging: create_rw_signal(false),
        last_mouse_x: create_rw_signal(0.0),
        current_interval: create_rw_signal(TimeInterval::OneMinute),
        current_symbol: create_rw_signal(Symbol::from("BTCUSDT")),
        stream_abort_handles: create_rw_signal(HashMap::new()),
        line_visibility: create_rw_signal(
            crate::infrastructure::rendering::renderer::LineVisibility::default(),
        ),
        domain_state: create_rw_signal(DomainState::new(
            Duration::from_secs(1),
            Arc::new(Vec::new()),
        )),
        view_state: create_rw_signal(ViewState::new(5.0, 1.0, 20.0)),
    })
}

/// Access the global ECS world.
pub fn ecs_world() -> &'static Mutex<EcsWorld> {
    ECS_WORLD.get_or_init(|| Mutex::new(EcsWorld::new()))
}

pub fn get_chart_signal(symbol: &Symbol) -> Option<RwSignal<Chart>> {
    let world = ecs_world().lock().unwrap();
    world
        .world
        .query::<&ChartComponent>()
        .iter()
        .find(|(_, c)| c.0.with(|ch| ch.id == symbol.value()))
        .map(|(_, c)| c.0)
}

pub fn ensure_chart(symbol: &Symbol) -> RwSignal<Chart> {
    if let Some(sig) = get_chart_signal(symbol) {
        return sig;
    }
    let mut world = ecs_world().lock().unwrap();
    let chart = Chart::new(symbol.value().to_string(), ChartType::Candlestick, 1000);
    let entity = world.spawn_chart(chart);
    world.world.get::<&ChartComponent>(entity).map(|c| c.0).expect("chart just spawned")
}

pub fn stream_abort_handles() -> RwSignal<HashMap<Symbol, AbortHandle>> {
    globals().stream_abort_handles
}

pub fn domain_state() -> RwSignal<DomainState> {
    globals().domain_state
}

pub fn view_state() -> RwSignal<ViewState> {
    globals().view_state
}

/// Add a candle to the ECS world and process systems.
pub fn push_realtime_candle(candle: Candle) {
    use crate::ecs::components::CandleComponent;
    {
        let mut world = ecs_world().lock().unwrap();
        world.world.spawn((CandleComponent(candle),));
        world.run_candle_system_parallel();
        world.run_viewport_system();
    }
}

/// Replace or spawn a chart entity in the ECS world.
pub fn set_chart_in_ecs(symbol: &Symbol, chart: Chart) {
    use crate::ecs::components::ChartComponent;
    {
        let mut world = ecs_world().lock().unwrap();
        let mut found = false;
        for (_, comp) in world.world.query::<&mut ChartComponent>().iter() {
            if comp.0.with(|c| c.id.clone()) == symbol.value() {
                comp.0.set(chart.clone());
                found = true;
                break;
            }
        }
        if !found {
            world.spawn_chart(chart);
        }
        world.run_viewport_system();
    }
}
