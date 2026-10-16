//! Compatibility facade for the current application's context.
//! State is owned by a reactive root; no process-wide signal or ECS registry is retained.

use crate::app_context::app_context;
pub use crate::app_context::{Globals, LIVE_CHART_CAPACITY, StressBenchmark};
use crate::domain::{
    DomainState,
    chart::Chart,
    market_data::{Candle, Symbol},
};
use crate::ecs::EcsWorld;
use crate::view_state::ViewState;
use futures::future::AbortHandle;
use leptos::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub fn globals() -> Globals {
    app_context().signals
}
pub fn ecs_world() -> Arc<Mutex<EcsWorld>> {
    app_context().ecs_world()
}
pub fn get_chart_signal(symbol: &Symbol) -> Option<RwSignal<Chart>> {
    app_context().get_chart_signal(symbol)
}
pub fn ensure_chart(symbol: &Symbol) -> RwSignal<Chart> {
    app_context().ensure_chart(symbol)
}
pub fn push_realtime_candle(candle: Candle) {
    app_context().push_realtime_candle(candle);
}
pub fn set_chart_in_ecs(symbol: &Symbol, chart: Chart) {
    app_context().set_chart(symbol, chart);
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
