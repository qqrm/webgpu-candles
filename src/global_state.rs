//! Lazily initialized global reactive signals.
//!
//! This module stores shared application state such as the current price and
//! UI flags. `OnceCell` is used to ensure the globals are created only on first
//! access.

use crate::app::TooltipData;
use crate::domain::{
    chart::{Chart, value_objects::ChartType},
    market_data::{Symbol, TimeInterval},
};
use futures::future::AbortHandle;
use leptos::*;
use once_cell::sync::OnceCell;
use std::collections::HashMap;

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

pub fn ensure_chart(symbol: &Symbol) -> RwSignal<Chart> {
    let charts = &globals().charts;
    charts.update(|map| {
        map.entry(symbol.clone()).or_insert_with(|| {
            create_rw_signal(Chart::new(symbol.value().to_string(), ChartType::Candlestick, 1000))
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
