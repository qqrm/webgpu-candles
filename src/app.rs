// src/app.rs

use js_sys;
use leptos::html::Canvas;
use leptos::spawn_local_with_current_owner;
use leptos::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;

use crate::{
    domain::{
        chart::Chart,
        logging::{LogComponent, get_logger},
        market_data::{Candle, TimeInterval, value_objects::Symbol},
    },
    global_state::globals,
    infrastructure::{
        rendering::{
            WebGpuRenderer,
            renderer::{set_global_renderer, with_global_renderer},
        },
        websocket::BinanceWebSocketClient,
    },
};

/// Maximum number of candles visible at 1x zoom
const MAX_VISIBLE_CANDLES: f64 = 300.0;

/// Calculate visible range based on zoom level and pan offset
pub fn visible_range(len: usize, zoom: f64, pan: f64) -> (usize, usize) {
    let visible = ((MAX_VISIBLE_CANDLES / zoom).max(10.0).min(len as f64)) as isize;
    let base_start = len as isize - visible;
    let offset = pan.round() as isize;
    let max_start = len as isize - visible;
    let start = (base_start + offset).clamp(0, max_start);
    (start as usize, visible as usize)
}

// Helper aliases for global signals
fn global_current_price() -> RwSignal<f64> {
    globals().current_price
}
fn global_candle_count() -> RwSignal<usize> {
    globals().candle_count
}
fn global_is_streaming() -> RwSignal<bool> {
    globals().is_streaming
}
fn global_max_volume() -> RwSignal<f64> {
    globals().max_volume
}
fn loading_more() -> RwSignal<bool> {
    globals().loading_more
}
fn tooltip_data() -> RwSignal<Option<TooltipData>> {
    globals().tooltip_data
}
fn tooltip_visible() -> RwSignal<bool> {
    globals().tooltip_visible
}
fn zoom_level() -> RwSignal<f64> {
    globals().zoom_level
}
fn pan_offset() -> RwSignal<f64> {
    globals().pan_offset
}
fn is_dragging() -> RwSignal<bool> {
    globals().is_dragging
}
fn last_mouse_x() -> RwSignal<f64> {
    globals().last_mouse_x
}
fn last_mouse_y() -> RwSignal<f64> {
    globals().last_mouse_y
}
pub fn current_interval() -> RwSignal<TimeInterval> {
    globals().current_interval
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

    let _ = spawn_local_with_current_owner(async move {
        let client = BinanceWebSocketClient::new(Symbol::from("BTCUSDT"), TimeInterval::OneMinute);
        match client.fetch_historical_data_before(end_time, 300).await {
            Ok(mut new_candles) => {
                new_candles.sort_by(|a, b| a.timestamp.value().cmp(&b.timestamp.value()));
                chart.update(|ch| {
                    for candle in new_candles.iter() {
                        ch.add_candle(candle.clone());
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

        let formatted_text = format!(
            "{} BTC/USDT\n📈 Open:   ${:.2}\n📊 High:   ${:.2}\n📉 Low:    ${:.2}\n💰 Close:  ${:.2}\n📈 Change: ${:.2} ({:.2}%)\n📊 Volume: {:.4}\n{}",
            trend,
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

/// 🦀 Main Bitcoin Chart component built with Leptos
#[component]
pub fn app() -> impl IntoView {
    // 🚀 Initialize the global logger on application start
    use crate::domain::logging::get_logger;

    // Extra console.log for diagnostics
    web_sys::console::log_1(&"🚀 Starting Bitcoin Chart App".into());

    get_logger().info(LogComponent::Presentation("App"), "🚀 Starting Bitcoin Chart App");

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
            <h1>"🌐 Bitcoin WebSocket Chart"</h1>
            <p>"BTC/USDT • Real-time Leptos + WebGPU"</p>

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
        let interval = current_interval().get_untracked();
        let candles = chart.with(|c| c.get_series(interval).unwrap().get_candles().clone());
        if candles.is_empty() {
            return vec![];
        }

        let (start_idx, visible) = visible_range(
            candles.len(),
            zoom_level().get_untracked(),
            pan_offset().get_untracked(),
        );
        let (min, max) = candles
            .iter()
            .skip(start_idx)
            .take(visible)
            .fold((f64::MAX, f64::MIN), |(min, max), c| {
                (min.min(c.ohlcv.low.value()), max.max(c.ohlcv.high.value()))
            });
        let step = (max - min) / 8.0;
        (0..=8).rev().map(|i| min + i as f64 * step).collect::<Vec<_>>()
    };

    let handle_wheel = {
        let chart_signal = chart;
        move |e: web_sys::WheelEvent| {
            e.prevent_default();
            let factor = if e.delta_y() < 0.0 { 1.1 } else { 0.9 };
            let center = e.offset_y() as f32 / 500.0;
            chart_signal.update(|c| c.zoom_price(factor as f32, center));

            chart_signal.with_untracked(|ch| {
                if ch.get_candle_count() > 0 {
                    with_global_renderer(|r| {
                        r.set_zoom_params(
                            zoom_level().with_untracked(|val| *val),
                            pan_offset().with_untracked(|val| *val),
                        );
                        let _ = r.render(ch);
                    });
                }
            });
        }
    };

    view! {
        <div style="width: 60px; height: 500px; background: #222; display: flex; flex-direction: column; justify-content: space-between; align-items: flex-end; margin-right: 8px;" on:wheel=handle_wheel>
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

    let handle_wheel = {
        let chart_signal = chart;
        move |event: web_sys::WheelEvent| {
            event.prevent_default();
            let delta = event.delta_y();
            let zoom_factor = if delta < 0.0 { 1.1 } else { 0.9 };

            let old_zoom = zoom_level().with_untracked(|z| *z);
            zoom_level().update(|z| {
                *z *= zoom_factor;
                *z = z.clamp(0.1, 10.0);
            });
            let new_zoom = zoom_level().with_untracked(|z| *z);
            web_sys::console::log_1(
                &format!("🔍 Zoom: {:.2}x -> {:.2}x", old_zoom, new_zoom).into(),
            );

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
        }
    };

    view! {
        <div style="width: 800px; height: 30px; background: #222; display: flex; align-items: center; justify-content: space-between; padding: 0 10px; margin-top: 5px; border-radius: 5px;" on:wheel=handle_wheel>
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

    // Effect to initialize WebGPU after mounting
    create_effect(move |_| {
        if canvas_ref.get().is_some() {
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
        }
    });

    // Effect to render when data changes
    create_effect(move |_| {
        renderer.with(|renderer_opt| {
            if let Some(renderer_rc) = renderer_opt {
                chart.with(|ch| {
                    if ch.get_candle_count() > 0 {
                        if let Ok(mut webgpu_renderer) = renderer_rc.try_borrow_mut() {
                            if let Err(e) = webgpu_renderer.render(ch) {
                                set_status.set(format!("❌ Render error: {:?}", e));
                            } else {
                                set_status
                                    .set(format!("✅ Rendered {} candles", ch.get_candle_count()));
                            }
                        }
                    }
                });
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
            is_dragging().with(|dragging| {
                if *dragging {
                    last_mouse_x().with(|last_x| {
                        let delta_x = mouse_x - *last_x;
                        pan_offset().update(|o| {
                            let pan_sensitivity = zoom_level().with_untracked(|val| *val) * 0.001;
                            *o += delta_x * pan_sensitivity;
                        });
                        chart_signal.update(|ch| {
                            let factor_x = -(delta_x as f32) / ch.viewport.width as f32;
                            ch.pan(factor_x, 0.0);
                        });
                        last_mouse_x().set(mouse_x);
                    });

                    last_mouse_y().with(|last_y| {
                        let delta_y = mouse_y - *last_y;
                        chart_signal.update(|ch| {
                            let factor = delta_y as f32 / ch.viewport.height as f32;
                            ch.pan(0.0, factor);
                        });
                        last_mouse_y().set(mouse_y);
                    });

                    let need_history = pan_offset().with_untracked(|val| *val <= -950.0);
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

                    chart_signal.with(|ch| {
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
                            let index_float =
                                visible.len() as f64 - (1.0 - ndc_x) / step_size - 1.0;
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
            });
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
            zoom_level().update(|z| {
                *z *= zoom_factor;
                *z = z.clamp(0.1, 10.0); // Clamp zoom from 0.1x to 10x
            });
            let center_x = event.offset_x() as f32 / 800.0;
            chart_signal.update(|ch| ch.zoom(zoom_factor as f32, center_x));
            let new_zoom = zoom_level().with_untracked(|z| *z);
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
            let need_history = pan_offset().with_untracked(|val| *val <= -950.0);
            if need_history {
                fetch_more_history(chart_signal, status_clone);
            }
        }
    };

    // 🖱️ Start panning
    let handle_mouse_down = move |event: web_sys::MouseEvent| {
        if event.button() == 0 {
            // Left mouse button
            is_dragging().set(true);
            last_mouse_x().set(event.offset_x() as f64);
            last_mouse_y().set(event.offset_y() as f64);

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
                        *z = z.min(10.0);
                    });
                    zoom_changed = true;
                }
                "-" | "_" => {
                    event.prevent_default();
                    zoom_level().update(|z| {
                        *z *= 0.8;
                        *z = z.max(0.1);
                    });
                    zoom_changed = true;
                }
                "PageUp" => {
                    event.prevent_default();
                    zoom_level().update(|z| {
                        *z *= 1.5;
                        *z = z.min(10.0);
                    });
                    zoom_changed = true;
                }
                "PageDown" => {
                    event.prevent_default();
                    zoom_level().update(|z| {
                        *z *= 0.67;
                        *z = z.max(0.1);
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
                let need_history = pan_offset().with_untracked(|val| *val <= -950.0);
                if need_history {
                    fetch_more_history(chart_signal, status_clone);
                }
            }
        }
    };

    // Zoom effect removed - handled directly in the wheel handler

    view! {
        <div class="chart-container">
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
                    <PriceScale />
                    <ChartTooltip />
                </div>
            </div>

            <TimeframeSelector />

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
fn PriceScale() -> impl IntoView {
    let current_price = global_current_price();

    // Calculate price levels for display (same as in the grid)
    let price_levels = move || {
        let price = current_price.get();
        if price <= 0.0 {
            return vec![];
        }

        // Approximate price range (±3% of the current price)
        let min_price = price * 0.97;
        let max_price = price * 1.03;
        let price_range = max_price - min_price;

        // 8 price levels (as in the grid)
        let num_levels = 8;
        let mut levels = Vec::new();

        for i in 0..=num_levels {
            let level_price = min_price + (price_range * i as f64 / num_levels as f64);
            let position_percent = (i as f64 / num_levels as f64) * 100.0;
            levels.push((level_price, position_percent));
        }

        levels.reverse(); // Top to bottom
        levels
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
fn TimeframeSelector() -> impl IntoView {
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
                children=|interval| {
                    let label = interval.as_ref().to_string();
                    view! {
                        <button
                            style="padding:4px 6px;border:none;border-radius:4px;background:#2a5298;color:white;"
                            on:click=move |_| {
                                current_interval().set(interval);
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
    let symbol = Symbol::from("BTCUSDT");
    let interval = TimeInterval::OneMinute;

    // Create a client for data loading
    let ws_client = BinanceWebSocketClient::new(symbol, interval);

    // Set the streaming status
    global_is_streaming().set(false);

    // 📈 First load historical data
    set_status.set("📈 Loading historical data...".to_string());

    match ws_client.fetch_historical_data(300).await {
        Ok(historical_candles) => {
            get_logger().info(
                LogComponent::Presentation("WebSocketStream"),
                &format!("✅ Loaded {} historical candles", historical_candles.len()),
            );

            chart.update(|ch| ch.set_historical_data(historical_candles.clone()));

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

    let mut ws_client =
        BinanceWebSocketClient::new(Symbol::from("BTCUSDT"), TimeInterval::OneMinute);

    let _ = spawn_local_with_current_owner(async move {
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

            // Update the status
            set_status.set("🌐 WebSocket LIVE • Real-time updates".to_string());
        };

        if let Err(e) = ws_client.start_stream(handler).await {
            set_status.set(format!("❌ WebSocket error: {}", e));
            global_is_streaming().set(false);
        }
    });
}
