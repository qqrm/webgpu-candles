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

                        // Рендерим
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

    view! {
        <div class="chart-container">
            <canvas 
                id="chart-canvas"
                node_ref=canvas_ref
                width="800"
                height="500"
                style="border: 2px solid #4a5d73; border-radius: 10px; background: #2c3e50;"
            />
            <div class="status">
                {move || status.get()}
            </div>
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
                        logs.set(Vec::new());
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
    
    // Сначала загружаем исторические данные
    match crate::infrastructure::http::BinanceHttpClient::new()
        .get_recent_candles(&symbol, interval, 200).await 
    {
        Ok(historical_candles) => {
            set_candles.set(historical_candles.clone());
            set_status.set(format!("✅ Loaded {} historical candles", historical_candles.len()));
            
            // Обновляем глобальные сигналы с историческими данными
            GLOBAL_CANDLE_COUNT.with(|count| count.set(historical_candles.len()));
            if let Some(last_candle) = historical_candles.last() {
                GLOBAL_CURRENT_PRICE.with(|price| price.set(last_candle.ohlcv.close.value() as f64));
            }
            
            // Теперь запускаем WebSocket
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
        Err(e) => {
            set_status.set(format!("❌ Failed to load historical data: {:?}", e));
            GLOBAL_IS_STREAMING.with(|streaming| streaming.set(false));
        }
    }
} 