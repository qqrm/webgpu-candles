use leptos::*;
use leptos::html::Canvas;
use std::rc::Rc;
use std::cell::RefCell;

use crate::{
    domain::market_data::entities::Candle,
    infrastructure::{
        rendering::WebGpuRenderer,
        websocket::BinanceWebSocketClient,
    },
    domain::{
        chart::Chart,
        market_data::{value_objects::Symbol, TimeInterval},
        logging::{LogComponent, get_logger},
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
    
    // 🎯 Tooltip данные
    static TOOLTIP_DATA: RwSignal<Option<TooltipData>> = create_rw_signal(None);
    static TOOLTIP_VISIBLE: RwSignal<bool> = create_rw_signal(false);
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

impl crate::domain::logging::Logger for LeptosLogger {
    fn log(&self, entry: crate::domain::logging::LogEntry) {
        use crate::domain::logging::get_time_provider;
        
        let timestamp_str = get_time_provider().format_timestamp(entry.timestamp);
        let formatted = format!("[{}] {} {}: {}", 
            timestamp_str, 
            entry.level,
            entry.component,
            entry.message
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
pub fn App() -> impl IntoView {
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
fn Header() -> impl IntoView {
    // Используем глобальные сигналы для реальных данных
    let current_price = GLOBAL_CURRENT_PRICE.with(|price| *price);
    let candle_count = GLOBAL_CANDLE_COUNT.with(|count| *count);
    let is_streaming = GLOBAL_IS_STREAMING.with(|streaming| *streaming);

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
            </div>
        </div>
    }
}

/// 🎨 Контейнер для WebGPU графика
#[component]
fn ChartContainer() -> impl IntoView {
    // Реактивные сигналы для графика
    let (candles, set_candles) = create_signal::<Vec<Candle>>(Vec::new());
    let (renderer, set_renderer) = create_signal::<Option<Rc<RefCell<WebGpuRenderer>>>>(None);
    let (status, set_status) = create_signal("Initializing...".to_string());

    // Ссылка на canvas элемент
    let canvas_ref = create_node_ref::<Canvas>();

    // Эффект для инициализации WebGPU после монтирования
    create_effect(move |_| {
        if canvas_ref.get().is_some() {
            spawn_local(async move {
                set_status.set("🚀 Initializing WebGPU renderer...".to_string());
                
                match WebGpuRenderer::new("chart-canvas", 800, 500).await {
                    Ok(webgpu_renderer) => {
                        let renderer_rc = Rc::new(RefCell::new(webgpu_renderer));
                        set_renderer.set(Some(renderer_rc));
                        set_status.set("✅ WebGPU renderer ready".to_string());
                        
                        // Запускаем WebSocket после инициализации renderer
                        start_websocket_stream(set_candles, set_status).await;
                    }
                    Err(e) => {
                        set_status.set(format!("❌ WebGPU failed: {:?}", e));
                    }
                }
            });
        }
    });

    // Эффект для рендеринга при изменении данных
    create_effect(move |_| {
        candles.with(|candles_data| {
            renderer.with(|renderer_opt| {
                if let Some(renderer_rc) = renderer_opt {
                    if !candles_data.is_empty() {
                        // Создаем Chart и рендерим
                        let mut chart = Chart::new(
                            "leptos-chart".to_string(),
                            crate::domain::chart::ChartType::Candlestick,
                            1000
                        );
                        
                        // Добавляем данные в chart
                        for candle in candles_data {
                            chart.data.add_candle(candle.clone());
                        }

                        // Рендерим реальные свечи (WebGPU работает!)
                        if let Ok(webgpu_renderer) = renderer_rc.try_borrow() {
                            if let Err(e) = webgpu_renderer.render(&chart) {
                                set_status.set(format!("❌ Render error: {:?}", e));
                            } else {
                                set_status.set(format!("✅ Rendered {} candles", candles_data.len()));
                            }
                        }
                    }
                }
            });
        });
    });

    // 🎯 Mouse events для tooltip
    let handle_mouse_move = {
        let candles_clone = candles.clone();
        move |event: web_sys::MouseEvent| {
            // Упрощенная версия без getBoundingClientRect
            let mouse_x = event.offset_x() as f64;
            let mouse_y = event.offset_y() as f64;
            
            // Конвертируем в NDC координаты (предполагаем canvas 800x500)
            let canvas_width = 800.0;
            let canvas_height = 500.0;
            let ndc_x = (mouse_x / canvas_width) * 2.0 - 1.0;
            let _ndc_y = 1.0 - (mouse_y / canvas_height) * 2.0;
            
            candles_clone.with(|candles_data| {
                if !candles_data.is_empty() {
                    let max_visible = 300;
                    let start_idx = if candles_data.len() > max_visible {
                        candles_data.len() - max_visible
                    } else { 0 };
                    let visible = &candles_data[start_idx..];
                    
                    let step_size = 2.0 / visible.len() as f64;
                    let candle_idx = ((ndc_x + 1.0) / step_size).floor() as usize;
                    
                    if candle_idx < visible.len() {
                        let candle = &visible[candle_idx];
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
    };

    view! {
        <div class="chart-container">
            <div class="chart-wrapper">
                <canvas 
                    id="chart-canvas"
                    node_ref=canvas_ref
                    width="800"
                    height="500"
                    style="border: 2px solid #4a5d73; border-radius: 10px; background: #2c3e50; cursor: crosshair;"
                    on:mousemove=handle_mouse_move
                    on:mouseleave=handle_mouse_leave
                />
                <PriceScale />
                <ChartTooltip />
            </div>
            <div class="status">
                {move || status.get()}
            </div>
        </div>
    }
}

/// 💰 Ценовая шкала справа от графика
#[component]
fn PriceScale() -> impl IntoView {
    let current_price = GLOBAL_CURRENT_PRICE.with(|price| *price);
    
    view! {
        <div class="price-scale">
            // Показываем текущую цену
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
async fn start_websocket_stream(
    set_candles: WriteSignal<Vec<Candle>>,
    set_status: WriteSignal<String>,
) {
    set_status.set("🔌 Starting WebSocket stream...".to_string());

    let symbol = Symbol::from("BTCUSDT");
    let interval = TimeInterval::OneMinute;
    
    // Устанавливаем статус стрима
    GLOBAL_IS_STREAMING.with(|streaming| streaming.set(true));
    
    // Запускаем WebSocket сразу - только реальное время!
    let mut ws_client = BinanceWebSocketClient::new(symbol, interval);
    
    spawn_local(async move {
        let handler = move |candle: Candle| {
            // Обновляем цену в глобальном сигнале
            GLOBAL_CURRENT_PRICE.with(|price| {
                price.set(candle.ohlcv.close.value() as f64);
            });
            
            // Реактивно обновляем данные в Leptos!
            set_candles.update(|candles| {
                let new_timestamp = candle.timestamp.value();
                
                if let Some(last_candle) = candles.last_mut() {
                    if last_candle.timestamp.value() == new_timestamp {
                        // Обновляем существующую свечу
                        *last_candle = candle;
                    } else if new_timestamp > last_candle.timestamp.value() {
                        // Добавляем новую свечу
                        candles.push(candle);
                        
                        // Ограничиваем до 300 свечей
                        while candles.len() > 300 {
                            candles.remove(0);
                        }
                    }
                } else {
                    candles.push(candle);
                }
                
                // Обновляем счетчик свечей
                GLOBAL_CANDLE_COUNT.with(|count| count.set(candles.len()));
            });
            
            // Обновляем статус
            set_status.set("🌐 WebSocket LIVE • Real-time updates".to_string());
        };

        if let Err(e) = ws_client.start_stream(handler).await {
            set_status.set(format!("❌ WebSocket error: {}", e));
            GLOBAL_IS_STREAMING.with(|streaming| streaming.set(false));
        }
    });
} 