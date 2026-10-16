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
use leptos::prelude::*;
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
        current_price: RwSignal::new(0.0),
        candle_count: RwSignal::new(0),
        is_streaming: RwSignal::new(false),
        render_time_ms: RwSignal::new(0.0),
        loading_more: RwSignal::new(false),
        tooltip_data: RwSignal::new(None),
        tooltip_visible: RwSignal::new(false),
        is_dragging: RwSignal::new(false),
        last_mouse_x: RwSignal::new(0.0),
        current_interval: RwSignal::new(TimeInterval::OneMinute),
        current_symbol: RwSignal::new(Symbol::from("BTCUSDT")),
        stream_abort_handles: RwSignal::new(HashMap::new()),
        streaming_symbols: RwSignal::new(HashSet::new()),
        line_visibility: RwSignal::new(LineVisibility::default()),
        domain_state: RwSignal::new(DomainState::new(Duration::from_secs(1), Arc::new(Vec::new()))),
        view_state: RwSignal::new(ViewState::new(5.0, 1.0, 20.0)),
        connection_id: RwSignal::new(0),
        chart_view_revision: RwSignal::new(0),
        stress_mode: RwSignal::new(false),
        stress_running: RwSignal::new(false),
        stress_result: RwSignal::new(None),
    }
}

/// State owned by one application root. Clones share only that root's ECS.
#[derive(Clone)]
pub struct AppContext {
    pub signals: Globals,
    world: Arc<Mutex<EcsWorld>>,
    owner: CallbackOwner,
}

impl AppContext {
    /// Install a fresh context in the current reactive owner.
    pub fn provide() -> Self {
        let owner = CallbackOwner::capture().expect("AppContext requires a reactive owner");
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
        world.world.query::<&ChartComponent>().iter().find_map(|c| {
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
        self.owner
            .run(|| {
                let mut world = self.world.lock().unwrap();
                let chart = Chart::new(
                    symbol.value().to_string(),
                    ChartType::Candlestick,
                    LIVE_CHART_CAPACITY,
                );
                let entity = world.spawn_chart(chart);
                world.world.get::<&ChartComponent>(entity).expect("chart just spawned").0
            })
            .expect("application owner has been disposed")
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
        let _ = owner.run(|| callback(value));
    }
}

/// A weak owner reference plus an arena token: callbacks cannot keep roots alive,
/// and cleanup invalidates callbacks even while a strong Owner handle still exists.
#[derive(Clone)]
pub(crate) struct CallbackOwner {
    owner: leptos::reactive::owner::WeakOwner,
    token: StoredValue<()>,
}

impl CallbackOwner {
    pub(crate) fn capture() -> Option<Self> {
        Owner::current().map(|owner| Self { owner: owner.downgrade(), token: StoredValue::new(()) })
    }

    pub(crate) fn run<T>(&self, callback: impl FnOnce() -> T) -> Option<T> {
        if self.token.is_disposed() {
            return None;
        }
        self.owner.upgrade().map(|owner| owner.with(callback))
    }
}
