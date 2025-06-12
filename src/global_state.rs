use crate::app::TooltipData;
use crate::domain::market_data::TimeInterval;
use leptos::*;
use once_cell::sync::OnceCell;

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
    pub last_mouse_y: RwSignal<f64>,
    pub current_interval: RwSignal<TimeInterval>,
}

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
        zoom_level: create_rw_signal(1.0),
        pan_offset: create_rw_signal(0.0),
        is_dragging: create_rw_signal(false),
        last_mouse_x: create_rw_signal(0.0),
        last_mouse_y: create_rw_signal(0.0),
        current_interval: create_rw_signal(TimeInterval::OneMinute),
    })
}
