//! Main Leptos application module.
//!
//! Handles canvas interactions, zoom/pan logic and connects to the
//! WebSocket stream providing market data.

use futures::{channel::oneshot, lock::Mutex};
use js_sys;
use leptos::html::Canvas;
use leptos::spawn_local_with_current_owner;
use leptos::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;
use wasm_bindgen::JsCast;

use crate::event_utils::{EventOptions, window_event_listener_with_options};
use crate::global_signals;
use crate::global_state::{connection_id, domain_state, ensure_chart, get_chart_signal, globals};
use crate::{
    domain::{
        chart::Chart,
        logging::{LogComponent, get_logger},
        market_data::{
            Candle, TimeInterval,
            value_objects::{Symbol, default_symbols},
        },
    },
    infrastructure::rendering::renderer::{
        EDGE_GAP, LineVisibility, MAX_ELEMENT_WIDTH, MIN_ELEMENT_WIDTH, enqueue_render_task,
        init_render_queue, set_global_renderer, spacing_ratio_for, with_global_renderer,
    },
    infrastructure::{
        http::binance_rest_client::BinanceRestClient, rendering::WebGpuRenderer,
        websocket::BinanceWebSocketClient,
    },
    time_utils::{format_time_label_for_span, format_timestamp_utc},
};
use gloo_timers::future::sleep;

/// Maximum number of candles visible at 1x zoom
const MAX_VISIBLE_CANDLES: f64 = 32.0;
/// Minimum number of candles that must remain visible
const MIN_VISIBLE_CANDLES: f64 = 1.0;

/// Minimum allowed zoom level
const MIN_ZOOM_LEVEL: f64 = MAX_VISIBLE_CANDLES / 300.0;
/// Maximum allowed zoom level
const MAX_ZOOM_LEVEL: f64 = 32.0;

const CHART_WIDTH_PX: f64 = 800.0;
const CHART_HEIGHT_PX: f64 = 500.0;
const ZOOM_STEP: f64 = 1.2;
const DEFAULT_VISIBLE_CANDLES: usize = 96;

/// Index threshold to trigger history backfill
pub const HISTORY_PRELOAD_THRESHOLD: usize = 200;

/// Maximum candles per backfill request
const HISTORY_FETCH_LIMIT: u32 = 1000;

/// Check if more historical data should be fetched
pub fn should_fetch_history(left_index: usize) -> bool {
    left_index < HISTORY_PRELOAD_THRESHOLD
}

/// Calculate visible range based on zoom level and pan offset
pub fn visible_range(len: usize, zoom: f64, pan: f64) -> (usize, usize) {
    let visible = ((MAX_VISIBLE_CANDLES / zoom).max(MIN_VISIBLE_CANDLES).min(len as f64)) as isize;
    let base_start = len as isize - visible;
    let offset = pan.round() as isize;
    let min_start = 0;
    let max_start = len as isize - visible;
    let start = (base_start + offset).clamp(min_start, max_start);
    (start as usize, visible as usize)
}

/// Check if the viewport is already at the latest candle
pub fn should_auto_scroll(len: usize, zoom: f64, pan: f64) -> bool {
    let (start, visible) = visible_range(len, zoom, pan);
    start + visible >= len
}

/// Convert a horizontal drag into the viewport fraction expected by [`Chart::pan`].
/// Dragging the chart to the right reveals older candles, hence the inverted sign.
pub fn pan_ratio_from_pixels(delta_x: f64, canvas_width: f64) -> f32 {
    if !delta_x.is_finite() || !canvas_width.is_finite() || canvas_width <= 0.0 {
        return 0.0;
    }

    (-(delta_x / canvas_width)).clamp(-1.0, 1.0) as f32
}

/// Determine visible range using timestamps from the viewport
pub fn visible_range_by_time(
    candles: &[Candle],
    viewport: &crate::domain::chart::value_objects::Viewport,
    zoom: f64,
) -> (usize, usize) {
    if candles.is_empty() {
        return (0, 0);
    }

    let visible =
        ((MAX_VISIBLE_CANDLES / zoom).max(MIN_VISIBLE_CANDLES).min(candles.len() as f64)) as usize;

    let start_ts = viewport.start_time as u64;
    // Use `partition_point` to find the first candle after `start_ts`.
    // This avoids scanning the entire slice manually.
    let start_idx = candles.partition_point(|c| c.timestamp.value() < start_ts);

    let max_start = candles.len().saturating_sub(visible);
    // Clamp to ensure we always display `visible` candles.
    let start = start_idx.min(max_start);
    (start, visible)
}

/// Calculate price axis levels based on the viewport
pub fn price_levels(viewport: &crate::domain::chart::value_objects::Viewport) -> Vec<f64> {
    let step = (viewport.max_price - viewport.min_price) as f64 / 8.0;
    (0..=8).rev().map(|i| viewport.min_price as f64 + i as f64 * step).collect()
}

/// Calculate zoom level and pan offset based on the viewport
use std::collections::VecDeque;

pub fn viewport_zoom_pan(
    candles: &VecDeque<Candle>,
    viewport: &crate::domain::chart::value_objects::Viewport,
) -> (f64, f64) {
    if candles.is_empty() {
        return (1.0, 0.0);
    }

    let start_idx = candles
        .iter()
        .position(|c| c.timestamp.value() >= viewport.start_time as u64)
        .unwrap_or(candles.len());
    let end_idx = candles
        .iter()
        .position(|c| c.timestamp.value() > viewport.end_time as u64)
        .unwrap_or(candles.len());

    let mut visible = end_idx.saturating_sub(start_idx);
    visible = visible.clamp(MIN_VISIBLE_CANDLES as usize, candles.len());

    let zoom = MAX_VISIBLE_CANDLES / visible as f64;
    let base_start = candles.len().saturating_sub(visible);
    let pan = (start_idx as isize - base_start as isize) as f64;
    (zoom, pan)
}

// Helper aliases for global signals
global_signals! {
    pub global_current_price => current_price: f64,
    global_candle_count => candle_count: usize,
    global_is_streaming => is_streaming: bool,
    global_max_volume => max_volume: f64,
    loading_more => loading_more: bool,
    tooltip_data => tooltip_data: Option<TooltipData>,
    tooltip_visible => tooltip_visible: bool,
    is_dragging => is_dragging: bool,
    last_mouse_x => last_mouse_x: f64,
    pub current_interval => current_interval: TimeInterval,
    pub current_symbol => current_symbol: Symbol,
    pub stream_abort_handles => stream_abort_handles: HashMap<Symbol, futures::future::AbortHandle>,
    pub global_line_visibility => line_visibility: LineVisibility,
}

/// 📈 Fetch additional history and prepend it to the list
fn fetch_more_history(set_status: WriteSignal<String>) {
    if loading_more().get() {
        return;
    }

    ensure_chart(&current_symbol().get_untracked());
    let chart = get_chart_signal(&current_symbol().get_untracked()).unwrap();
    let oldest_ts = chart.with(|c| {
        c.get_series(current_interval().get_untracked())
            .and_then(|s| s.get_candles().front())
            .map(|c| c.timestamp.value())
    });
    let end_time = match oldest_ts {
        Some(ts) if ts > 0 => ts - 1,
        _ => return,
    };

    loading_more().set(true);

    let symbol = current_symbol().get_untracked();
    let _ = spawn_local_with_current_owner(async move {
        let interval = current_interval().get_untracked();
        let client = BinanceRestClient::new(symbol.clone(), interval);
        let result = client.fetch_historical_before(end_time, HISTORY_FETCH_LIMIT).await;
        match result {
            Ok(mut new_candles) => {
                new_candles.sort_by_key(|c| c.timestamp.value());
                new_candles.dedup_by_key(|c| c.timestamp.value());
                chart.update(|ch| {
                    for candle in new_candles.iter() {
                        ch.add_candle(candle.clone());
                    }
                });
                chart.with_untracked(|c| {
                    if c.get_candle_count() > 0
                        && with_global_renderer(|r| {
                            let interval = current_interval().get_untracked();
                            let series = c.get_series(interval).unwrap();
                            let (zoom, pan) = viewport_zoom_pan(series.get_candles(), &c.viewport);
                            r.set_zoom_params(zoom, pan);
                            let _ = r.render(c);
                        })
                        .is_none()
                    {
                        // renderer not available
                    }
                });

                let new_count = chart.with(|c| c.get_candle_count());
                let max_volume = chart.with(|c| {
                    c.get_series(current_interval().get_untracked())
                        .unwrap()
                        .get_candles()
                        .iter()
                        .map(|c| c.ohlcv.volume.value())
                        .fold(0.0f64, |a, b| a.max(b))
                });
                global_candle_count().set(new_count);
                global_max_volume().set(max_volume);

                set_status.set(format!("📈 Loaded {} older candles", new_candles.len()));
            }
            Err(e) => set_status.set(format!("❌ Failed to load more data: {e}")),
        }

        sleep(Duration::from_millis(500)).await;
        loading_more().set(false);
    });
}

/// 🎯 Data for the tooltip
#[derive(Clone, Debug)]
pub struct TooltipData {
    pub candle: Candle,
    pub x: f64,
    pub y: f64,
    pub formatted_text: String,
}

impl TooltipData {
    pub fn new(candle: Candle, x: f64, y: f64) -> Self {
        let change = candle.ohlcv.close.value() - candle.ohlcv.open.value();
        let change_pct = (change / candle.ohlcv.open.value()) * 100.0;
        let direction = if change >= 0.0 { "UP" } else { "DOWN" };
        let time_str = format_timestamp_utc(candle.timestamp.value());

        let symbol = current_symbol().get_untracked();
        let formatted_text = format!(
            "{}  {}\nOpen       ${:.2}\nHigh       ${:.2}\nLow        ${:.2}\nClose      ${:.2}\nChange     ${:.2} ({:.2}%)\nVolume     {:.4}\n{}",
            symbol.value(),
            direction,
            candle.ohlcv.open.value(),
            candle.ohlcv.high.value(),
            candle.ohlcv.low.value(),
            candle.ohlcv.close.value(),
            change,
            change_pct,
            candle.ohlcv.volume.value(),
            time_str
        );

        Self { candle, x, y, formatted_text }
    }
}

/// 🦀 Main Crypto Chart component built with Leptos
#[component]
pub fn app() -> impl IntoView {
    // 🚀 Initialize the global logger on application start
    use crate::domain::logging::get_logger;

    // Extra console.log for diagnostics
    web_sys::console::log_1(&"🚀 Starting Crypto Chart App".into());

    get_logger().info(LogComponent::Presentation("App"), "🚀 Starting Crypto Chart App");

    web_sys::console::log_1(&"📦 Creating view...".into());

    // Create application-wide signals under the root component owner. Creating
    // them lazily inside a reactive closure would dispose them when that closure
    // is rebuilt while the static registry still retained their handles.
    globals();
    ensure_chart(&current_symbol().get_untracked());

    view! {
        <style>
            {r#"
            :root {
                color-scheme: dark;
                --page: #080b11;
                --surface: #10151e;
                --surface-raised: #151c27;
                --surface-hover: #1b2431;
                --border: #263141;
                --border-strong: #344258;
                --text: #f4f7fb;
                --text-muted: #8d9aab;
                --text-subtle: #667386;
                --buy: #74c787;
                --sell: #e16c48;
                --accent: #8db4ff;
                --chart: #253242;
            }

            * {
                box-sizing: border-box;
            }

            body {
                min-width: 320px;
                background: var(--page);
            }

            button,
            input {
                font: inherit;
            }

            button:focus-visible,
            input:focus-visible,
            canvas:focus-visible {
                outline: 2px solid var(--accent);
                outline-offset: 2px;
            }

            .bitcoin-chart-app {
                min-height: 100dvh;
                padding: clamp(12px, 2.5vw, 32px);
                color: var(--text);
                background:
                    radial-gradient(circle at 15% -20%, rgba(78, 111, 168, 0.18), transparent 38rem),
                    var(--page);
                font-family: Inter, ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
                font-variant-numeric: tabular-nums;
            }

            .app-shell {
                width: min(1180px, 100%);
                margin: 0 auto;
            }

            .header {
                margin-bottom: 14px;
                overflow: hidden;
                border: 1px solid var(--border);
                border-radius: 16px;
                background: rgba(16, 21, 30, 0.92);
                box-shadow: 0 24px 60px rgba(0, 0, 0, 0.24);
            }

            .topbar {
                display: flex;
                align-items: center;
                justify-content: space-between;
                gap: 16px;
                padding: 14px 18px;
                border-bottom: 1px solid var(--border);
            }

            .brand {
                display: flex;
                align-items: center;
                gap: 11px;
                min-width: 0;
            }

            .brand-mark {
                display: grid;
                width: 34px;
                height: 34px;
                flex: 0 0 auto;
                place-items: center;
                border: 1px solid #4a6283;
                border-radius: 10px;
                background: linear-gradient(145deg, #24344c, #151e2b);
                color: #bcd3ff;
                font: 800 13px/1 ui-monospace, monospace;
            }

            .brand-name {
                font-size: 15px;
                font-weight: 720;
                letter-spacing: -0.01em;
            }

            .brand-caption {
                margin-top: 2px;
                color: var(--text-muted);
                font-size: 11px;
            }

            .connection-pill {
                display: inline-flex;
                align-items: center;
                gap: 7px;
                padding: 7px 10px;
                border: 1px solid rgba(225, 108, 72, 0.35);
                border-radius: 999px;
                color: #f1a48e;
                background: rgba(225, 108, 72, 0.08);
                font-size: 11px;
                font-weight: 700;
                letter-spacing: 0.07em;
            }

            .connection-pill.live {
                border-color: rgba(116, 199, 135, 0.35);
                color: #9ee0ad;
                background: rgba(116, 199, 135, 0.08);
            }

            .connection-dot {
                width: 7px;
                height: 7px;
                border-radius: 50%;
                background: currentColor;
                box-shadow: 0 0 12px currentColor;
            }

            .market-summary {
                display: grid;
                grid-template-columns: minmax(220px, 1.35fr) repeat(3, minmax(110px, 0.65fr));
                gap: 1px;
                background: var(--border);
            }

            .market-primary,
            .metric {
                min-width: 0;
                padding: 17px 18px;
                background: var(--surface);
            }

            .market-symbol {
                color: var(--text-muted);
                font-size: 12px;
                font-weight: 650;
                letter-spacing: 0.06em;
            }

            .market-price {
                margin-top: 5px;
                font: 720 clamp(24px, 3vw, 34px)/1.08 ui-monospace, "SFMono-Regular", Consolas, monospace;
                letter-spacing: -0.05em;
            }

            .metric-value {
                overflow: hidden;
                color: var(--text);
                font: 650 16px/1.2 ui-monospace, "SFMono-Regular", Consolas, monospace;
                text-overflow: ellipsis;
            }

            .metric-label {
                margin-top: 6px;
                color: var(--text-muted);
                font-size: 11px;
            }

            .chart-container {
                display: flex;
                flex-direction: column;
                align-items: center;
                gap: 0;
                overflow: hidden;
                border: 1px solid var(--border);
                border-radius: 16px;
                background: var(--surface);
                box-shadow: 0 24px 60px rgba(0, 0, 0, 0.24);
            }

            .chart-toolbar {
                display: flex;
                width: 100%;
                align-items: flex-end;
                justify-content: space-between;
                gap: 16px;
                padding: 12px 14px;
                border-bottom: 1px solid var(--border);
                background: var(--surface-raised);
            }

            .toolbar-cluster,
            .selector-group,
            .chart-actions {
                display: flex;
                align-items: center;
                gap: 6px;
            }

            .toolbar-cluster {
                flex-wrap: wrap;
                gap: 10px 16px;
            }

            .selector-label {
                margin-right: 2px;
                color: var(--text-subtle);
                font-size: 10px;
                font-weight: 750;
                letter-spacing: 0.09em;
                text-transform: uppercase;
            }

            .selector-button,
            .chart-action {
                min-height: 30px;
                border: 1px solid transparent;
                border-radius: 7px;
                color: var(--text-muted);
                background: transparent;
                cursor: pointer;
                font-size: 11px;
                font-weight: 680;
                transition: 120ms ease;
            }

            .selector-button {
                padding: 5px 8px;
            }

            .selector-button:hover,
            .chart-action:hover {
                color: var(--text);
                background: var(--surface-hover);
            }

            .selector-button.active {
                border-color: var(--border-strong);
                color: var(--text);
                background: #253247;
                box-shadow: inset 0 1px rgba(255, 255, 255, 0.04);
            }

            .chart-action {
                min-width: 32px;
                padding: 5px 9px;
                border-color: var(--border);
                background: #111823;
            }

            .chart-action.reset {
                min-width: auto;
                color: #bcd3ff;
            }

            .chart-workspace {
                width: 100%;
                padding: clamp(10px, 2vw, 20px);
                background: #0d121a;
            }

            .chart-viewport {
                position: relative;
                width: min(100%, 960px);
                margin: 0 auto;
                aspect-ratio: 8 / 5;
            }

            #chart-canvas {
                display: block;
                width: 100%;
                height: 100%;
                border: 1px solid var(--border-strong);
                border-radius: 10px;
                background: var(--chart);
                cursor: crosshair;
                outline: none;
                touch-action: none;
                box-shadow: inset 0 1px rgba(255, 255, 255, 0.03);
            }

            .price-scale {
                position: absolute;
                inset: 0;
                top: 0;
                pointer-events: none;
            }

            .price-level {
                position: absolute;
                right: 7px;
                transform: translateY(-50%);
                padding: 2px 5px;
                border-radius: 4px;
                color: #9aa8bb;
                background: rgba(8, 11, 17, 0.78);
                font: 10px/1.35 ui-monospace, "SFMono-Regular", Consolas, monospace;
                backdrop-filter: blur(5px);
            }

            .current-price-label {
                position: absolute;
                right: 7px;
                transform: translateY(-50%);
                padding: 4px 7px;
                border: 1px solid rgba(141, 180, 255, 0.55);
                border-radius: 5px;
                color: #d9e6ff;
                background: rgba(31, 55, 94, 0.94);
                font-size: 11px;
                font-weight: 750;
                white-space: nowrap;
                box-shadow: 0 4px 14px rgba(0, 0, 0, 0.25);
            }

            .price-value {
                font-family: ui-monospace, "SFMono-Regular", Consolas, monospace;
            }

            .tooltip {
                position: absolute;
                max-width: min(250px, calc(100% - 16px));
                padding: 10px 12px;
                border: 1px solid var(--border-strong);
                border-radius: 8px;
                color: #dce5f2;
                background: rgba(9, 13, 20, 0.94);
                font: 11px/1.55 ui-monospace, "SFMono-Regular", Consolas, monospace;
                white-space: pre-line;
                pointer-events: none;
                z-index: 4;
                box-shadow: 0 14px 34px rgba(0, 0, 0, 0.45);
                backdrop-filter: blur(10px);
                transform: translate(10px, calc(-100% - 8px));
            }

            .loading-badge {
                position: absolute;
                top: 10px;
                left: 10px;
                z-index: 3;
                padding: 5px 8px;
                border: 1px solid var(--border-strong);
                border-radius: 999px;
                color: #bcd3ff;
                background: rgba(9, 13, 20, 0.86);
                font-size: 10px;
            }

            .time-scale {
                display: flex;
                width: min(100%, 960px);
                height: 28px;
                align-items: center;
                justify-content: space-between;
                margin: 5px auto 0;
                padding: 0 8px;
                color: var(--text-subtle);
                font: 10px/1 ui-monospace, "SFMono-Regular", Consolas, monospace;
            }

            .indicator-bar {
                display: flex;
                width: 100%;
                flex-wrap: wrap;
                align-items: center;
                gap: 7px;
                padding: 11px 14px;
                border-top: 1px solid var(--border);
                background: var(--surface-raised);
            }

            .indicator-title {
                margin-right: 3px;
                color: var(--text-subtle);
                font-size: 10px;
                font-weight: 750;
                letter-spacing: 0.09em;
                text-transform: uppercase;
            }

            .indicator-toggle {
                display: inline-flex;
                align-items: center;
                gap: 6px;
                padding: 5px 8px;
                border: 1px solid var(--border);
                border-radius: 999px;
                color: var(--text-subtle);
                background: #111823;
                cursor: pointer;
                font-size: 10px;
                font-weight: 650;
                transition: 120ms ease;
            }

            .indicator-toggle:has(input:checked) {
                color: #dce5f2;
                border-color: var(--border-strong);
                background: var(--surface-hover);
            }

            .indicator-toggle input {
                position: absolute;
                width: 1px;
                height: 1px;
                overflow: hidden;
                opacity: 0;
            }

            .indicator-dot {
                width: 7px;
                height: 7px;
                border-radius: 50%;
                background: var(--indicator-color, #fff);
                opacity: 0.34;
            }

            .indicator-toggle:has(input:checked) .indicator-dot {
                opacity: 1;
                box-shadow: 0 0 8px color-mix(in srgb, var(--indicator-color) 70%, transparent);
            }

            .chart-footer {
                display: flex;
                width: 100%;
                align-items: center;
                justify-content: space-between;
                gap: 12px;
                padding: 10px 14px;
                border-top: 1px solid var(--border);
                color: var(--text-subtle);
                background: #0e141d;
                font-size: 10px;
            }

            .status {
                overflow: hidden;
                color: #98dba8;
                text-overflow: ellipsis;
                white-space: nowrap;
            }

            .control-hints {
                text-align: right;
            }

            @media (max-width: 760px) {
                .market-summary {
                    grid-template-columns: repeat(3, 1fr);
                }

                .market-primary {
                    grid-column: 1 / -1;
                }

                .chart-toolbar {
                    align-items: flex-start;
                    flex-direction: column;
                }

                .chart-actions {
                    width: 100%;
                    justify-content: flex-end;
                }

                .chart-footer {
                    align-items: flex-start;
                    flex-direction: column;
                }

                .control-hints {
                    text-align: left;
                }
            }

            @media (max-width: 460px) {
                .bitcoin-chart-app {
                    padding: 8px;
                }

                .header,
                .chart-container {
                    border-radius: 12px;
                }

                .brand-caption,
                .control-hints {
                    display: none;
                }

                .market-primary,
                .metric {
                    padding: 13px 12px;
                }

                .selector-group {
                    flex-wrap: wrap;
                }
            }
            "#}
        </style>
        <div class="bitcoin-chart-app">
            <main class="app-shell">
                <Header />
                <ChartContainer />
            </main>
        </div>
    }
}

/// 📊 Price header with real data
#[component]
fn header() -> impl IntoView {
    // Use global signals for real data
    let current_price = global_current_price();
    let candle_count = global_candle_count();
    let is_streaming = global_is_streaming();
    let max_volume = global_max_volume();

    let zoom_level = move || {
        get_chart_signal(&current_symbol().get_untracked())
            .and_then(|chart| {
                chart.try_with(|c| {
                    let interval = current_interval().get_untracked();
                    let series = c.get_series(interval)?;
                    Some(viewport_zoom_pan(series.get_candles(), &c.viewport).0)
                })
            })
            .flatten()
            .unwrap_or(1.0)
    };

    view! {
        <header class="header">
            <div class="topbar">
                <div class="brand">
                    <span class="brand-mark" aria-hidden="true">"WG"</span>
                    <div>
                        <div class="brand-name">"WebGPU Candles"</div>
                        <div class="brand-caption">"Live market rendering in Rust + WebAssembly"</div>
                    </div>
                </div>
                <div
                    class="connection-pill"
                    class:live=move || is_streaming.get()
                    role="status"
                >
                    <span class="connection-dot"></span>
                    {move || if is_streaming.get() { "LIVE" } else { "CONNECTING" }}
                </div>
            </div>

            <div class="market-summary">
                <div class="market-primary">
                    <div class="market-symbol">
                        {move || format!("{} · SPOT", current_symbol().get().value())}
                    </div>
                    <div class="market-price">
                        {move || format!("${:.2}", current_price.get())}
                    </div>
                </div>
                <div class="metric">
                    <div class="metric-value">{move || candle_count.get().to_string()}</div>
                    <div class="metric-label">"Candles loaded"</div>
                </div>
                <div class="metric">
                    <div class="metric-value">{move || format!("{:.2}", max_volume.get())}</div>
                    <div class="metric-label">"Peak volume"</div>
                </div>
                <div class="metric">
                    <div class="metric-value">{move || format!("{:.1}×", zoom_level())}</div>
                    <div class="metric-label">"Viewport zoom"</div>
                </div>
            </div>
        </header>
    }
}

/// ⏰ Time scale below the chart
#[component]
fn TimeScale(chart: RwSignal<Chart>) -> impl IntoView {
    let time_labels = move || {
        let interval = current_interval().get_untracked();
        let candles = chart.with(|c| c.get_series(interval).unwrap().get_candles().clone());
        let zoom = chart.with(|c| viewport_zoom_pan(&candles, &c.viewport).0);

        if candles.is_empty() {
            return vec![];
        }

        let (start_idx, visible) = chart.with(|c| {
            let candle_vec: Vec<_> = candles.iter().cloned().collect();
            visible_range_by_time(&candle_vec, &c.viewport, zoom)
        });
        let end_idx = (start_idx + visible).min(candles.len());
        let span_ms = candles
            .get(start_idx)
            .zip(candles.get(end_idx.saturating_sub(1)))
            .map(|(first, last)| last.timestamp.value().saturating_sub(first.timestamp.value()))
            .unwrap_or_default();

        // Show 5 time labels
        let num_labels = 5;
        let mut labels = Vec::new();

        for i in 0..num_labels {
            let index = (i * visible) / (num_labels - 1);
            if let Some(candle) =
                candles.iter().skip(start_idx).nth(index.min(visible.saturating_sub(1)))
            {
                let timestamp = candle.timestamp.value();
                let time_str = format_time_label_for_span(timestamp, span_ms);
                let position_percent = (i as f64 / (num_labels as f64 - 1.0)) * 100.0;
                labels.push((time_str, position_percent));
            }
        }

        labels
    };

    view! {
        <div class="time-scale" aria-label="Visible time range">
            <For
                each=time_labels
                key=|(time, _pos)| time.clone()
                children=|(time, _position)| view! {
                    <span>{time}</span>
                }
            />
        </div>
    }
}

fn sync_chart_view(chart: RwSignal<Chart>) -> usize {
    chart.with_untracked(|current| {
        let Some(series) = current.get_series(current_interval().get_untracked()) else {
            return 0;
        };
        let candles = series.get_candles();
        let (zoom, pan) = viewport_zoom_pan(candles, &current.viewport);
        let start = visible_range(candles.len(), zoom, pan).0;

        let _ = with_global_renderer(|renderer| {
            renderer.set_zoom_params(zoom, pan);
            let _ = renderer.render(current);
        });

        start
    })
}

fn zoom_chart(chart: RwSignal<Chart>, factor: f64, center_x: f32) -> usize {
    chart.update(|current| {
        let Some(series) = current.get_series(current_interval().get_untracked()) else {
            return;
        };
        let candles = series.get_candles();
        if candles.is_empty() || !factor.is_finite() || factor <= 0.0 {
            return;
        }

        let candle_vec: Vec<_> = candles.iter().cloned().collect();
        let (old_zoom, _) = viewport_zoom_pan(candles, &current.viewport);
        let (start, visible) = visible_range_by_time(&candle_vec, &current.viewport, old_zoom);
        let max_visible = (MAX_VISIBLE_CANDLES / MIN_ZOOM_LEVEL) as usize;
        let min_visible = (MAX_VISIBLE_CANDLES / MAX_ZOOM_LEVEL).ceil() as usize;
        let target_visible = ((visible as f64 / factor).round() as usize)
            .clamp(min_visible, max_visible.min(candles.len()));
        let anchor = center_x.clamp(0.0, 1.0) as f64;
        let anchor_index = start as f64 + visible.saturating_sub(1) as f64 * anchor;
        let desired_start =
            (anchor_index - target_visible.saturating_sub(1) as f64 * anchor).round() as isize;
        let max_start = candles.len().saturating_sub(target_visible) as isize;
        let new_start = desired_start.clamp(0, max_start) as usize;
        let new_end = new_start + target_visible - 1;
        let start_time = candles[new_start].timestamp.value() as f64;
        let end_time = candles[new_end].timestamp.value() as f64;
        current.viewport.start_time = start_time;
        current.viewport.end_time = end_time;
    });
    sync_chart_view(chart)
}

fn pan_chart(chart: RwSignal<Chart>, delta_x: f64, canvas_width: f64) -> usize {
    let ratio = pan_ratio_from_pixels(delta_x, canvas_width);
    if ratio != 0.0 {
        chart.update(|current| {
            let Some(series) = current.get_series(current_interval().get_untracked()) else {
                return;
            };
            let candles = series.get_candles();
            if candles.is_empty() {
                return;
            }
            let candle_vec: Vec<_> = candles.iter().cloned().collect();
            let (zoom, _) = viewport_zoom_pan(candles, &current.viewport);
            let (start, visible) = visible_range_by_time(&candle_vec, &current.viewport, zoom);
            let shift = (visible as f64 * ratio as f64).round() as isize;
            let max_start = candles.len().saturating_sub(visible) as isize;
            let new_start = (start as isize + shift).clamp(0, max_start) as usize;
            let new_end = new_start + visible.saturating_sub(1);
            let start_time = candles[new_start].timestamp.value() as f64;
            let end_time = candles[new_end].timestamp.value() as f64;
            current.viewport.start_time = start_time;
            current.viewport.end_time = end_time;
        });
    }
    sync_chart_view(chart)
}

fn reset_chart(chart: RwSignal<Chart>) {
    chart.update(|current| {
        current.update_viewport_for_data();
        let interval = current_interval().get_untracked();
        let Some(series) = current.get_series(interval) else {
            return;
        };
        let candles = series.get_candles();
        if candles.is_empty() {
            return;
        }

        let start = candles.len().saturating_sub(DEFAULT_VISIBLE_CANDLES);
        let start_time = candles[start].timestamp.value() as f64;
        let end_time = candles.back().unwrap().timestamp.value() as f64;
        current.viewport.start_time = start_time;
        current.viewport.end_time = end_time;
    });
    let _ = sync_chart_view(chart);
}

/// 🎨 Container for the WebGPU chart
#[component]
fn ChartContainer() -> impl IntoView {
    ensure_chart(&current_symbol().get_untracked());
    let chart_memo = create_memo(move |_| {
        let sym = current_symbol().get();
        get_chart_signal(&sym).unwrap_or_else(|| ensure_chart(&sym))
    });
    let chart = move || chart_memo.get_untracked();
    let (_renderer, set_renderer) = create_signal::<Option<Rc<RefCell<WebGpuRenderer>>>>(None);
    let (status, set_status) = create_signal("Initializing...".to_string());

    // Reference to the canvas element
    let canvas_ref = create_node_ref::<Canvas>();
    let (initialized, set_initialized) = create_signal(false);

    // Initialize WebGPU once the canvas is available
    create_effect(move |_| {
        if initialized.get() {
            return;
        }

        if let Some(canvas) = canvas_ref.get() {
            let canvas_id = std::ops::Deref::deref(&canvas).id();
            set_initialized.set(true);
            let _ = spawn_local_with_current_owner(async move {
                web_sys::console::log_1(&"🔍 Canvas found, starting WebGPU init...".into());
                set_status.set("🚀 Initializing WebGPU renderer...".to_string());

                // Detailed WebGPU diagnostics
                web_sys::console::log_1(&"🏗️ Creating WebGPU renderer...".into());
                get_logger().info(
                    LogComponent::Infrastructure("WebGPU"),
                    "🔍 Starting WebGPU initialization...",
                );

                web_sys::console::log_1(&"⚡ About to call WebGpuRenderer::new...".into());

                match WebGpuRenderer::new(
                    canvas_id.as_str(),
                    CHART_WIDTH_PX as u32,
                    CHART_HEIGHT_PX as u32,
                )
                .await
                {
                    Ok(webgpu_renderer) => {
                        get_logger().info(
                            LogComponent::Infrastructure("WebGPU"),
                            "✅ WebGPU renderer created successfully",
                        );

                        let renderer_rc = Rc::new(RefCell::new(webgpu_renderer));
                        set_renderer.set(Some(renderer_rc.clone()));
                        set_global_renderer(renderer_rc.clone());
                        init_render_queue();
                        let _ = renderer_rc.borrow().log_gpu_memory_usage();
                        set_status.set("✅ WebGPU renderer ready".to_string());

                        // Start WebSocket after the renderer is initialized
                        get_logger().info(
                            LogComponent::Infrastructure("WebSocket"),
                            "🌐 Starting WebSocket stream...",
                        );
                        start_websocket_stream(set_status).await;
                    }
                    Err(e) => {
                        let msg = e.as_string().unwrap_or_else(|| format!("{e:?}"));
                        web_sys::console::error_1(
                            &format!("❌ WebGPU initialization error: {msg}").into(),
                        );
                        get_logger().error(
                            LogComponent::Infrastructure("WebGPU"),
                            &format!("❌ WebGPU initialization failed: {msg}"),
                        );
                        set_status.set(format!(
                            "❌ WebGPU failed: {msg}\n💡 Try Chrome Canary with --enable-unsafe-webgpu flag",
                        ));

                        // Fallback: show data even without the chart
                        get_logger().info(
                            LogComponent::Infrastructure("Fallback"),
                            "🔄 Starting fallback mode without WebGPU...",
                        );

                        // Generate sample data for demo purposes
                        let mut test_candles = Vec::new();
                        let base_price = 90000.0;
                        let base_time = js_sys::Date::now() as u64;

                        for i in 0..50 {
                            let price_variation = (i as f64 * 0.1).sin() * 1000.0;
                            let open = base_price + price_variation;
                            let close = open + (i as f64 % 3.0 - 1.0) * 200.0;
                            let high = open.max(close) + 100.0;
                            let low = open.min(close) - 100.0;
                            let volume = 100.0 + (i as f64 * 0.2).cos() * 50.0;

                            let candle = Candle::new(
                                crate::domain::market_data::Timestamp::from(base_time + i * 60000),
                                crate::domain::market_data::OHLCV::new(
                                    crate::domain::market_data::Price::from(open),
                                    crate::domain::market_data::Price::from(high),
                                    crate::domain::market_data::Price::from(low),
                                    crate::domain::market_data::Price::from(close),
                                    crate::domain::market_data::Volume::from(volume),
                                ),
                            );
                            test_candles.push(candle);
                        }

                        chart().update(|ch| ch.set_historical_data(test_candles));
                        set_status.set(format!(
                            "🎯 Demo mode: Using test data (WebSocket disabled)\nReason: {msg}",
                        ));
                    }
                }
            });
        }
    });

    // 🎯 Mouse events for the tooltip
    let handle_mouse_move = {
        let chart_signal = chart;
        let status_clone = set_status;
        move |event: web_sys::MouseEvent| {
            let mouse_x = event.offset_x() as f64;
            let mouse_y = event.offset_y() as f64;
            let canvas_width = event
                .target()
                .and_then(|target| target.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                .map(|canvas| canvas.client_width().max(1) as f64)
                .unwrap_or(CHART_WIDTH_PX);
            let scaled_x = mouse_x * CHART_WIDTH_PX / canvas_width;

            let dragging = is_dragging().get_untracked();
            if dragging {
                let last_x = last_mouse_x().get_untracked();
                let delta_x = mouse_x - last_x;
                last_mouse_x().set(mouse_x);
                let need_history = pan_chart(chart_signal(), delta_x, canvas_width);
                if should_fetch_history(need_history) {
                    fetch_more_history(status_clone);
                }
            } else {
                let ndc_x = (scaled_x / CHART_WIDTH_PX) * 2.0 - 1.0;

                chart_signal().with_untracked(|ch| {
                    let interval = current_interval().get_untracked();
                    let candles = ch.get_series(interval).unwrap().get_candles();
                    if !candles.is_empty() {
                        let (zoom, pan) = viewport_zoom_pan(candles, &ch.viewport);
                        let (start_idx, visible_count) = visible_range(candles.len(), zoom, pan);
                        let visible: Vec<_> =
                            candles.iter().skip(start_idx).take(visible_count).collect();

                        // Use the same logic as in candle_x_position
                        let step_size = 2.0 / visible.len() as f64;
                        let spacing = spacing_ratio_for(visible.len()) as f64;
                        let width = (step_size * (1.0 - spacing))
                            .clamp(MIN_ELEMENT_WIDTH as f64, MAX_ELEMENT_WIDTH as f64);
                        let half_width = width / 2.0;
                        // Inverse formula matching candle_x_position
                        // index = visible_len - 1 - (1.0 - EDGE_GAP as f64 - half_width - ndc_x) / step_size
                        let index_float = visible.len() as f64
                            - 1.0
                            - (1.0 - EDGE_GAP as f64 - half_width - ndc_x) / step_size;
                        let candle_idx = index_float.round() as i32;

                        if candle_idx >= 0 && (candle_idx as usize) < visible.len() {
                            let candle = visible[candle_idx as usize];
                            let data = TooltipData::new(candle.clone(), mouse_x, mouse_y);

                            tooltip_data().set(Some(data));
                            tooltip_visible().set(true);
                        } else {
                            tooltip_visible().set(false);
                        }
                    } else {
                        tooltip_visible().set(false);
                    }
                });
            }
        }
    };

    let handle_mouse_leave = move |_event: web_sys::MouseEvent| {
        tooltip_visible().set(false);
        is_dragging().set(false);
    };

    // 🔍 Mouse wheel zoom - simplified without effects
    let handle_wheel = {
        let chart_signal = chart;
        let status_clone = set_status;
        move |event: web_sys::WheelEvent| {
            if chart_signal().try_get_untracked().is_none() {
                return;
            }
            let Some(canvas) = event
                .target()
                .and_then(|target| target.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                .filter(|canvas| canvas.id() == "chart-canvas")
            else {
                return;
            };
            event.prevent_default();

            let delta_y = event.delta_y();
            if delta_y == 0.0 {
                return;
            }
            let canvas_width = canvas.client_width().max(1) as f64;
            let cursor_ratio = (event.offset_x() as f64 / canvas_width).clamp(0.0, 1.0) as f32;
            let factor = if delta_y < 0.0 { ZOOM_STEP } else { 1.0 / ZOOM_STEP };
            let start_idx = zoom_chart(chart_signal(), factor, cursor_ratio);
            if should_fetch_history(start_idx) {
                fetch_more_history(status_clone);
            }
            get_logger().info(LogComponent::Presentation("ChartZoom"), "🔍 Zoom applied");
        }
    };

    // 🖱️ Start panning
    let handle_mouse_down = move |event: web_sys::MouseEvent| {
        if event.button() == 0 {
            // Left mouse button
            web_sys::console::log_1(&"🖱️ Mouse down".into());
            is_dragging().set(true);
            last_mouse_x().set(event.offset_x() as f64);

            // Give the canvas focus for keyboard events
            if let Some(target) = event.target()
                && let Ok(canvas) = target.dyn_into::<web_sys::HtmlCanvasElement>()
            {
                let _ = canvas.focus();
            }
        }
    };

    // 🖱️ End panning
    let handle_mouse_up = move |_event: web_sys::MouseEvent| {
        web_sys::console::log_1(&"🖱️ Mouse up".into());
        is_dragging().set(false);
    };

    // ⌨️ Zoom keys (+/- and PageUp/PageDown)
    let handle_keydown = {
        let chart_signal = chart;
        let status_clone = set_status;
        move |event: web_sys::KeyboardEvent| {
            let key = event.key();

            let factor = match key.as_str() {
                "+" | "=" => {
                    event.prevent_default();
                    Some(1.2)
                }
                "-" | "_" => {
                    event.prevent_default();
                    Some(0.8)
                }
                "PageUp" => {
                    event.prevent_default();
                    Some(1.5)
                }
                "PageDown" => {
                    event.prevent_default();
                    Some(0.67)
                }
                _ => None,
            };
            if let Some(factor) = factor {
                let start_idx = zoom_chart(chart_signal(), factor, 0.5);
                get_logger()
                    .info(LogComponent::Presentation("KeyboardZoom"), "Keyboard zoom applied");
                if should_fetch_history(start_idx) {
                    fetch_more_history(status_clone);
                }
            }
        }
    };

    // Attach wheel event listener to the window
    let wheel_listener = window_event_listener_with_options(
        ev::wheel,
        &EventOptions { passive: false, capture: false, once: false },
        handle_wheel,
    );
    on_cleanup(move || wheel_listener.remove());

    // Reset dragging state when the mouse is released anywhere
    let mouseup_listener =
        window_event_listener_with_options(ev::mouseup, &EventOptions::default(), move |_| {
            is_dragging().set(false)
        });
    on_cleanup(move || mouseup_listener.remove());

    let handle_zoom_in = move |_| {
        let start = zoom_chart(chart(), ZOOM_STEP, 0.5);
        if should_fetch_history(start) {
            fetch_more_history(set_status);
        }
    };
    let handle_zoom_out = move |_| {
        let _ = zoom_chart(chart(), 1.0 / ZOOM_STEP, 0.5);
    };
    let handle_reset = move |_| reset_chart(chart());
    let handle_double_click = move |_| reset_chart(chart());

    view! {
        <div class="chart-container">
            <div class="chart-toolbar">
                <div class="toolbar-cluster">
                    <div class="selector-group" aria-label="Market">
                        <span class="selector-label">"Market"</span>
                        <AssetSelector set_status=set_status />
                    </div>
                    <div class="selector-group" aria-label="Timeframe">
                        <span class="selector-label">"Timeframe"</span>
                        <TimeframeSelector chart=chart() set_status=set_status />
                    </div>
                </div>
                <div class="chart-actions" aria-label="Chart controls">
                    <button
                        class="chart-action"
                        aria-label="Zoom out"
                        title="Zoom out"
                        on:click=handle_zoom_out
                    >"−"</button>
                    <button
                        class="chart-action"
                        aria-label="Zoom in"
                        title="Zoom in"
                        on:click=handle_zoom_in
                    >"+"</button>
                    <button
                        class="chart-action reset"
                        title="Reset chart to live data"
                        on:click=handle_reset
                    >"Live view"</button>
                </div>
            </div>

            <div class="chart-workspace">
                <div class="chart-viewport">
                    <Show when=move || loading_more().get()>
                        <div class="loading-badge">"Loading history…"</div>
                    </Show>
                    <canvas
                        id="chart-canvas"
                        node_ref=canvas_ref
                        width="800"
                        height="500"
                        tabindex="0"
                        aria-label="Interactive candlestick chart. Use the mouse wheel to zoom and drag to pan."
                        on:mousemove=handle_mouse_move
                        on:mouseleave=handle_mouse_leave
                        on:mousedown=handle_mouse_down
                        on:mouseup=handle_mouse_up
                        on:dblclick=handle_double_click
                        on:keydown=handle_keydown
                    />
                    <PriceScale chart=chart() />
                    <ChartTooltip />
                </div>
                <TimeScale chart=chart() />
            </div>

            <Legend chart=chart() />

            <div class="chart-footer">
                <div class="status" role="status">{move || status.get()}</div>
                <div class="control-hints">
                    "Wheel / + − to zoom · Drag to pan · Double-click to reset"
                </div>
            </div>
        </div>
    }
}

/// 💰 Price scale on the right side of the chart
fn visible_price_bounds(chart: &Chart) -> Option<(f64, f64)> {
    let interval = current_interval().get_untracked();
    let series = chart.get_series(interval)?;
    let candle_vec: Vec<_> = series.get_candles().iter().cloned().collect();
    let (zoom, _) = viewport_zoom_pan(series.get_candles(), &chart.viewport);
    let (start, visible) = visible_range_by_time(&candle_vec, &chart.viewport, zoom);
    let mut range = candle_vec.iter().skip(start).take(visible);
    let first = range.next()?;
    let mut min_price = first.ohlcv.low.value();
    let mut max_price = first.ohlcv.high.value();
    for candle in range {
        min_price = min_price.min(candle.ohlcv.low.value());
        max_price = max_price.max(candle.ohlcv.high.value());
    }
    let padding = ((max_price - min_price).abs().max(1e-6)) * 0.05;
    Some((min_price - padding, max_price + padding))
}

#[component]
fn PriceScale(chart: RwSignal<Chart>) -> impl IntoView {
    let current_price = global_current_price();

    let price_levels = move || {
        let Some((min_price, max_price)) = chart.with(visible_price_bounds) else {
            return Vec::new();
        };
        let step = 100.0 / 8.0;
        (0..=8)
            .map(|i| {
                let position = i as f64 * step;
                let price = max_price - (max_price - min_price) * position / 100.0;
                (price, position)
            })
            .collect::<Vec<_>>()
    };
    let current_price_position = move || {
        chart.with(|current| {
            let Some((min_price, max_price)) = visible_price_bounds(current) else {
                return 50.0;
            };
            ((max_price - current_price.get()) / (max_price - min_price) * 100.0).clamp(0.0, 100.0)
        })
    };

    view! {
        <div class="price-scale">
            // Display price levels
            <For
                each=price_levels
                key=|(_price, pos)| (*pos * 100.0) as i64
                children=|(price, position)| view! {
                    <div
                        class="price-level"
                        style:top=format!("{}%", position)
                    >
                        {format!("{price:.2}")}
                    </div>
                }
            />

            // Display the current price (highlighted)
            <div
                class="current-price-label"
                style:top=move || format!("{}%", current_price_position())
            >
                <span class="price-value">{move || format!("${:.2}", current_price.get())}</span>
            </div>
        </div>
    }
}

/// 🎯 Chart Tooltip component inside the chart wrapper
#[component]
fn ChartTooltip() -> impl IntoView {
    let tooltip_visible = tooltip_visible();
    let tooltip_data = tooltip_data();

    view! {
        <div
            class="tooltip"
            style:display=move || if tooltip_visible.get() { "block" } else { "none" }
            style:left=move || {
                tooltip_data.with(|data| {
                    if let Some(tooltip) = data {
                        format!("{}px", tooltip.x)
                    } else {
                        "0px".to_string()
                    }
                })
            }
            style:top=move || {
                tooltip_data.with(|data| {
                    if let Some(tooltip) = data {
                        format!("{}px", tooltip.y)
                    } else {
                        "0px".to_string()
                    }
                })
            }
        >
            {move || {
                tooltip_data.with(|data| {
                    if let Some(tooltip) = data {
                        tooltip.formatted_text.clone()
                    } else {
                        String::new()
                    }
                })
            }}
        </div>
    }
}

#[component]
fn TimeframeSelector(chart: RwSignal<Chart>, set_status: WriteSignal<String>) -> impl IntoView {
    let options = vec![
        TimeInterval::TwoSeconds,
        TimeInterval::OneMinute,
        TimeInterval::FiveMinutes,
        TimeInterval::FifteenMinutes,
        TimeInterval::OneHour,
        TimeInterval::OneDay,
        TimeInterval::OneWeek,
        TimeInterval::OneMonth,
    ];

    view! {
        <div class="selector-group">
            <For
                each=move || options.clone()
                key=|i| i.as_ref().to_string()
                children=move |interval| {
                    let label = interval.as_ref().to_string();
                    let chart_signal = chart;
                    let status_signal = set_status;
                    view! {
                        <button
                            class=move || if current_interval().get() == interval {
                                "selector-button active"
                            } else {
                                "selector-button"
                            }
                            on:click=move |_| {
                                current_interval().set(interval);
                                if let Some(handle) = stream_abort_handles()
                                    .with(|m| m.get(&current_symbol().get_untracked()).cloned())
                                {
                                    handle.abort();
                                    stream_abort_handles().update(|m| {
                                        m.remove(&current_symbol().get_untracked());
                                    });
                                }
                                let status = status_signal;
                                let _ = spawn_local_with_current_owner(async move {
                                    start_websocket_stream(status).await;
                                });
                                reset_chart(chart_signal);
                            }
                        >
                            {label}
                        </button>
                    }
                }
            />
        </div>
    }
}

#[component]
fn LegendIndicatorToggle(name: &'static str, chart: RwSignal<Chart>) -> impl IntoView {
    let id = name;
    let label = name.to_uppercase();
    let checked = move || {
        global_line_visibility().with(|v| match name {
            "sma20" => v.sma_20,
            "sma50" => v.sma_50,
            "sma200" => v.sma_200,
            "ema12" => v.ema_12,
            "ema26" => v.ema_26,
            _ => true,
        })
    };
    let color = match name {
        "sma20" => "#e16c48",
        "sma50" => "#ffff00",
        "sma200" => "#6f9fff",
        "ema12" => "#bb86fc",
        "ema26" => "#59d5e0",
        _ => "#ffffff",
    };
    view! {
        <label class="indicator-toggle" style=format!("--indicator-color: {color}")>
            <input
                type="checkbox"
                id=id
                prop:checked=checked
                on:change=move |_| {
                    chart.with_untracked(|c| {
                        if with_global_renderer(|r| {
                            r.toggle_line_visibility(name);
                            let _ = r.render(c);
                        }).is_none() {
                            // renderer not available
                        }
                    });
                }
            />
            <span class="indicator-dot" aria-hidden="true"></span>
            {label}
        </label>
    }
}

#[component]
fn Legend(chart: RwSignal<Chart>) -> impl IntoView {
    let names = vec!["sma20", "sma50", "sma200", "ema12", "ema26"];
    view! {
        <div class="indicator-bar">
            <span class="indicator-title">"Indicators"</span>
            <For
                each=move || names.clone()
                key=|name| name.to_string()
                children=move |name| view! { <LegendIndicatorToggle name=name chart=chart /> }
            />
        </div>
    }
}

#[component]
fn AssetSelector(set_status: WriteSignal<String>) -> impl IntoView {
    let options = default_symbols();

    view! {
        <div class="selector-group">
            <For
                each=move || options.clone()
                key=|s: &Symbol| s.value().to_string()
                children=move |sym: Symbol| {
                    let label = sym.value().to_string();
                    let status_cloned = set_status;
                    let selected_symbol = sym.clone();
                    let click_symbol = sym.clone();
                    view! {
                        <button
                            class=move || if current_symbol().get() == selected_symbol {
                                "selector-button active"
                            } else {
                                "selector-button"
                            }
                            on:click=move |_| {
                                ensure_chart(&click_symbol);
                                current_symbol().set(click_symbol.clone());
                                let _ = spawn_local_with_current_owner(async move {
                                    start_websocket_stream(status_cloned).await;
                                });
                            }
                        >
                            {label}
                        </button>
                    }
                }
            />
        </div>
    }
}

/// Abort all active streams except the one for `symbol`.
pub fn abort_other_streams(symbol: &Symbol) {
    stream_abort_handles().update(|m| {
        m.retain(|sym, handle| {
            if sym != symbol {
                handle.abort();
                false
            } else {
                true
            }
        });
    });
}

/// 🌐 Start WebSocket stream in Leptos and update global signals
pub async fn start_websocket_stream(set_status: WriteSignal<String>) {
    let symbol = current_symbol().get_untracked();
    abort_other_streams(&symbol);
    ensure_chart(&symbol);
    let chart = get_chart_signal(&symbol).unwrap();

    if let Some(handle) = stream_abort_handles().with(|m| m.get(&symbol).cloned()) {
        handle.abort();
        stream_abort_handles().update(|m| {
            m.remove(&symbol);
        });
        set_status.set("🔄 Restarting stream".to_string());
    }

    let interval = current_interval().get_untracked();
    let conn_id = connection_id().get_untracked() + 1;
    connection_id().set(conn_id);
    domain_state().update(|ds| {
        ds.timeframe = Duration::from_millis(interval.duration_ms());
        ds.candles = Arc::new(Vec::new());
        ds.indicators = Arc::new(Vec::new());
    });

    let rest_client_arc =
        Arc::new(Mutex::new(BinanceWebSocketClient::new(symbol.clone(), interval)));

    // Set the streaming status
    global_is_streaming().set(false);

    // 📈 First load historical data
    set_status.set("📈 Loading historical data...".to_string());

    let hist_res = {
        let client = rest_client_arc.lock().await;
        client.fetch_historical_data(1000).await
    };
    if conn_id != connection_id().get_untracked() {
        return;
    }
    match hist_res {
        Ok(historical_candles) => {
            get_logger().info(
                LogComponent::Presentation("WebSocketStream"),
                &format!("✅ Loaded {} historical candles", historical_candles.len()),
            );

            chart.update(|ch| ch.set_historical_data(historical_candles.clone()));
            reset_chart(chart);
            domain_state().update(|ds| {
                ds.candles = Arc::new(historical_candles.clone());
                ds.indicators = Arc::new(Vec::new());
            });
            // Update global signals using the historical data
            let cnt = chart.with(|c| c.get_candle_count());
            global_candle_count().set(cnt);

            if let Some(last_candle) = historical_candles.last() {
                global_current_price().set(last_candle.ohlcv.close.value());
            }

            // Compute the maximum volume from history
            let max_vol = historical_candles
                .iter()
                .map(|c| c.ohlcv.volume.value())
                .fold(0.0f64, |a, b| a.max(b));
            global_max_volume().set(max_vol);

            set_status.set("✅ Historical data loaded. Starting real-time stream...".to_string());
        }
        Err(e) => {
            get_logger().error(
                LogComponent::Presentation("WebSocketStream"),
                &format!("❌ Failed to load historical data: {e}"),
            );
            set_status.set("⚠️ Historical data failed. Starting real-time only...".to_string());
        }
    }

    // 🔌 Start the WebSocket for real-time updates
    set_status.set("🔌 Starting WebSocket stream...".to_string());
    global_is_streaming().set(true);

    let stream_client_arc =
        Arc::new(Mutex::new(BinanceWebSocketClient::new(symbol.clone(), interval)));
    let (abort_handle, abort_reg) = futures::future::AbortHandle::new_pair();
    let (done_tx, done_rx) = oneshot::channel::<()>();
    stream_abort_handles().update(|m| {
        m.insert(symbol.clone(), abort_handle.clone());
    });
    on_cleanup({
        let symbol = symbol.clone();
        let handle = abort_handle.clone();
        let done_rx = done_rx;
        move || {
            handle.abort();
            let _ = spawn_local_with_current_owner(async move {
                let _ = done_rx.await;
                stream_abort_handles().update(|m| {
                    m.remove(&symbol);
                });
            });
        }
    });
    let handle_check = abort_handle.clone();
    let fut = futures::future::Abortable::new(
        async move {
            let handler_handle = handle_check.clone();
            let connection_guard = conn_id;
            let handler = move |candle: Candle| {
                if handler_handle.is_aborted()
                    || connection_guard != connection_id().get_untracked()
                {
                    return;
                }
                global_current_price().set(candle.ohlcv.close.value());

                chart.update(|ch| {
                    let interval = current_interval().get_untracked();
                    let (was_at_live_edge, visible_before) = ch
                        .get_series(interval)
                        .map(|series| {
                            let candles = series.get_candles();
                            let (zoom, pan) = viewport_zoom_pan(candles, &ch.viewport);
                            let (start, visible) = visible_range(candles.len(), zoom, pan);
                            (start + visible >= candles.len(), visible)
                        })
                        .unwrap_or((true, DEFAULT_VISIBLE_CANDLES));

                    ch.add_realtime_candle(candle.clone());
                    if was_at_live_edge {
                        let series = ch.get_series(interval).unwrap();
                        let candles = series.get_candles();
                        let start = candles.len().saturating_sub(visible_before.max(1));
                        let start_time = candles[start].timestamp.value() as f64;
                        let end_time = candles.back().unwrap().timestamp.value() as f64;
                        ch.viewport.start_time = start_time;
                        ch.viewport.end_time = end_time;
                    }
                });
                domain_state().update(|ds| {
                    let mut v = (*ds.candles).clone();
                    v.push(candle.clone());
                    ds.candles = Arc::new(v);
                });

                let count = chart.with(|c| c.get_candle_count());
                global_candle_count().set(count);

                let max_vol = chart.with(|c| {
                    c.get_series(interval)
                        .unwrap()
                        .get_candles()
                        .iter()
                        .map(|c| c.ohlcv.volume.value())
                        .fold(0.0f64, |a, b| a.max(b))
                });
                global_max_volume().set(max_vol);

                let sym_for_queue = symbol.clone();
                enqueue_render_task(Box::new(move |r| {
                    let chart_signal = get_chart_signal(&sym_for_queue).unwrap();
                    chart_signal.with_untracked(|ch| {
                        if ch.get_candle_count() > 0 {
                            let interval = current_interval().get_untracked();
                            let series = ch.get_series(interval).unwrap();
                            let (zoom, pan) = viewport_zoom_pan(series.get_candles(), &ch.viewport);
                            r.set_zoom_params(zoom, pan);
                            let _ = r.render(ch);
                        }
                    });
                }));

                if handler_handle.is_aborted() {
                    return;
                }
                set_status.set("🌐 WebSocket LIVE • Real-time updates".to_string());
            };

            let result = {
                let mut client = stream_client_arc.lock().await;
                client.start_stream(handler).await
            };
            if handle_check.is_aborted() {
                return;
            }
            if let Err(e) = result {
                if handle_check.is_aborted() {
                    return;
                }
                set_status.set(format!("❌ WebSocket error: {e}"));
                global_is_streaming().set(false);
            }
        },
        abort_reg,
    );

    let _ = spawn_local_with_current_owner(async move {
        let _ = fut.await;
        let _ = done_tx.send(());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chart::value_objects::ChartType;
    use crate::domain::market_data::value_objects::Symbol;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn setup_container() -> web_sys::HtmlElement {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let div =
            document.create_element("div").unwrap().dyn_into::<web_sys::HtmlElement>().unwrap();
        document.body().unwrap().append_child(&div).unwrap();
        div
    }

    fn find_button(
        container: &web_sys::HtmlElement,
        label: &str,
    ) -> Result<web_sys::HtmlElement, String> {
        let buttons = container.get_elements_by_tag_name("button");
        for i in 0..buttons.length() {
            let btn = buttons
                .item(i)
                .ok_or_else(|| format!("button index {i} missing"))?
                .dyn_into::<web_sys::HtmlElement>()
                .map_err(|_| format!("element at index {i} is not an HtmlElement"))?;
            if btn.text_content().unwrap_or_default() == label {
                return Ok(btn);
            }
        }
        Err(format!("button with label {label} not found"))
    }

    fn find_checkbox(
        container: &web_sys::HtmlElement,
        id: &str,
    ) -> Result<web_sys::HtmlInputElement, String> {
        let elem = container
            .query_selector(&format!("#{id}"))
            .map_err(|e| format!("selector error for #{id}: {e:?}"))?
            .ok_or_else(|| format!("checkbox with id {id} not found"))?;
        elem.dyn_into::<web_sys::HtmlInputElement>()
            .map_err(|_| format!("element with id {id} is not an HtmlInputElement"))
    }

    #[wasm_bindgen_test(async)]
    async fn timeframe_buttons_update_interval() {
        use gloo_timers::future::sleep;
        use std::time::Duration;

        let container = setup_container();
        let chart = create_rw_signal(Chart::new("test".to_string(), ChartType::Candlestick, 100));
        let (_, set_status) = create_signal(String::new());
        leptos::mount_to(
            container.clone(),
            move || view! { <TimeframeSelector chart=chart set_status=set_status /> },
        );

        let two_sec = find_button(&container, "2s").expect("2s button not found");
        two_sec.click();
        sleep(Duration::from_millis(10)).await;
        abort_other_streams(&current_symbol().get_untracked());
        assert_eq!(current_interval().get(), TimeInterval::TwoSeconds);

        let five = find_button(&container, "5m").expect("5m button not found");
        five.click();
        sleep(Duration::from_millis(10)).await;
        abort_other_streams(&current_symbol().get_untracked());
        assert_eq!(current_interval().get(), TimeInterval::FiveMinutes);

        let fifteen = find_button(&container, "15m").expect("15m button not found");
        fifteen.click();
        sleep(Duration::from_millis(10)).await;
        abort_other_streams(&current_symbol().get_untracked());
        assert_eq!(current_interval().get(), TimeInterval::FifteenMinutes);

        let one_hour = find_button(&container, "1h").expect("1h button not found");
        one_hour.click();
        sleep(Duration::from_millis(10)).await;
        abort_other_streams(&current_symbol().get_untracked());
        assert_eq!(current_interval().get(), TimeInterval::OneHour);
    }

    #[wasm_bindgen_test]
    fn legend_checkbox_toggles_visibility() {
        use crate::infrastructure::rendering::renderer::{dummy_renderer, set_global_renderer};
        use std::cell::RefCell;
        use std::rc::Rc;

        let container = setup_container();
        let chart = create_rw_signal(Chart::new("test".to_string(), ChartType::Candlestick, 10));
        let renderer = Rc::new(RefCell::new(dummy_renderer()));

        set_global_renderer(renderer.clone());
        leptos::mount_to(container.clone(), move || view! { <Legend chart=chart /> });

        let cb = find_checkbox(&container, "sma20").expect("sma20 checkbox not found");
        cb.click();

        assert!(!renderer.borrow().line_visibility().sma_20);
    }

    #[wasm_bindgen_test]
    fn legend_checkbox_updates_on_renderer_change() {
        use crate::infrastructure::rendering::renderer::{dummy_renderer, set_global_renderer};
        use std::cell::RefCell;
        use std::rc::Rc;

        let container = setup_container();
        let chart = create_rw_signal(Chart::new("test".to_string(), ChartType::Candlestick, 10));
        let renderer = Rc::new(RefCell::new(dummy_renderer()));

        set_global_renderer(renderer.clone());
        leptos::mount_to(container.clone(), move || view! { <Legend chart=chart /> });

        let cb = find_checkbox(&container, "sma20").expect("sma20 checkbox not found");
        assert!(cb.checked());

        renderer.borrow_mut().toggle_line_visibility("sma20");

        assert!(!cb.checked());
    }

    #[test]
    fn zoom_limits_respected_by_visible_range() {
        let (_, visible_min_zoom) = visible_range(1000, MIN_ZOOM_LEVEL, 0.0);
        assert!(visible_min_zoom <= 300);

        let (_, visible_max_zoom) = visible_range(1000, MAX_ZOOM_LEVEL, 0.0);
        assert!(visible_max_zoom as f64 >= MIN_VISIBLE_CANDLES);
    }

    #[test]
    fn drag_distance_maps_to_inverted_viewport_fraction() {
        assert!((pan_ratio_from_pixels(80.0, 800.0) + 0.1).abs() < f32::EPSILON);
        assert!((pan_ratio_from_pixels(-200.0, 800.0) - 0.25).abs() < f32::EPSILON);
        assert_eq!(pan_ratio_from_pixels(10.0, 0.0), 0.0);
        assert_eq!(pan_ratio_from_pixels(f64::NAN, 800.0), 0.0);
    }

    #[wasm_bindgen_test]
    fn timeframe_selector_exposes_long_ranges() {
        let container = setup_container();
        let chart = create_rw_signal(Chart::new("test".to_string(), ChartType::Candlestick, 100));
        let (_, set_status) = create_signal(String::new());
        leptos::mount_to(
            container.clone(),
            move || view! { <TimeframeSelector chart=chart set_status=set_status /> },
        );

        find_button(&container, "1d").expect("1d button not found");
        find_button(&container, "1w").expect("1w button not found");
        find_button(&container, "1M").expect("1M button not found");
    }

    #[wasm_bindgen_test]
    fn asset_buttons_update_current_symbol() {
        let container = setup_container();
        let (_status, set_status) = create_signal(String::new());
        leptos::mount_to(
            container.clone(),
            move || view! { <AssetSelector set_status=set_status /> },
        );

        let eth_btn = find_button(&container, "ETHUSDT").expect("ETHUSDT button not found");
        eth_btn.click();
        assert_eq!(current_symbol().get(), Symbol::from("ETHUSDT"));

        let btc_btn = find_button(&container, "BTCUSDT").expect("BTCUSDT button not found");
        btc_btn.click();
        assert_eq!(current_symbol().get(), Symbol::from("BTCUSDT"));
    }
}
