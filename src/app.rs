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
use wasm_bindgen::JsCast;

use crate::event_utils::{EventOptions, wheel_event_options, window_event_listener_with_options};
use crate::global_signals;
use crate::global_state::{ensure_chart, get_chart_signal, set_chart_in_ecs};
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
    infrastructure::{rendering::WebGpuRenderer, websocket::BinanceWebSocketClient},
    time_utils::format_time_label,
};

/// Maximum number of candles visible at 1x zoom
const MAX_VISIBLE_CANDLES: f64 = 32.0;
/// Minimum number of candles that must remain visible
const MIN_VISIBLE_CANDLES: f64 = 1.0;

/// Default canvas width
const CHART_WIDTH: f64 = 800.0;

/// Base factor for converting mouse movement to candle offset
pub const PAN_SENSITIVITY_BASE: f64 = MAX_VISIBLE_CANDLES / CHART_WIDTH;

/// Minimum allowed zoom level
const MIN_ZOOM_LEVEL: f64 = MAX_VISIBLE_CANDLES / 300.0;
/// Maximum allowed zoom level
const MAX_ZOOM_LEVEL: f64 = 32.0;

/// Pan offset required to trigger history loading
pub const HISTORY_FETCH_THRESHOLD: f64 = -50.0;

/// Number of candles kept in memory beyond the visible range
const HISTORY_BUFFER_SIZE: usize = 150;

/// Check if more historical data should be fetched
pub fn should_fetch_history(pan: f64) -> bool {
    pan <= HISTORY_FETCH_THRESHOLD
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
        let client_arc =
            Arc::new(Mutex::new(BinanceWebSocketClient::new(symbol.clone(), interval)));
        let visible = chart.with(|c| {
            let interval = current_interval().get_untracked();
            let series = c.get_series(interval).unwrap();
            let (zoom, pan) = viewport_zoom_pan(series.get_candles(), &c.viewport);
            let len = c.get_candle_count();
            visible_range(len, zoom, pan).1
        });
        let limit = (visible + HISTORY_BUFFER_SIZE) as u32;
        let result = {
            let client = client_arc.lock().await;
            client.fetch_historical_data_before(end_time, limit).await
        };
        match result {
            Ok(mut new_candles) => {
                new_candles.sort_by(|a, b| a.timestamp.value().cmp(&b.timestamp.value()));
                chart.update(|ch| {
                    for candle in new_candles.iter() {
                        ch.add_candle(candle.clone());
                    }
                });
                chart.with_untracked(|c| set_chart_in_ecs(&symbol, c.clone()));
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
            Err(e) => set_status.set(format!("❌ Failed to load more data: {}", e)),
        }

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
        let trend = if change >= 0.0 { "🟢" } else { "🔴" };

        // Format time from the timestamp
        let time_str = format!("Time: {}", candle.timestamp.value());

        let symbol = current_symbol().get_untracked();
        let formatted_text = format!(
            "{} {}\n📈 Open:   ${:.2}\n📊 High:   ${:.2}\n📉 Low:    ${:.2}\n💰 Close:  ${:.2}\n📈 Change: ${:.2} ({:.2}%)\n📊 Volume: {:.4}\n{}",
            trend,
            symbol.value(),
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

    view! {
        <style>
            {r#"
            .bitcoin-chart-app {
                font-family: 'SF Pro Display', -apple-system, BlinkMacSystemFont, sans-serif;
                background: linear-gradient(135deg, #1e3c72 0%, #2a5298 100%);
                min-height: 100vh;
                padding: 20px;
                color: white;
            }
            
            .header {
                text-align: center;
                margin-bottom: 20px;
                background: rgba(255, 255, 255, 0.1);
                backdrop-filter: blur(10px);
                padding: 20px;
                border-radius: 15px;
                border: 1px solid rgba(255, 255, 255, 0.2);
            }
            
            .price-info {
                display: flex;
                justify-content: center;
                gap: 40px;
                margin-top: 15px;
            }
            
            .price-item {
                text-align: center;
            }
            
            .price-value {
                font-size: 24px;
                font-weight: 700;
                color: #74c787;
                text-shadow: 0 0 10px rgba(114, 198, 133, 0.3);
            }
            
            .price-label {
                font-size: 12px;
                color: #a0a0a0;
                margin-top: 5px;
            }
            
            .chart-container {
                position: relative;
                display: flex;
                flex-direction: column;
                align-items: center;
                gap: 10px;
                margin-bottom: 20px;
            }
            
            .chart-wrapper {
                position: relative;
                display: inline-block;
            }
            
            .price-scale {
                position: absolute;
                right: -60px;
                top: 0;
                height: 100%;
                width: 80px;
                pointer-events: none;
            }
            
            .current-price-label {
                position: absolute;
                right: 0;
                transform: translateY(-50%);
                background: #f39c12;
                color: white;
                padding: 4px 8px;
                border-radius: 4px;
                font-size: 12px;
                font-weight: bold;
                white-space: nowrap;
                box-shadow: 0 2px 4px rgba(0,0,0,0.3);
            }
            
            .price-value {
                font-family: 'Courier New', monospace;
            }
            
            .tooltip {
                position: absolute;
                background: rgba(0, 0, 0, 0.9);
                color: white;
                padding: 8px 12px;
                border-radius: 6px;
                font-size: 12px;
                font-family: 'Courier New', monospace;
                white-space: pre-line;
                pointer-events: none;
                z-index: 1000;
                border: 1px solid #4a5d73;
                box-shadow: 0 4px 12px rgba(0, 0, 0, 0.5);
                backdrop-filter: blur(5px);
                line-height: 1.4;
                transform: translate(10px, -100%);
            }
            
            .status {
                color: #74c787;
                font-size: 14px;
                text-align: center;
            }
            

            "#}
        </style>
        <div class="bitcoin-chart-app">
            <Header />
            <ChartContainer />
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
        let chart = get_chart_signal(&current_symbol().get_untracked()).unwrap();
        chart.with(|c| {
            let interval = current_interval().get_untracked();
            let series = c.get_series(interval).unwrap();
            viewport_zoom_pan(series.get_candles(), &c.viewport).0
        })
    };

    view! {
        <div class="header">
            <h1>{move || format!("🌐 {} WebSocket Chart", current_symbol().get().value())}</h1>
            <p>{move || format!("{} • Real-time Leptos + WebGPU", current_symbol().get().value())}</p>

            <div class="price-info">
                <div class="price-item">
                    <div class="price-value">
                        {move || format!("${:.2}", current_price.get())}
                    </div>
                    <div class="price-label">"Current Price"</div>
                </div>
                <div class="price-item">
                    <div class="price-value">
                        {move || candle_count.get().to_string()}
                    </div>
                    <div class="price-label">"Candles"</div>
                </div>
                <div class="price-item">
                    <div class="price-value">
                        {move || if is_streaming.get() { "🟢 LIVE" } else { "🔴 OFF" }}
                    </div>
                    <div class="price-label">"WebSocket"</div>
                </div>
                <div class="price-item">
                    <div class="price-value">
                        {move || format!("{:.2}", max_volume.get())}
                    </div>
                    <div class="price-label">"Max Volume"</div>
                </div>
                <div class="price-item">
                    <div class="price-value">
                        {move || format!("{:.1}x", zoom_level())}
                    </div>
                    <div class="price-label">"🔍 Zoom"</div>
                </div>
            </div>
        </div>
    }
}

#[component]
fn PriceAxisLeft(chart: RwSignal<Chart>) -> impl IntoView {
    let labels = move || {
        let vp = chart.with(|c| c.viewport.clone());
        price_levels(&vp)
    };

    view! {
        <div style="width: 60px; height: 500px; background: #222; display: flex; flex-direction: column; justify-content: space-between; align-items: flex-end; margin-right: 8px;">
            <For
                each=labels
                key=|v| (*v * 100.0) as i64
                children=|v| view! {
                    <div style="font-size: 12px; color: #fff;">{format!("{:.2}", v)}</div>
                }
            />
        </div>
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

        let pan = chart.with(|c| viewport_zoom_pan(&candles, &c.viewport).1);
        let (start_idx, visible) = visible_range(candles.len(), zoom, pan);

        // Show 5 time labels
        let num_labels = 5;
        let mut labels = Vec::new();

        for i in 0..num_labels {
            let index = (i * visible) / (num_labels - 1);
            if let Some(candle) =
                candles.iter().skip(start_idx).nth(index.min(visible.saturating_sub(1)))
            {
                let timestamp = candle.timestamp.value();
                let time_str = format_time_label(timestamp, zoom);
                let position_percent = (i as f64 / (num_labels as f64 - 1.0)) * 100.0;
                labels.push((time_str, position_percent));
            }
        }

        labels
    };

    view! {
        <div style="width: 800px; height: 30px; background: #222; display: flex; align-items: center; justify-content: space-between; padding: 0 10px; margin-top: 5px; border-radius: 5px;">
            <For
                each=time_labels
                key=|(time, _pos)| time.clone()
                children=|(time, _position)| view! {
                    <div style="font-size: 11px; color: #888;">
                        {time}
                    </div>
                }
            />
        </div>
    }
}

/// 🎨 Container for the WebGPU chart
#[component]
fn ChartContainer() -> impl IntoView {
    ensure_chart(&current_symbol().get_untracked());
    create_effect(move |_| {
        let sym = current_symbol().get();
        ensure_chart(&sym);
    });
    let chart_memo = create_memo(move |_| {
        let sym = current_symbol().get();
        get_chart_signal(&sym).unwrap_or_else(|| ensure_chart(&sym))
    });
    let chart = move || chart_memo.get();
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

                match WebGpuRenderer::new(canvas_id.as_str(), 800, 500).await {
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
                        let symbol = current_symbol().get_untracked();
                        chart().with_untracked(|c| set_chart_in_ecs(&symbol, c.clone()));
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

            // 🔍 Handle panning
            let dragging = is_dragging().get_untracked();
            if dragging {
                let last_x = last_mouse_x().get_untracked();
                let delta_x = mouse_x - last_x;
                chart_signal().update(|ch| {
                    let factor_x = -(delta_x as f32) / ch.viewport.width as f32;
                    ch.pan(factor_x, 0.0);
                });
                let symbol = current_symbol().get_untracked();
                chart_signal().with_untracked(|c| set_chart_in_ecs(&symbol, c.clone()));
                last_mouse_x().set(mouse_x);
                let need_history = chart_signal().with_untracked(|c| {
                    let interval = current_interval().get_untracked();
                    let series = c.get_series(interval).unwrap();
                    let (_, pan) = viewport_zoom_pan(series.get_candles(), &c.viewport);
                    should_fetch_history(pan)
                });
                if need_history {
                    fetch_more_history(status_clone);
                }

                enqueue_render_task(Box::new(|r| {
                    let chart_signal = get_chart_signal(&current_symbol().get_untracked()).unwrap();
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
            } else {
                // Convert to NDC coordinates (assuming an 800x500 canvas)
                let canvas_width = 800.0;
                let canvas_height = 500.0;
                let ndc_x = (mouse_x / canvas_width) * 2.0 - 1.0;
                let _ndc_y = 1.0 - (mouse_y / canvas_height) * 2.0;

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
            web_sys::console::log_1(&format!("🖱️ Wheel event: delta_y={}", event.delta_y()).into());
            event.prevent_default();

            let delta_y = event.delta_y();
            let delta_zoom = if delta_y < 0.0 { 0.2 } else { -0.2 }; // constant step

            let (old_zoom, _) = chart_signal().with_untracked(|c| {
                let interval = current_interval().get_untracked();
                let candles = c.get_series(interval).unwrap().get_candles();
                viewport_zoom_pan(candles, &c.viewport)
            });
            let new_zoom = (old_zoom + delta_zoom).clamp(MIN_ZOOM_LEVEL, MAX_ZOOM_LEVEL);
            let applied_factor = (new_zoom / old_zoom) as f32;
            let center_x = event.offset_x() as f32 / 800.0;
            let pan_diff = center_x - 0.5;
            chart_signal().update(|ch| {
                ch.zoom(applied_factor, center_x);
                ch.pan(pan_diff, 0.0);
            });
            let symbol = current_symbol().get_untracked();
            chart_signal().with_untracked(|c| set_chart_in_ecs(&symbol, c.clone()));
            web_sys::console::log_1(
                &format!("🔍 Zoom: {:.2}x -> {:.2}x", old_zoom, new_zoom).into(),
            );

            // Apply zoom immediately without effects
            chart_signal().with_untracked(|ch| {
                if ch.get_candle_count() > 0
                    && with_global_renderer(|r| {
                        let interval = current_interval().get_untracked();
                        let series = ch.get_series(interval).unwrap();
                        let (_, pan) = viewport_zoom_pan(series.get_candles(), &ch.viewport);
                        r.set_zoom_params(new_zoom, pan);
                        let _ = r.render(ch);
                        get_logger().info(
                            LogComponent::Infrastructure("ZoomWheel"),
                            &format!("✅ Applied zoom {:.2}x to WebGPU renderer", new_zoom),
                        );
                    })
                    .is_none()
                {
                    // renderer not available
                }
            });
            get_logger().info(
                LogComponent::Presentation("ChartZoom"),
                &format!("🔍 Zoom level: {:.2}x", new_zoom),
            );
            let need_history = chart_signal().with_untracked(|c| {
                let interval = current_interval().get_untracked();
                let series = c.get_series(interval).unwrap();
                let (_, pan) = viewport_zoom_pan(series.get_candles(), &c.viewport);
                should_fetch_history(pan)
            });
            if need_history {
                fetch_more_history(status_clone);
            }
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
            if let Some(target) = event.target() {
                if let Ok(canvas) = target.dyn_into::<web_sys::HtmlCanvasElement>() {
                    let _ = canvas.focus();
                }
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
                let (old_zoom, _) = chart_signal().with_untracked(|c| {
                    let interval = current_interval().get_untracked();
                    let candles = c.get_series(interval).unwrap().get_candles();
                    viewport_zoom_pan(candles, &c.viewport)
                });
                let new_zoom = (old_zoom * factor).clamp(MIN_ZOOM_LEVEL, MAX_ZOOM_LEVEL);
                chart_signal().update(|ch| {
                    let apply = (new_zoom / old_zoom) as f32;
                    ch.zoom(apply, 0.5);
                });
                let symbol = current_symbol().get_untracked();
                chart_signal().with_untracked(|c| set_chart_in_ecs(&symbol, c.clone()));
                chart_signal().with_untracked(|c| {
                    if c.get_candle_count() > 0
                        && with_global_renderer(|r| {
                            let interval = current_interval().get_untracked();
                            let series = c.get_series(interval).unwrap();
                            let (_, pan) = viewport_zoom_pan(series.get_candles(), &c.viewport);
                            r.set_zoom_params(new_zoom, pan);
                            let _ = r.render(c);
                            get_logger().info(
                                LogComponent::Infrastructure("KeyboardZoom"),
                                &format!(
                                    "⌨️ Applied keyboard zoom {:.2}x to WebGPU renderer",
                                    new_zoom
                                ),
                            );
                        })
                        .is_none()
                    {
                        // renderer not available
                    }
                });
                get_logger().info(
                    LogComponent::Presentation("KeyboardZoom"),
                    &format!("⌨️ Zoom level: {:.2}x", new_zoom),
                );
                let need_history = chart_signal().with_untracked(|c| {
                    let interval = current_interval().get_untracked();
                    let series = c.get_series(interval).unwrap();
                    let (_, pan) = viewport_zoom_pan(series.get_candles(), &c.viewport);
                    should_fetch_history(pan)
                });
                if need_history {
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

    // Zoom effect removed - handled directly in the wheel handler

    view! {
        <div class="chart-container">
            <div style="display:flex;justify-content:space-between;margin-bottom:8px;width:800px;">
                <AssetSelector set_status=set_status />
                <div style="display:flex;gap:6px;">
                    <TimeframeSelector chart=chart() />
                </div>
            </div>

            <div style="display: flex; flex-direction: row; align-items: flex-start;">
                <PriceAxisLeft chart=chart() />
                <div style="position: relative;">
                    <canvas
                        id="chart-canvas"
                        node_ref=canvas_ref
                        use:wheel_event_options=&EventOptions { passive: false, capture: false, once: false }
                        width="800"
                        height="500"
                        tabindex="0"
                        style="border: 2px solid #4a5d73; border-radius: 10px; background: #253242; cursor: crosshair; outline: none;"
                        on:mousemove=handle_mouse_move
                        on:mouseleave=handle_mouse_leave
                        on:mousedown=handle_mouse_down
                        on:mouseup=handle_mouse_up
                        on:keydown=handle_keydown
                    />
                    <PriceScale chart=chart() />
                    <ChartTooltip />
                </div>
            </div>

            <Legend chart=chart() />

            // Time scale below the chart
            <div style="display: flex; justify-content: center; margin-top: 10px;">
                <TimeScale chart=chart() />
            </div>

            <div class="status">
                {move || status.get()}
            </div>

            // Control hints
            <div style="text-align: center; margin-top: 10px; font-size: 12px; color: #888;">
                "🔍 Zoom: Mouse wheel, +/- keys, PageUp/PageDown | 🖱️ Pan: Left click + drag | 🎯 Tooltip: Mouse hover"
            </div>
        </div>
    }
}

/// 💰 Price scale on the right side of the chart
#[component]
fn PriceScale(chart: RwSignal<Chart>) -> impl IntoView {
    let current_price = global_current_price();

    // Calculate price levels for display (same as in the grid)
    let price_levels = move || {
        let vp = chart.with(|c| c.viewport.clone());
        let levels = price_levels(&vp);
        let step = 100.0 / 8.0;
        levels
            .into_iter()
            .enumerate()
            .map(|(i, level_price)| (level_price, i as f64 * step))
            .collect::<Vec<_>>()
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
                        style=format!("position: absolute; top: {}%; right: 5px; transform: translateY(-50%); font-size: 11px; color: #888; background: rgba(0,0,0,0.7); padding: 2px 4px; border-radius: 2px;", position)
                    >
                        {format!("{:.2}", price)}
                    </div>
                }
            />

            // Display the current price (highlighted)
            <div class="current-price-label" style=format!("top: 50%")>
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
fn TimeframeSelector(chart: RwSignal<Chart>) -> impl IntoView {
    let options = vec![
        TimeInterval::TwoSeconds,
        TimeInterval::OneMinute,
        TimeInterval::FiveMinutes,
        TimeInterval::FifteenMinutes,
        TimeInterval::OneHour,
    ];

    view! {
        <div style="display:flex;gap:6px;margin-top:8px;">
            <For
                each=move || options.clone()
                key=|i| i.as_ref().to_string()
                children=move |interval| {
                    let label = interval.as_ref().to_string();
                    let chart_signal = chart;
                    view! {
                        <button
                            style="padding:4px 6px;border:none;border-radius:4px;background:#74c787;color:black;"
                            on:click=move |_| {
                                current_interval().set(interval);
                                chart_signal.update(|c| c.update_viewport_for_data());
                                chart_signal.with_untracked(|c| {
                                    if c.get_candle_count() > 0 && with_global_renderer(|r| {
                                            let interval = current_interval().get_untracked();
                                            let series = c.get_series(interval).unwrap();
                                            let (zoom, pan) = viewport_zoom_pan(series.get_candles(), &c.viewport);
                                            r.set_zoom_params(zoom, pan);
                                            let _ = r.render(c);
                                        }).is_none() {
                                        // renderer not available
                                    }
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
    view! {
        <label style="display:flex;align-items:center;gap:4px;">
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
            {label}
        </label>
    }
}

#[component]
fn Legend(chart: RwSignal<Chart>) -> impl IntoView {
    let names = vec!["sma20", "sma50", "sma200", "ema12", "ema26"];
    view! {
        <div style="display:flex;gap:6px;margin-top:8px;">
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
        <div style="display:flex;gap:6px;margin-top:8px;">
            <For
                each=move || options.clone()
                key=|s: &Symbol| s.value().to_string()
                children=move |sym: Symbol| {
                    let label = sym.value().to_string();
                    let status_cloned = set_status;
                    view! {
                        <button
                            style="padding:4px 6px;border:none;border-radius:4px;background:#2a5298;color:white;"
                            on:click=move |_| {
                                current_symbol().set(sym.clone());
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

    if let Some(_handle) = stream_abort_handles().with(|m| m.get(&symbol).cloned()) {
        // Already streaming for this symbol
        set_status.set("🔄 Using existing stream".to_string());
        return;
    }

    let interval = current_interval().get_untracked();

    let rest_client_arc =
        Arc::new(Mutex::new(BinanceWebSocketClient::new(symbol.clone(), interval)));

    // Set the streaming status
    global_is_streaming().set(false);

    // 📈 First load historical data
    set_status.set("📈 Loading historical data...".to_string());

    let hist_res = {
        let client = rest_client_arc.lock().await;
        client.fetch_historical_data(500).await
    };
    match hist_res {
        Ok(historical_candles) => {
            get_logger().info(
                LogComponent::Presentation("WebSocketStream"),
                &format!("✅ Loaded {} historical candles", historical_candles.len()),
            );

            chart.update(|ch| ch.set_historical_data(historical_candles.clone()));
            chart.with_untracked(|c| set_chart_in_ecs(&symbol, c.clone()));
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
                &format!("❌ Failed to load historical data: {}", e),
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
            let handler = move |candle: Candle| {
                if handler_handle.is_aborted() {
                    return;
                }
                global_current_price().set(candle.ohlcv.close.value());

                chart.update(|ch| {
                    ch.add_realtime_candle(candle.clone());
                    let interval = current_interval().get_untracked();
                    let series = ch.get_series(interval).unwrap();
                    let (zoom, pan) = viewport_zoom_pan(series.get_candles(), &ch.viewport);
                    let len = ch.get_candle_count();
                    if should_auto_scroll(len, zoom, pan) {
                        ch.update_viewport_for_data();
                    }
                });
                chart.with_untracked(|c| set_chart_in_ecs(&symbol, c.clone()));
                crate::global_state::push_realtime_candle(candle.clone());

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
                set_status.set(format!("❌ WebSocket error: {}", e));
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

    #[wasm_bindgen_test]
    fn timeframe_buttons_update_interval() {
        let container = setup_container();
        let chart = create_rw_signal(Chart::new("test".to_string(), ChartType::Candlestick, 100));
        leptos::mount_to(container.clone(), move || view! { <TimeframeSelector chart=chart /> });

        let two_sec = find_button(&container, "2s").expect("2s button not found");
        two_sec.click();
        assert_eq!(current_interval().get(), TimeInterval::TwoSeconds);

        let five = find_button(&container, "5m").expect("5m button not found");
        five.click();
        assert_eq!(current_interval().get(), TimeInterval::FiveMinutes);

        let fifteen = find_button(&container, "15m").expect("15m button not found");
        fifteen.click();
        assert_eq!(current_interval().get(), TimeInterval::FifteenMinutes);

        let one_hour = find_button(&container, "1h").expect("1h button not found");
        one_hour.click();
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
