//! Main Leptos application module.
//!
//! Handles canvas interactions, zoom/pan logic and connects to the
//! WebSocket stream providing market data.

use futures::lock::Mutex;
use js_sys;
use leptos::html::Canvas;
use leptos::spawn_local_with_current_owner;
use leptos::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wasm_bindgen::JsCast;

use crate::global_signals;
use crate::{
    domain::{
        chart::Chart,
        logging::{LogComponent, get_logger},
        market_data::{
            Candle, TimeInterval,
            value_objects::{Symbol, default_symbols},
        },
    },
    infrastructure::{
        rendering::{
            WebGpuRenderer,
            renderer::{set_global_renderer, with_global_renderer},
        },
        websocket::BinanceWebSocketClient,
    },
};

/// Maximum number of candles visible at 1x zoom
const MAX_VISIBLE_CANDLES: f64 = 32.0;
/// Minimum number of candles that must remain visible
const MIN_VISIBLE_CANDLES: f64 = 1.0;

/// Minimum allowed zoom level
const MIN_ZOOM_LEVEL: f64 = MAX_VISIBLE_CANDLES / 150.0;
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
    let min_start = (base_start - HISTORY_BUFFER_SIZE as isize).max(0);
    let max_start = len as isize - visible;
    let start = (base_start + offset).clamp(min_start, max_start);
    (start as usize, visible as usize)
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

// Helper aliases for global signals
global_signals! {
    pub global_current_price => current_price: f64,
    global_candle_count => candle_count: usize,
    global_is_streaming => is_streaming: bool,
    global_max_volume => max_volume: f64,
    loading_more => loading_more: bool,
    tooltip_data => tooltip_data: Option<TooltipData>,
    tooltip_visible => tooltip_visible: bool,
    zoom_level => zoom_level: f64,
    pan_offset => pan_offset: f64,
    is_dragging => is_dragging: bool,
    last_mouse_x => last_mouse_x: f64,
    pub current_interval => current_interval: TimeInterval,
    pub current_symbol => current_symbol: Symbol,
    pub stream_abort_handle => stream_abort_handle: Option<futures::future::AbortHandle>,
}

/// 📈 Fetch additional history and prepend it to the list
fn fetch_more_history(chart: RwSignal<Chart>, set_status: WriteSignal<String>) {
    if loading_more().get() {
        return;
    }

    let oldest_ts = chart.with(|c| {
        c.get_series(TimeInterval::OneMinute)
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
        let client_arc = Arc::new(Mutex::new(BinanceWebSocketClient::new(
            symbol.clone(),
            TimeInterval::OneMinute,
        )));
        let visible = chart.with(|c| {
            let len = c.get_candle_count();
            visible_range(len, zoom_level().get_untracked(), pan_offset().get_untracked()).1
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
                chart.with_untracked(|c| {
                    if c.get_candle_count() > 0 {
                        with_global_renderer(|r| {
                            r.set_zoom_params(
                                zoom_level().with_untracked(|z| *z),
                                pan_offset().with_untracked(|p| *p),
                            );
                            let _ = r.render(c);
                        });
                    }
                });

                let new_count = chart.with(|c| c.get_candle_count());
                let max_volume = chart.with(|c| {
                    c.get_series(TimeInterval::OneMinute)
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
    let zoom_level = zoom_level();

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
                        {move || format!("{:.1}x", zoom_level.get())}
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
        let zoom = zoom_level().get_untracked();
        let interval = current_interval().get_untracked();
        let candles = chart.with(|c| c.get_series(interval).unwrap().get_candles().clone());

        if candles.is_empty() {
            return vec![];
        }

        let (start_idx, visible) = visible_range(candles.len(), zoom, pan_offset().get_untracked());

        // Show 5 time labels
        let num_labels = 5;
        let mut labels = Vec::new();

        for i in 0..num_labels {
            let index = (i * visible) / (num_labels - 1);
            if let Some(candle) =
                candles.iter().skip(start_idx).nth(index.min(visible.saturating_sub(1)))
            {
                let timestamp = candle.timestamp.value();
                let date = js_sys::Date::new(&(timestamp as f64).into());
                let time_str = if zoom >= 2.0 {
                    format!("{:02}:{:02}", date.get_hours(), date.get_minutes())
                } else if zoom >= 1.0 {
                    format!("{:02}.{:02}", date.get_date(), date.get_month() + 1)
                } else {
                    format!("{:02}.{}", date.get_month() + 1, date.get_full_year())
                };
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
    // Signals for the chart
    let chart = create_rw_signal(Chart::new(
        "leptos-chart".to_string(),
        crate::domain::chart::ChartType::Candlestick,
        1000,
    ));
    let (renderer, set_renderer) = create_signal::<Option<Rc<RefCell<WebGpuRenderer>>>>(None);
    let (status, set_status) = create_signal("Initializing...".to_string());

    // Reference to the canvas element
    let canvas_ref = create_node_ref::<Canvas>();
    let (initialized, set_initialized) = create_signal(false);

    // Initialize WebGPU only once when the canvas is mounted
    canvas_ref.on_load(move |_| {
        if initialized.get() {
            return;
        }
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

                match WebGpuRenderer::new("chart-canvas", 800, 500).await {
                    Ok(webgpu_renderer) => {
                        get_logger().info(
                            LogComponent::Infrastructure("WebGPU"),
                            "✅ WebGPU renderer created successfully",
                        );

                        let renderer_rc = Rc::new(RefCell::new(webgpu_renderer));
                        set_renderer.set(Some(renderer_rc.clone()));
                        set_global_renderer(renderer_rc.clone());
                        let _ = renderer_rc.borrow().log_gpu_memory_usage();
                        set_status.set("✅ WebGPU renderer ready".to_string());

                        // Start WebSocket after the renderer is initialized
                        get_logger().info(
                            LogComponent::Infrastructure("WebSocket"),
                            "🌐 Starting WebSocket stream...",
                        );
                        start_websocket_stream(chart, set_status).await;
                    }
                    Err(e) => {
                        get_logger().error(
                            LogComponent::Infrastructure("WebGPU"),
                            &format!("❌ WebGPU initialization failed: {:?}", e),
                        );
                        set_status.set(format!("❌ WebGPU failed: {:?}\n💡 Try Chrome Canary with --enable-unsafe-webgpu flag", e));

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

                        chart.update(|ch| ch.set_historical_data(test_candles));
                        set_status
                            .set("🎯 Demo mode: Using test data (WebSocket disabled)".to_string());
                    }
                }
            });
    });

    // 🎯 Mouse events for the tooltip
    let handle_mouse_move = {
        let chart_signal = chart;
        let renderer_clone = renderer;
        let status_clone = set_status;
        move |event: web_sys::MouseEvent| {
            let mouse_x = event.offset_x() as f64;
            let mouse_y = event.offset_y() as f64;

            // 🔍 Handle panning
            let dragging = is_dragging().get_untracked();
            if dragging {
                let last_x = last_mouse_x().get_untracked();
                let delta_x = mouse_x - last_x;
                pan_offset().update(|o| {
                    let pan_sensitivity = zoom_level().with_untracked(|val| *val) * 0.001;
                    *o -= delta_x * pan_sensitivity;
                });
                chart_signal.update(|ch| {
                    let factor_x = -(delta_x as f32) / ch.viewport.width as f32;
                    ch.pan(factor_x, 0.0);
                });
                last_mouse_x().set(mouse_x);

                let need_history = pan_offset().with_untracked(|val| should_fetch_history(*val));
                if need_history {
                    fetch_more_history(chart_signal, status_clone);
                }

                chart_signal.with_untracked(|ch| {
                    if ch.get_candle_count() > 0 {
                        renderer_clone.with_untracked(|renderer_opt| {
                            if let Some(renderer_rc) = renderer_opt {
                                if let Ok(mut webgpu_renderer) = renderer_rc.try_borrow_mut() {
                                    webgpu_renderer.set_zoom_params(
                                        zoom_level().with_untracked(|val| *val),
                                        pan_offset().with_untracked(|val| *val),
                                    );
                                    let _ = webgpu_renderer.render(ch);
                                }
                            }
                        });
                    }
                });
            } else {
                // Convert to NDC coordinates (assuming an 800x500 canvas)
                let canvas_width = 800.0;
                let canvas_height = 500.0;
                let ndc_x = (mouse_x / canvas_width) * 2.0 - 1.0;
                let _ndc_y = 1.0 - (mouse_y / canvas_height) * 2.0;

                chart_signal.with_untracked(|ch| {
                    let interval = current_interval().get_untracked();
                    let candles = ch.get_series(interval).unwrap().get_candles();
                    if !candles.is_empty() {
                        let (start_idx, visible_count) = visible_range(
                            candles.len(),
                            zoom_level().get_untracked(),
                            pan_offset().get_untracked(),
                        );
                        let visible: Vec<_> =
                            candles.iter().skip(start_idx).take(visible_count).collect();

                        // Use the same logic as in candle_x_position
                        let step_size = 2.0 / visible.len() as f64;
                        // Inverse formula: if x = 1.0 - (visible_len - index - 1) * step_size
                        // then index = visible_len - (1.0 - x) / step_size - 1
                        let index_float = visible.len() as f64 - (1.0 - ndc_x) / step_size - 1.0;
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
            web_sys::console::log_1(&format!("🖱️ Wheel event: delta_y={}", event.delta_y()).into());
            event.prevent_default();

            let delta_y = event.delta_y();
            let zoom_factor = if delta_y < 0.0 { 1.1 } else { 0.9 }; // Zoom in/out

            let old_zoom = zoom_level().with_untracked(|z| *z);
            let new_zoom = (old_zoom * zoom_factor).clamp(MIN_ZOOM_LEVEL, MAX_ZOOM_LEVEL);
            zoom_level().set(new_zoom);
            let applied_factor = (new_zoom / old_zoom) as f32;
            let center_x = event.offset_x() as f32 / 800.0;
            chart_signal.update(|ch| ch.zoom(applied_factor, center_x));
            web_sys::console::log_1(
                &format!("🔍 Zoom: {:.2}x -> {:.2}x", old_zoom, new_zoom).into(),
            );

            // Apply zoom immediately without effects
            chart_signal.with_untracked(|ch| {
                if ch.get_candle_count() > 0 {
                    with_global_renderer(|r| {
                        r.set_zoom_params(new_zoom, pan_offset().with_untracked(|val| *val));
                        let _ = r.render(ch);
                        get_logger().info(
                            LogComponent::Infrastructure("ZoomWheel"),
                            &format!("✅ Applied zoom {:.2}x to WebGPU renderer", new_zoom),
                        );
                    });
                }
            });
            get_logger().info(
                LogComponent::Presentation("ChartZoom"),
                &format!("🔍 Zoom level: {:.2}x", zoom_level().with_untracked(|z_val| *z_val)),
            );
            let need_history = pan_offset().with_untracked(|val| should_fetch_history(*val));
            if need_history {
                fetch_more_history(chart_signal, status_clone);
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
            let mut zoom_changed = false;

            match key.as_str() {
                "+" | "=" => {
                    event.prevent_default();
                    zoom_level().update(|z| {
                        *z *= 1.2;
                        *z = z.min(MAX_ZOOM_LEVEL);
                    });
                    zoom_changed = true;
                }
                "-" | "_" => {
                    event.prevent_default();
                    zoom_level().update(|z| {
                        *z *= 0.8;
                        *z = z.max(MIN_ZOOM_LEVEL);
                    });
                    zoom_changed = true;
                }
                "PageUp" => {
                    event.prevent_default();
                    zoom_level().update(|z| {
                        *z *= 1.5;
                        *z = z.min(MAX_ZOOM_LEVEL);
                    });
                    zoom_changed = true;
                }
                "PageDown" => {
                    event.prevent_default();
                    zoom_level().update(|z| {
                        *z *= 0.67;
                        *z = z.max(MIN_ZOOM_LEVEL);
                    });
                    zoom_changed = true;
                }
                _ => {}
            }

            if zoom_changed {
                let new_zoom = zoom_level().with_untracked(|z_val| *z_val);
                web_sys::console::log_1(&format!("⌨️ Keyboard zoom: {:.2}x", new_zoom).into());

                // Apply zoom to the renderer for keyboard commands
                chart_signal.with_untracked(|ch| {
                    if ch.get_candle_count() > 0 {
                        with_global_renderer(|r| {
                            r.set_zoom_params(new_zoom, pan_offset().with_untracked(|val| *val));
                            let _ = r.render(ch);
                            get_logger().info(
                                LogComponent::Infrastructure("KeyboardZoom"),
                                &format!(
                                    "⌨️ Applied keyboard zoom {:.2}x to WebGPU renderer",
                                    new_zoom
                                ),
                            );
                        });
                    }
                });

                get_logger().info(
                    LogComponent::Presentation("KeyboardZoom"),
                    &format!("⌨️ Zoom level: {:.2}x", new_zoom),
                );
                let need_history = pan_offset().with_untracked(|val| should_fetch_history(*val));
                if need_history {
                    fetch_more_history(chart_signal, status_clone);
                }
            }
        }
    };

    // Zoom effect removed - handled directly in the wheel handler

    view! {
        <div class="chart-container">
            <div style="display:flex;justify-content:space-between;margin-bottom:8px;">
                <AssetSelector chart=chart set_status=set_status />
                <div style="display:flex;gap:6px;">
                    <TimeframeSelector chart=chart />
                    <CurrentTimeButton chart=chart />
                    <IndicatorToggles chart=chart />
                </div>
            </div>

            <div style="display: flex; flex-direction: row; align-items: flex-start;">
                <PriceAxisLeft chart=chart />
                <div style="position: relative;">
                    <canvas
                        id="chart-canvas"
                        node_ref=canvas_ref
                        width="800"
                        height="500"
                        tabindex="0"
                        style="border: 2px solid #4a5d73; border-radius: 10px; background: #253242; cursor: crosshair; outline: none;"
                        on:mousemove=handle_mouse_move
                        on:mouseleave=handle_mouse_leave
                        on:wheel=handle_wheel
                        on:mousedown=handle_mouse_down
                        on:mouseup=handle_mouse_up
                        on:keydown=handle_keydown
                    />
                    <PriceScale chart=chart />
                    <ChartTooltip />
                </div>
            </div>

            <Legend chart=chart />

            // Time scale below the chart
            <div style="display: flex; justify-content: center; margin-top: 10px;">
                <TimeScale chart=chart />
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
        TimeInterval::OneMinute,
        TimeInterval::FiveMinutes,
        TimeInterval::FifteenMinutes,
        TimeInterval::ThirtyMinutes,
        TimeInterval::OneHour,
        TimeInterval::OneDay,
        TimeInterval::OneWeek,
        TimeInterval::OneMonth,
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
                            style="padding:4px 6px;border:none;border-radius:4px;background:#2a5298;color:white;"
                            on:click=move |_| {
                                current_interval().set(interval);
                                chart_signal.update(|c| c.update_viewport_for_data());
                                chart_signal.with_untracked(|c| {
                                    if c.get_candle_count() > 0 {
                                        with_global_renderer(|r| {
                                            r.set_zoom_params(
                                                zoom_level().with_untracked(|z| *z),
                                                pan_offset().with_untracked(|p| *p),
                                            );
                                            let _ = r.render(c);
                                        });
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
fn CurrentTimeButton(chart: RwSignal<Chart>) -> impl IntoView {
    view! {
        <button
            style="padding:4px 6px;border:none;border-radius:4px;background:#2a5298;color:white;"
            on:click=move |_| {
                pan_offset().set(0.0);
                chart.update(|c| c.update_viewport_for_data());
                chart.with_untracked(|c| {
                    if c.get_candle_count() > 0 {
                        with_global_renderer(|r| {
                            r.set_zoom_params(zoom_level().with_untracked(|z| *z), 0.0);
                            let _ = r.render(c);
                        });
                    }
                });
            }
        >
            "now"
        </button>
    }
}

#[component]
fn IndicatorToggles(chart: RwSignal<Chart>) -> impl IntoView {
    let options = vec!["sma20", "sma50", "sma200", "ema12", "ema26"];

    view! {
        <div style="display:flex;gap:6px;margin-top:8px;">
            <For
                each=move || options.clone()
                key=|name| name.to_string()
                children=move |name| {
                    let label = name.to_uppercase();
                    let chart_signal = chart;
                    view! {
                        <button
                            style="padding:4px 6px;border:none;border-radius:4px;background:#2a5298;color:white;"
                            on:click=move |_| {
                                chart_signal.with_untracked(|c| {
                                    with_global_renderer(|r| {
                                        r.toggle_line_visibility(name);
                                        let _ = r.render(c);
                                    });
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
    view! {
        <label style="display:flex;align-items:center;gap:4px;">
            <input
                type="checkbox"
                id=id
                checked=true
                on:change=move |_| {
                    chart.with_untracked(|c| {
                        with_global_renderer(|r| {
                            r.toggle_line_visibility(name);
                            let _ = r.render(c);
                        });
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
fn AssetSelector(chart: RwSignal<Chart>, set_status: WriteSignal<String>) -> impl IntoView {
    let options = default_symbols();

    view! {
        <div style="display:flex;gap:6px;margin-top:8px;">
            <For
                each=move || options.clone()
                key=|s: &Symbol| s.value().to_string()
                children=move |sym: Symbol| {
                    let label = sym.value().to_string();
                    let chart_cloned = chart;
                    let status_cloned = set_status;
                    view! {
                        <button
                            style="padding:4px 6px;border:none;border-radius:4px;background:#2a5298;color:white;"
                            on:click=move |_| {
                                current_symbol().set(sym.clone());
                                let _ = spawn_local_with_current_owner(async move {
                                    start_websocket_stream(chart_cloned, status_cloned).await;
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

/// 🌐 Start WebSocket stream in Leptos and update global signals
async fn start_websocket_stream(chart: RwSignal<Chart>, set_status: WriteSignal<String>) {
    if let Some(handle) = stream_abort_handle().get_untracked() {
        handle.abort();
        stream_abort_handle().set(None);
    }

    let symbol = current_symbol().get_untracked();
    let interval = TimeInterval::OneMinute;

    let rest_client_arc =
        Arc::new(Mutex::new(BinanceWebSocketClient::new(symbol.clone(), interval)));

    // Set the streaming status
    global_is_streaming().set(false);

    // 📈 First load historical data
    set_status.set("📈 Loading historical data...".to_string());

    let hist_res = {
        let client = rest_client_arc.lock().await;
        client.fetch_historical_data(300).await
    };
    match hist_res {
        Ok(historical_candles) => {
            get_logger().info(
                LogComponent::Presentation("WebSocketStream"),
                &format!("✅ Loaded {} historical candles", historical_candles.len()),
            );

            chart.update(|ch| ch.set_historical_data(historical_candles.clone()));
            chart.with_untracked(|c| {
                if c.get_candle_count() > 0 {
                    with_global_renderer(|r| {
                        r.set_zoom_params(
                            zoom_level().with_untracked(|z| *z),
                            pan_offset().with_untracked(|p| *p),
                        );
                        let _ = r.render(c);
                    });
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

    let stream_client_arc = Arc::new(Mutex::new(BinanceWebSocketClient::new(symbol, interval)));
    let (abort_handle, abort_reg) = futures::future::AbortHandle::new_pair();
    stream_abort_handle().set(Some(abort_handle.clone()));
    let fut = futures::future::Abortable::new(
        async move {
            let handler = move |candle: Candle| {
                // Update the price in the global signal
                global_current_price().set(candle.ohlcv.close.value());

                chart.update(|ch| {
                    ch.add_realtime_candle(candle.clone());
                    if (zoom_level().get_untracked() - 1.0).abs() < f64::EPSILON
                        && pan_offset().get_untracked().abs() < f64::EPSILON
                    {
                        ch.update_viewport_for_data();
                    }
                });

                let count = chart.with(|c| c.get_candle_count());
                global_candle_count().set(count);

                let max_vol = chart.with(|c| {
                    c.get_series(TimeInterval::OneMinute)
                        .unwrap()
                        .get_candles()
                        .iter()
                        .map(|c| c.ohlcv.volume.value())
                        .fold(0.0f64, |a, b| a.max(b))
                });
                global_max_volume().set(max_vol);

                chart.with_untracked(|ch| {
                    if ch.get_candle_count() > 0 {
                        with_global_renderer(|r| {
                            r.set_zoom_params(
                                zoom_level().with_untracked(|z| *z),
                                pan_offset().with_untracked(|p| *p),
                            );
                            let _ = r.render(ch);
                        });
                    }
                });

                // Update the status
                set_status.set("🌐 WebSocket LIVE • Real-time updates".to_string());
            };

            let result = {
                let mut client = stream_client_arc.lock().await;
                client.start_stream(handler).await
            };
            if let Err(e) = result {
                set_status.set(format!("❌ WebSocket error: {}", e));
                global_is_streaming().set(false);
            }
        },
        abort_reg,
    );

    let _ = spawn_local_with_current_owner(async move {
        let _ = fut.await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chart::value_objects::ChartType;
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

    fn find_button(container: &web_sys::HtmlElement, label: &str) -> web_sys::HtmlElement {
        let buttons = container.get_elements_by_tag_name("button");
        for i in 0..buttons.length() {
            let btn = buttons.item(i).unwrap().dyn_into::<web_sys::HtmlElement>().unwrap();
            if btn.text_content().unwrap_or_default() == label {
                return btn;
            }
        }
        panic!("button with label {label} not found", label = label);
    }

    fn find_checkbox(container: &web_sys::HtmlElement, id: &str) -> web_sys::HtmlInputElement {
        container
            .query_selector(&format!("#{}", id))
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap()
    }

    #[wasm_bindgen_test]
    fn timeframe_buttons_update_interval() {
        let container = setup_container();
        let chart = create_rw_signal(Chart::new("test".to_string(), ChartType::Candlestick, 100));
        leptos::mount_to(container.clone(), move || view! { <TimeframeSelector chart=chart /> });

        let five = find_button(&container, "5m");
        five.click();
        assert_eq!(current_interval().get(), TimeInterval::FiveMinutes);

        let fifteen = find_button(&container, "15m");
        fifteen.click();
        assert_eq!(current_interval().get(), TimeInterval::FifteenMinutes);

        let thirty = find_button(&container, "30m");
        thirty.click();
        assert_eq!(current_interval().get(), TimeInterval::ThirtyMinutes);
    }

    #[wasm_bindgen_test]
    fn indicator_toggle_changes_visibility() {
        use crate::infrastructure::rendering::renderer::{dummy_renderer, set_global_renderer};
        use std::cell::RefCell;
        use std::rc::Rc;

        let container = setup_container();
        let chart = create_rw_signal(Chart::new("test".to_string(), ChartType::Candlestick, 10));
        let renderer = Rc::new(RefCell::new(dummy_renderer()));
        chart.with_untracked(|c| renderer.borrow_mut().cache_geometry_for_test(c));
        let initial = renderer.borrow().cached_hash_for_test();

        set_global_renderer(renderer.clone());
        leptos::mount_to(container.clone(), move || view! { <IndicatorToggles chart=chart /> });

        let btn = find_button(&container, "SMA20");
        btn.click();

        let changed = renderer.borrow().cached_hash_for_test();
        assert_ne!(changed, initial);
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

        let cb = find_checkbox(&container, "sma20");
        cb.click();

        assert!(!renderer.borrow().line_visibility().sma_20);
    }
    #[wasm_bindgen_test]
    fn now_button_resets_pan() {
        let container = setup_container();
        let chart = create_rw_signal(Chart::new("test".to_string(), ChartType::Candlestick, 100));
        leptos::mount_to(container.clone(), move || view! { <CurrentTimeButton chart=chart /> });

        pan_offset().set(5.0);

        let now = find_button(&container, "Now");
        now.click();

        assert_eq!(pan_offset().get(), 0.0);
    }

    #[test]
    fn zoom_limits_respected_by_visible_range() {
        let (_, visible_min_zoom) = visible_range(1000, MIN_ZOOM_LEVEL, 0.0);
        assert!(visible_min_zoom <= 150);

        let (_, visible_max_zoom) = visible_range(1000, MAX_ZOOM_LEVEL, 0.0);
        assert!(visible_max_zoom as f64 >= MIN_VISIBLE_CANDLES);
    }
}
