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
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const LIVE_CHART_CAPACITY: usize = 50_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StressBenchmark {
    pub generated_ms: f64,
    pub loaded_ms: f64,
    pub first_render_ms: f64,
    pub source_candles: usize,
    pub rendered_candles: usize,
}

#[derive(Clone, Copy)]
pub struct Globals {
    pub current_price: RwSignal<f64>,
    pub candle_count: RwSignal<usize>,
    pub is_streaming: RwSignal<bool>,
    pub render_time_ms: RwSignal<f64>,
    pub loading_more: RwSignal<bool>,
    pub tooltip_data: RwSignal<Option<TooltipData>>,
    pub tooltip_visible: RwSignal<bool>,
    pub is_dragging: RwSignal<bool>,
    pub last_mouse_x: RwSignal<f64>,
    pub current_interval: RwSignal<TimeInterval>,
    pub current_symbol: RwSignal<Symbol>,
    pub stream_abort_handles: RwSignal<HashMap<Symbol, AbortHandle>>,
    pub streaming_symbols: RwSignal<HashSet<Symbol>>,
    pub line_visibility: RwSignal<crate::infrastructure::rendering::renderer::LineVisibility>,
    pub domain_state: RwSignal<DomainState>,
    pub view_state: RwSignal<ViewState>,
    pub connection_id: RwSignal<u64>,
    pub chart_view_revision: RwSignal<u64>,
    pub stress_mode: RwSignal<bool>,
    pub stress_running: RwSignal<bool>,
    pub stress_result: RwSignal<Option<StressBenchmark>>,
}

// The production app has one Leptos runtime, so a OnceCell gives lock-free
// access after initialization. Browser unit tests create and dispose several
// runtimes in one process; their signals must be recreated for each live
// runtime instead of being retained forever by the OnceCell.
#[cfg(not(test))]
static GLOBALS: OnceCell<Globals> = OnceCell::new();
#[cfg(test)]
static TEST_GLOBALS: Mutex<Option<Globals>> = Mutex::new(None);
static ECS_WORLD: OnceCell<Mutex<EcsWorld>> = OnceCell::new();

fn create_globals() -> Globals {
    Globals {
        current_price: create_rw_signal(0.0),
        candle_count: create_rw_signal(0),
        is_streaming: create_rw_signal(false),
        render_time_ms: create_rw_signal(0.0),
        loading_more: create_rw_signal(false),
        tooltip_data: create_rw_signal(None),
        tooltip_visible: create_rw_signal(false),
        is_dragging: create_rw_signal(false),
        last_mouse_x: create_rw_signal(0.0),
        current_interval: create_rw_signal(TimeInterval::OneMinute),
        current_symbol: create_rw_signal(Symbol::from("BTCUSDT")),
        stream_abort_handles: create_rw_signal(HashMap::new()),
        streaming_symbols: create_rw_signal(HashSet::new()),
        line_visibility: create_rw_signal(
            crate::infrastructure::rendering::renderer::LineVisibility::default(),
        ),
        domain_state: create_rw_signal(DomainState::new(
            Duration::from_secs(1),
            Arc::new(Vec::new()),
        )),
        view_state: create_rw_signal(ViewState::new(5.0, 1.0, 20.0)),
        connection_id: create_rw_signal(0),
        chart_view_revision: create_rw_signal(0),
        stress_mode: create_rw_signal(false),
        stress_running: create_rw_signal(false),
        stress_result: create_rw_signal(None),
    }
}

pub fn globals() -> Globals {
    #[cfg(not(test))]
    {
        *GLOBALS.get_or_init(create_globals)
    }

    #[cfg(test)]
    {
        let mut globals = TEST_GLOBALS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let signals_are_live =
            globals.as_ref().and_then(|state| state.current_interval.try_get_untracked()).is_some();
        if !signals_are_live {
            *globals = Some(create_globals());
        }
        globals.expect("test globals were initialized")
    }
}

/// Access the global ECS world.
pub fn ecs_world() -> &'static Mutex<EcsWorld> {
    ECS_WORLD.get_or_init(|| Mutex::new(EcsWorld::new()))
}

pub fn get_chart_signal(symbol: &Symbol) -> Option<RwSignal<Chart>> {
    let world = ecs_world().lock().unwrap();
    world.world.query::<&ChartComponent>().iter().find_map(|(_, c)| {
        c.0.try_with_untracked(|chart| chart.id == symbol.value())
            .filter(|matches| *matches)
            .map(|_| c.0)
    })
}

pub fn ensure_chart(symbol: &Symbol) -> RwSignal<Chart> {
    if let Some(sig) = get_chart_signal(symbol) {
        return sig;
    }
    let mut world = ecs_world().lock().unwrap();
    let chart = Chart::new(symbol.value().to_string(), ChartType::Candlestick, LIVE_CHART_CAPACITY);
    let entity = world.spawn_chart(chart);
    world.world.get::<&ChartComponent>(entity).map(|c| c.0).expect("chart just spawned")
}

pub fn stream_abort_handles() -> RwSignal<HashMap<Symbol, AbortHandle>> {
    globals().stream_abort_handles
}

pub fn streaming_symbols() -> RwSignal<HashSet<Symbol>> {
    globals().streaming_symbols
}

pub fn domain_state() -> RwSignal<DomainState> {
    globals().domain_state
}

pub fn view_state() -> RwSignal<ViewState> {
    globals().view_state
}

pub fn connection_id() -> RwSignal<u64> {
    globals().connection_id
}

pub fn chart_view_revision() -> RwSignal<u64> {
    globals().chart_view_revision
}

pub fn stress_mode() -> RwSignal<bool> {
    globals().stress_mode
}

pub fn stress_running() -> RwSignal<bool> {
    globals().stress_running
}

pub fn stress_result() -> RwSignal<Option<StressBenchmark>> {
    globals().stress_result
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
            let matches_symbol =
                comp.0.try_with_untracked(|current| current.id == symbol.value()).unwrap_or(false);
            if matches_symbol {
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
