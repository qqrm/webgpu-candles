//! Application-owned reactive state and chart operations.

use crate::domain::{
    DomainState,
    chart::{Chart, value_objects::ChartType},
    market_data::{Candle, Symbol, TimeInterval},
};
use crate::ecs::{EcsWorld, components::ChartComponent};
use crate::ui_state::{LineVisibility, TooltipData};
use crate::view_state::ViewState;
use futures::future::AbortHandle;
use leptos::*;
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
    pub line_visibility: RwSignal<LineVisibility>,
    pub domain_state: RwSignal<DomainState>,
    pub view_state: RwSignal<ViewState>,
    pub connection_id: RwSignal<u64>,
    pub chart_view_revision: RwSignal<u64>,
    pub stress_mode: RwSignal<bool>,
    pub stress_running: RwSignal<bool>,
    pub stress_result: RwSignal<Option<StressBenchmark>>,
}

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
        line_visibility: create_rw_signal(LineVisibility::default()),
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

/// State owned by one application root. Clones share only that root's ECS.
#[derive(Clone)]
pub struct AppContext {
    pub signals: Globals,
    world: Arc<Mutex<EcsWorld>>,
    owner: Owner,
}

impl AppContext {
    /// Install a fresh context in the current reactive owner.
    pub fn provide() -> Self {
        let owner = Owner::current().expect("AppContext requires a reactive owner");
        let context =
            Self { signals: create_globals(), world: Arc::new(Mutex::new(EcsWorld::new())), owner };
        provide_context(context.clone());
        let cleanup = context.clone();
        on_cleanup(move || {
            // Cleanup runs before owned signals are disposed. Do not start new tasks here.
            let _ = cleanup.signals.stream_abort_handles.try_update(|handles| {
                for (_, handle) in handles.drain() {
                    handle.abort();
                }
            });
            cleanup.world.lock().unwrap().world.clear();
        });
        context
    }

    /// Cancel the previous timeframe's streams and reset its connection status.
    pub fn abort_streams(&self) {
        self.signals.stream_abort_handles.update(|handles| {
            for (_, handle) in handles.drain() {
                handle.abort();
            }
        });
        self.signals.streaming_symbols.set(Default::default());
        self.signals.is_streaming.set(false);
    }

    pub fn ecs_world(&self) -> Arc<Mutex<EcsWorld>> {
        self.world.clone()
    }

    pub fn get_chart_signal(&self, symbol: &Symbol) -> Option<RwSignal<Chart>> {
        let world = self.world.lock().unwrap();
        world.world.query::<&ChartComponent>().iter().find_map(|(_, c)| {
            c.0.try_with_untracked(|chart| chart.id == symbol.value())
                .filter(|matches| *matches)
                .map(|_| c.0)
        })
    }

    pub fn ensure_chart(&self, symbol: &Symbol) -> RwSignal<Chart> {
        if let Some(signal) = self.get_chart_signal(symbol) {
            return signal;
        }
        // Charts must survive short-lived effects which first request them.
        with_owner(self.owner, || {
            let mut world = self.world.lock().unwrap();
            let chart =
                Chart::new(symbol.value().to_string(), ChartType::Candlestick, LIVE_CHART_CAPACITY);
            let entity = world.spawn_chart(chart);
            world.world.get::<&ChartComponent>(entity).expect("chart just spawned").0
        })
    }

    pub fn push_realtime_candle(&self, candle: Candle) {
        use crate::ecs::components::CandleComponent;
        let mut world = self.world.lock().unwrap();
        world.world.spawn((CandleComponent(candle),));
        world.run_candle_system_parallel();
        world.run_viewport_system();
    }

    pub fn set_chart(&self, symbol: &Symbol, chart: Chart) {
        let signal = self.ensure_chart(symbol);
        signal.set(chart);
        self.world.lock().unwrap().run_viewport_system();
    }
}

/// Compatibility access for consumers not yet passed an explicit context.
/// Production installs the context at App; standalone tests may initialize their owner lazily.
pub fn app_context() -> AppContext {
    use_context::<AppContext>().unwrap_or_else(AppContext::provide)
}

/// Keep DOM callbacks in their application's owner, ignoring events after disposal.
pub fn bind_context<T: 'static>(mut callback: impl FnMut(T) + 'static) -> impl FnMut(T) {
    let owner = app_context().owner;
    move |value| {
        let _ = try_with_owner(owner, || callback(value));
    }
}
