// src/app.rs

use js_sys;
use leptos::html::Canvas;
use leptos::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::{
    domain::{
        chart::Chart,
        logging::{LogComponent, LogLevel, get_logger},
        market_data::{Candle, TimeInterval, value_objects::Symbol},
    },
    infrastructure::{
        rendering::{WebGpuRenderer, renderer::set_global_renderer},
        websocket::BinanceWebSocketClient,
    },
};

// 🔗 Глобальные сигналы для логов (bridge к domain::logging)
thread_local! {
    static GLOBAL_LOGS: RwSignal<Vec<String>> = create_rw_signal(Vec::new());
    static IS_LOG_PAUSED: RwSignal<bool> = create_rw_signal(false);

    // 🌐 Глобальные сигналы для real-time данных
    static GLOBAL_CURRENT_PRICE: RwSignal<f64> = create_rw_signal(0.0);
    static GLOBAL_CANDLE_COUNT: RwSignal<usize> = create_rw_signal(0);
    static GLOBAL_IS_STREAMING: RwSignal<bool> = create_rw_signal(false);
    static GLOBAL_MAX_VOLUME: RwSignal<f64> = create_rw_signal(0.0);
    static LOADING_MORE: RwSignal<bool> = create_rw_signal(false);

    // 🎯 Tooltip данные
    static TOOLTIP_DATA: RwSignal<Option<TooltipData>> = create_rw_signal(None);
    static TOOLTIP_VISIBLE: RwSignal<bool> = create_rw_signal(false);

    // 🔍 Зум и панорамирование
    static ZOOM_LEVEL: RwSignal<f64> = create_rw_signal(1.0);
    static PAN_OFFSET: RwSignal<f64> = create_rw_signal(0.0);
    static IS_DRAGGING: RwSignal<bool> = create_rw_signal(false);
    static LAST_MOUSE_X: RwSignal<f64> = create_rw_signal(0.0);
    static LAST_MOUSE_Y: RwSignal<f64> = create_rw_signal(0.0);
}

/// 📈 Запрашивает дополнительную историю и добавляет в начало списка
fn fetch_more_history(chart: RwSignal<Chart>, set_status: WriteSignal<String>) {
    if LOADING_MORE.with(|l| l.get()) {
        return;
    }

    let oldest_ts = chart.with(|c| c.data.get_candles().front().map(|c| c.timestamp.value()));
    let end_time = match oldest_ts {
        Some(ts) if ts > 0 => ts - 1,
        _ => return,
    };

    LOADING_MORE.with(|l| l.set(true));

    spawn_local(async move {
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
                    c.data
                        .get_candles()
                        .iter()
                        .map(|c| c.ohlcv.volume.value())
                        .fold(0.0f64, |a, b| a.max(b))
                });
                GLOBAL_CANDLE_COUNT.with(|c| c.set(new_count));
                GLOBAL_MAX_VOLUME.with(|v| v.set(max_volume));

                set_status.set(format!("📈 Loaded {} older candles", new_candles.len()));
            }
            Err(e) => set_status.set(format!("❌ Failed to load more data: {}", e)),
        }

        LOADING_MORE.with(|l| l.set(false));
    });
}

/// 🎯 Данные для tooltip
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

        // Форматируем время из timestamp
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

        Self {
            candle,
            x,
            y,
            formatted_text,
        }
    }
}

/// 🌉 Bridge logger для подключения domain::logging к Leptos сигналам
pub struct LeptosLogger;

/// ⏰ Web time provider для domain::logging
pub struct WebTimeProvider;

/// Минимальный уровень логирования для LeptosLogger
const MIN_LOG_LEVEL: LogLevel = LogLevel::Warn;

impl crate::domain::logging::TimeProvider for WebTimeProvider {
    fn current_timestamp(&self) -> u64 {
        js_sys::Date::now() as u64
    }

    fn format_timestamp(&self, timestamp: u64) -> String {
        let date = js_sys::Date::new(&(timestamp as f64).into());
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            date.get_hours(),
            date.get_minutes(),
            date.get_seconds(),
            date.get_milliseconds()
        )
    }
}

impl crate::domain::logging::Logger for LeptosLogger {
    fn log(&self, entry: crate::domain::logging::LogEntry) {
        use crate::domain::logging::get_time_provider;

        if entry.level < MIN_LOG_LEVEL {
            return;
        }

        let timestamp_str = get_time_provider().format_timestamp(entry.timestamp);
        let formatted = format!(
            "[{}] {} {}: {}",
            timestamp_str, entry.level, entry.component, entry.message
        );

        // Обновляем глобальные Leptos сигналы!
        GLOBAL_LOGS.with(|logs| {
            IS_LOG_PAUSED.with(|paused| {
                if !paused.get() {
                    logs.update(|log_vec| {
                        log_vec.push(formatted);
                        // Ограничиваем до 100 логов
                        while log_vec.len() > 100 {
                            log_vec.remove(0);
                        }
                    });
                }
            });
        });
    }
}

/// 🦀 Главный компонент Bitcoin Chart на Leptos
#[component]
pub fn app() -> impl IntoView {
    // 🚀 Инициализируем глобальный логгер при старте приложения
    use crate::domain::logging::{init_logger, init_time_provider};

    // Добавляем console.log для диагностики
    web_sys::console::log_1(&"🚀 Starting Bitcoin Chart App".into());

    // Инициализируем логгер только один раз
    std::sync::Once::new().call_once(|| {
        // Создаем и устанавливаем Leptos логгер
        init_logger(Box::new(LeptosLogger));

        // Создаем и устанавливаем Web time provider
        init_time_provider(Box::new(WebTimeProvider));

        web_sys::console::log_1(&"✅ Logger initialized".into());

        get_logger().info(
            LogComponent::Presentation("App"),
            "🚀 Global logger and time provider initialized!",
        );
    });

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
                color: #72c685;
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
                color: #72c685;
                font-size: 14px;
                text-align: center;
            }
            
            .debug-console {
                background: rgba(0, 0, 0, 0.8);
                border-radius: 10px;
                padding: 15px;
                max-height: 300px;
                overflow-y: auto;
                border: 1px solid #4a5d73;
            }
            
            .debug-header {
                display: flex;
                justify-content: space-between;
                align-items: center;
                margin-bottom: 10px;
                color: #72c685;
                font-weight: bold;
            }
            
            .debug-btn {
                background: #4a5d73;
                color: white;
                border: none;
                padding: 5px 10px;
                border-radius: 5px;
                cursor: pointer;
                font-size: 12px;
                margin-left: 5px;
            }
            
            .debug-btn:hover {
                background: #5a6d83;
            }
            
            .debug-log {
                font-family: 'Courier New', monospace;
                font-size: 11px;
                line-height: 1.3;
            }
            
            .log-line {
                color: #e0e0e0;
                margin: 2px 0;
                padding: 1px 5px;
                border-radius: 3px;
            }
            
            .log-line:hover {
                background: rgba(255, 255, 255, 0.1);
            }
            "#}
        </style>
        <div class="bitcoin-chart-app">
            <Header />
            <ChartContainer />
            <DebugConsole />
        </div>
    }
}

/// 📊 Заголовок с информацией о цене - теперь с реальными данными!
#[component]
fn header() -> impl IntoView {
    // Используем глобальные сигналы для реальных данных
    let current_price = GLOBAL_CURRENT_PRICE.with(|price| *price);
    let candle_count = GLOBAL_CANDLE_COUNT.with(|count| *count);
    let is_streaming = GLOBAL_IS_STREAMING.with(|streaming| *streaming);
    let max_volume = GLOBAL_MAX_VOLUME.with(|volume| *volume);
    let zoom_level = ZOOM_LEVEL.with(|zoom| *zoom);

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
        let candles = chart.with(|c| c.data.get_candles().clone());
        if candles.is_empty() {
            return vec![];
        }
        let max_visible = 300;
        let start_idx = if candles.len() > max_visible {
            candles.len() - max_visible
        } else {
            0
        };
        let (min, max) = candles
            .iter()
            .skip(start_idx)
            .fold((f64::MAX, f64::MIN), |(min, max), c| {
                (min.min(c.ohlcv.low.value()), max.max(c.ohlcv.high.value()))
            });
        let step = (max - min) / 8.0;
        (0..=8)
            .rev()
            .map(|i| min + i as f64 * step)
            .collect::<Vec<_>>()
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

/// ⏰ Временная шкала снизу графика
#[component]
fn TimeScale(chart: RwSignal<Chart>) -> impl IntoView {
    let time_labels = move || {
        let candles = chart.with(|c| c.data.get_candles().clone());
        if candles.is_empty() {
            return vec![];
        }

        let max_visible = 300;
        let start_idx = if candles.len() > max_visible {
            candles.len() - max_visible
        } else {
            0
        };

        // Показываем 5 временных меток
        let num_labels = 5;
        let mut labels = Vec::new();

        for i in 0..num_labels {
            let index = (i * (candles.len() - start_idx)) / (num_labels - 1);
            if let Some(candle) = candles
                .iter()
                .skip(start_idx)
                .nth(index.min(candles.len() - start_idx - 1))
            {
                let timestamp = candle.timestamp.value();
                // Конвертируем timestamp в читаемое время
                let date = js_sys::Date::new(&(timestamp as f64).into());
                let time_str = format!("{:02}:{:02}", date.get_hours(), date.get_minutes());
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

/// 🎨 Контейнер для WebGPU графика
#[component]
fn ChartContainer() -> impl IntoView {
    // Сигналы для графика
    let chart = create_rw_signal(Chart::new(
        "leptos-chart".to_string(),
        crate::domain::chart::ChartType::Candlestick,
        1000,
    ));
    let (renderer, set_renderer) = create_signal::<Option<Rc<RefCell<WebGpuRenderer>>>>(None);
    let (status, set_status) = create_signal("Initializing...".to_string());

    // Ссылка на canvas элемент
    let canvas_ref = create_node_ref::<Canvas>();

    // Эффект для инициализации WebGPU после монтирования
    create_effect(move |_| {
        if canvas_ref.get().is_some() {
            spawn_local(async move {
                web_sys::console::log_1(&"🔍 Canvas found, starting WebGPU init...".into());
                set_status.set("🚀 Initializing WebGPU renderer...".to_string());

                // Детальная диагностика WebGPU
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

                        // Запускаем WebSocket после инициализации renderer
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

                        // Fallback: показываем хотя бы данные без графика
                        get_logger().info(
                            LogComponent::Infrastructure("Fallback"),
                            "🔄 Starting fallback mode without WebGPU...",
                        );

                        // Создаем тестовые данные для демонстрации
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

    // Эффект для рендеринга при изменении данных
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

    // 🎯 Mouse events для tooltip
    let handle_mouse_move = {
        let chart_signal = chart;
        let status_clone = set_status.clone();
        move |event: web_sys::MouseEvent| {
            let mouse_x = event.offset_x() as f64;
            let mouse_y = event.offset_y() as f64;

            // 🔍 Обработка панорамирования
            IS_DRAGGING.with(|dragging| {
                if dragging.get() {
                    LAST_MOUSE_X.with(|last_x| {
                        let delta_x = mouse_x - last_x.get();
                        PAN_OFFSET.with(|offset| {
                            let pan_sensitivity = ZOOM_LEVEL.with(|z| z.with_untracked(|val| *val)) * 0.001;
                            offset.update(|o| *o += delta_x * pan_sensitivity);
                        });
                        last_x.set(mouse_x);
                    });

                    LAST_MOUSE_Y.with(|last_y| {
                        let delta_y = mouse_y - last_y.get();
                        chart_signal.update(|ch| {
                            let factor = delta_y as f32 / ch.viewport.height as f32;
                            ch.pan(0.0, factor);
                        });
                        last_y.set(mouse_y);
                    });

                    let need_history = PAN_OFFSET.with(|p| p.with_untracked(|val| *val <= -950.0));
                    if need_history {
                        fetch_more_history(chart_signal, status_clone);
                    }
                    return; // При драге не показываем tooltip
                }
            });

            // Конвертируем в NDC координаты (предполагаем canvas 800x500)
            let canvas_width = 800.0;
            let canvas_height = 500.0;
            let ndc_x = (mouse_x / canvas_width) * 2.0 - 1.0;
            let _ndc_y = 1.0 - (mouse_y / canvas_height) * 2.0;

            chart_signal.with(|ch| {
                let candles = ch.data.get_candles();
                if !candles.is_empty() {
                    let max_visible = 300;
                    let start_idx = if candles.len() > max_visible {
                        candles.len() - max_visible
                    } else {
                        0
                    };
                    let visible: Vec<_> = candles.iter().skip(start_idx).collect();

                    let step_size = 2.0 / visible.len() as f64;
                    let candle_idx = ((ndc_x + 1.0) / step_size).floor() as usize;

                    if candle_idx < visible.len() {
                        let candle = visible[candle_idx];
                        let tooltip_data = TooltipData::new(candle.clone(), mouse_x, mouse_y);

                        TOOLTIP_DATA.with(|data| data.set(Some(tooltip_data)));
                        TOOLTIP_VISIBLE.with(|visible| visible.set(true));
                    } else {
                        TOOLTIP_VISIBLE.with(|visible| visible.set(false));
                    }
                } else {
                    TOOLTIP_VISIBLE.with(|visible| visible.set(false));
                }
            });
        }
    };

    let handle_mouse_leave = move |_event: web_sys::MouseEvent| {
        TOOLTIP_VISIBLE.with(|visible| visible.set(false));
        IS_DRAGGING.with(|dragging| dragging.set(false));
    };

    // 🔍 Зум колесиком мыши - упрощенная версия без эффектов
    let handle_wheel = {
        let chart_signal = chart;
        let renderer_clone = renderer.clone();
        let status_clone = set_status.clone();
        move |event: web_sys::WheelEvent| {
            web_sys::console::log_1(&format!("🖱️ Wheel event: delta_y={}", event.delta_y()).into());

            let delta_y = event.delta_y();
            let zoom_factor = if delta_y < 0.0 { 1.1 } else { 0.9 }; // Zoom in/out

            ZOOM_LEVEL.with(|zoom| {
                let old_zoom = zoom.with_untracked(|z| *z);
                zoom.update(|z| {
                    *z *= zoom_factor;
                    *z = z.max(0.1).min(10.0); // Ограничиваем зум от 0.1x до 10x
                });
                let new_zoom = zoom.with_untracked(|z| *z);
                web_sys::console::log_1(
                    &format!("🔍 Zoom: {:.2}x -> {:.2}x", old_zoom, new_zoom).into(),
                );

                // Сразу применяем зум без эффектов
                chart_signal.with_untracked(|ch| {
                    if ch.get_candle_count() > 0 {
                        renderer_clone.with_untracked(|renderer_opt| {
                            if let Some(renderer_rc) = renderer_opt {
                                if let Ok(mut webgpu_renderer) = renderer_rc.try_borrow_mut() {
                                    webgpu_renderer.set_zoom_params(
                                        new_zoom,
                                        PAN_OFFSET.with(|p| p.with_untracked(|val| *val)),
                                    );

                                    let _ = webgpu_renderer.render(ch);

                                    get_logger().info(
                                        LogComponent::Infrastructure("ZoomWheel"),
                                        &format!(
                                            "✅ Applied zoom {:.2}x to WebGPU renderer",
                                            new_zoom
                                        ),
                                    );
                                }
                            }
                        });
                    }
                });
            });

            get_logger().info(
                LogComponent::Presentation("ChartZoom"),
                &format!(
                    "🔍 Zoom level: {:.2}x",
                    ZOOM_LEVEL.with(|z| z.with_untracked(|z_val| *z_val))
                ),
            );
            let need_history = PAN_OFFSET.with(|p| p.with_untracked(|val| *val <= -950.0));
            if need_history {
                fetch_more_history(chart_signal, status_clone);
            }
        }
    };

    // 🖱️ Начало панорамирования
    let handle_mouse_down = move |event: web_sys::MouseEvent| {
        if event.button() == 0 {
            // Левая кнопка мыши
            IS_DRAGGING.with(|dragging| dragging.set(true));
            LAST_MOUSE_X.with(|last_x| last_x.set(event.offset_x() as f64));
            LAST_MOUSE_Y.with(|last_y| last_y.set(event.offset_y() as f64));

            // Даем canvas фокус для клавиатурных событий
            if let Some(target) = event.target() {
                if let Ok(canvas) = target.dyn_into::<web_sys::HtmlCanvasElement>() {
                    let _ = canvas.focus();
                }
            }
        }
    };

    // 🖱️ Конец панорамирования
    let handle_mouse_up = move |_event: web_sys::MouseEvent| {
        IS_DRAGGING.with(|dragging| dragging.set(false));
    };

    // ⌨️ Клавиши для зума (+/- и PageUp/PageDown)
    let handle_keydown = {
        let chart_signal = chart;
        let renderer_clone = renderer.clone();
        let status_clone = set_status.clone();
        move |event: web_sys::KeyboardEvent| {
            let key = event.key();
            let mut zoom_changed = false;

            match key.as_str() {
                "+" | "=" => {
                    event.prevent_default();
                    ZOOM_LEVEL.with(|zoom| {
                        zoom.update(|z| {
                            *z *= 1.2;
                            *z = z.min(10.0);
                        });
                    });
                    zoom_changed = true;
                }
                "-" | "_" => {
                    event.prevent_default();
                    ZOOM_LEVEL.with(|zoom| {
                        zoom.update(|z| {
                            *z *= 0.8;
                            *z = z.max(0.1);
                        });
                    });
                    zoom_changed = true;
                }
                "PageUp" => {
                    event.prevent_default();
                    ZOOM_LEVEL.with(|zoom| {
                        zoom.update(|z| {
                            *z *= 1.5;
                            *z = z.min(10.0);
                        });
                    });
                    zoom_changed = true;
                }
                "PageDown" => {
                    event.prevent_default();
                    ZOOM_LEVEL.with(|zoom| {
                        zoom.update(|z| {
                            *z *= 0.67;
                            *z = z.max(0.1);
                        });
                    });
                    zoom_changed = true;
                }
                _ => {}
            }

            if zoom_changed {
                let new_zoom = ZOOM_LEVEL.with(|z| z.with_untracked(|z_val| *z_val));
                web_sys::console::log_1(&format!("⌨️ Keyboard zoom: {:.2}x", new_zoom).into());

                // Применяем зум к renderer для клавиатурных команд
                chart_signal.with_untracked(|ch| {
                    if ch.get_candle_count() > 0 {
                        renderer_clone.with_untracked(|renderer_opt| {
                            if let Some(renderer_rc) = renderer_opt {
                                if let Ok(mut webgpu_renderer) = renderer_rc.try_borrow_mut() {
                                    webgpu_renderer.set_zoom_params(
                                        new_zoom,
                                        PAN_OFFSET.with(|p| p.with_untracked(|val| *val)),
                                    );

                                    let _ = webgpu_renderer.render(ch);

                                    get_logger().info(
                                        LogComponent::Infrastructure("KeyboardZoom"),
                                        &format!(
                                            "⌨️ Applied keyboard zoom {:.2}x to WebGPU renderer",
                                            new_zoom
                                        ),
                                    );
                                }
                            }
                        });
                    }
                });

                get_logger().info(
                    LogComponent::Presentation("KeyboardZoom"),
                    &format!("⌨️ Zoom level: {:.2}x", new_zoom),
                );
                let need_history = PAN_OFFSET.with(|p| p.with_untracked(|val| *val <= -950.0));
                if need_history {
                    fetch_more_history(chart_signal, status_clone);
                }
            }
        }
    };

    // Эффект зума удален - теперь зум обрабатывается прямо в wheel handler

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
                        style="border: 2px solid #4a5d73; border-radius: 10px; background: #2c3e50; cursor: crosshair; outline: none;"
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

            // Временная шкала под графиком
            <div style="display: flex; justify-content: center; margin-top: 10px;">
                <TimeScale chart=chart />
            </div>

            <div class="status">
                {move || status.get()}
            </div>

            // Подсказки по управлению
            <div style="text-align: center; margin-top: 10px; font-size: 12px; color: #888;">
                "🔍 Zoom: Mouse wheel, +/- keys, PageUp/PageDown | 🖱️ Pan: Left click + drag | 🎯 Tooltip: Mouse hover"
            </div>
        </div>
    }
}

/// 💰 Ценовая шкала справа от графика
#[component]
fn PriceScale() -> impl IntoView {
    let current_price = GLOBAL_CURRENT_PRICE.with(|price| *price);

    // Вычисляем ценовые уровни для отображения (такие же как в сетке)
    let price_levels = move || {
        let price = current_price.get();
        if price <= 0.0 {
            return vec![];
        }

        // Примерный диапазон цен (±3% от текущей цены)
        let min_price = price * 0.97;
        let max_price = price * 1.03;
        let price_range = max_price - min_price;

        // 8 ценовых уровней (как в сетке)
        let num_levels = 8;
        let mut levels = Vec::new();

        for i in 0..=num_levels {
            let level_price = min_price + (price_range * i as f64 / num_levels as f64);
            let position_percent = (i as f64 / num_levels as f64) * 100.0;
            levels.push((level_price, position_percent));
        }

        levels.reverse(); // Сверху вниз
        levels
    };

    view! {
        <div class="price-scale">
            // Показываем ценовые уровни
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

            // Показываем текущую цену (более заметно)
            <div class="current-price-label" style=format!("top: 50%")>
                <span class="price-value">{move || format!("${:.2}", current_price.get())}</span>
            </div>
        </div>
    }
}

/// 🎯 Chart Tooltip компонент - теперь внутри chart-wrapper
#[component]
fn ChartTooltip() -> impl IntoView {
    let tooltip_visible = TOOLTIP_VISIBLE.with(|visible| *visible);
    let tooltip_data = TOOLTIP_DATA.with(|data| *data);

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

/// 🎯 Отладочная консоль с bridge к domain::logging
#[component]
fn DebugConsole() -> impl IntoView {
    // Используем глобальные сигналы вместо локальных!
    let logs = GLOBAL_LOGS.with(|logs| *logs);
    let is_paused = IS_LOG_PAUSED.with(|paused| *paused);

    // Логируем инициализацию компонента
    get_logger().info(
        LogComponent::Presentation("DebugConsole"),
        "🎯 Debug console component initialized",
    );

    view! {
        <div class="debug-console">
            <div class="debug-header">
                <span>"🐛 Domain Logger Console"</span>
                <button
                    on:click=move |_| {
                        is_paused.update(|p| *p = !*p);
                        if is_paused.get() {
                            get_logger().info(
                                LogComponent::Presentation("DebugConsole"),
                                "🛑 Logging paused"
                            );
                        } else {
                            get_logger().info(
                                LogComponent::Presentation("DebugConsole"),
                                "▶️ Logging resumed"
                            );
                        }
                    }
                    class="debug-btn"
                >
                    {move || if is_paused.get() { "▶️ Resume" } else { "⏸️ Pause" }}
                </button>
                <button
                    on:click=move |_| {
                        GLOBAL_LOGS.with(|logs| logs.set(Vec::new()));
                        get_logger().info(
                            LogComponent::Presentation("DebugConsole"),
                            "🗑️ Log history cleared"
                        );
                    }
                    class="debug-btn"
                >
                    "🗑️ Clear"
                </button>
            </div>
            <div class="debug-log">
                <For
                    each=move || logs.get()
                    key=|log| log.clone()
                    children=move |log| {
                        view! { <div class="log-line">{log}</div> }
                    }
                />
            </div>
        </div>
    }
}

/// 🌐 Запуск WebSocket стрима в Leptos с обновлением глобальных сигналов
async fn start_websocket_stream(chart: RwSignal<Chart>, set_status: WriteSignal<String>) {
    let symbol = Symbol::from("BTCUSDT");
    let interval = TimeInterval::OneMinute;

    // Создаем клиент для загрузки данных
    let ws_client = BinanceWebSocketClient::new(symbol, interval);

    // Устанавливаем статус стрима
    GLOBAL_IS_STREAMING.with(|streaming| streaming.set(false));

    // 📈 Сначала загружаем исторические данные
    set_status.set("📈 Loading historical data...".to_string());

    match ws_client.fetch_historical_data(300).await {
        Ok(historical_candles) => {
            get_logger().info(
                LogComponent::Presentation("WebSocketStream"),
                &format!("✅ Loaded {} historical candles", historical_candles.len()),
            );

            chart.update(|ch| ch.set_historical_data(historical_candles.clone()));

            // Обновляем глобальные сигналы с историческими данными
            let cnt = chart.with(|c| c.get_candle_count());
            GLOBAL_CANDLE_COUNT.with(|count| count.set(cnt));

            if let Some(last_candle) = historical_candles.last() {
                GLOBAL_CURRENT_PRICE.with(|price| {
                    price.set(last_candle.ohlcv.close.value());
                });
            }

            // Вычисляем максимальный объем из истории
            let max_vol = historical_candles
                .iter()
                .map(|c| c.ohlcv.volume.value())
                .fold(0.0f64, |a, b| a.max(b));
            GLOBAL_MAX_VOLUME.with(|volume| volume.set(max_vol));

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

    // 🔌 Теперь запускаем WebSocket для real-time обновлений
    set_status.set("🔌 Starting WebSocket stream...".to_string());
    GLOBAL_IS_STREAMING.with(|streaming| streaming.set(true));

    let mut ws_client =
        BinanceWebSocketClient::new(Symbol::from("BTCUSDT"), TimeInterval::OneMinute);

    spawn_local(async move {
        let handler = move |candle: Candle| {
            // Обновляем цену в глобальном сигнале
            GLOBAL_CURRENT_PRICE.with(|price| {
                price.set(candle.ohlcv.close.value() as f64);
            });

            chart.update(|ch| {
                ch.add_realtime_candle(candle.clone());
            });

            let count = chart.with(|c| c.get_candle_count());
            GLOBAL_CANDLE_COUNT.with(|cnt| cnt.set(count));

            let max_vol = chart.with(|c| {
                c.data
                    .get_candles()
                    .iter()
                    .map(|c| c.ohlcv.volume.value())
                    .fold(0.0f64, |a, b| a.max(b))
            });
            GLOBAL_MAX_VOLUME.with(|volume| volume.set(max_vol));

            // Обновляем статус
            set_status.set("🌐 WebSocket LIVE • Real-time updates".to_string());
        };

        if let Err(e) = ws_client.start_stream(handler).await {
            set_status.set(format!("❌ WebSocket error: {}", e));
            GLOBAL_IS_STREAMING.with(|streaming| streaming.set(false));
        }
    });
}
