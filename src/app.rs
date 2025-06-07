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
    },
};

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

/// 📊 Заголовок с информацией о цене
#[component]
fn Header() -> impl IntoView {
    // Реактивные сигналы для данных
    let (current_price, set_current_price) = create_signal(0.0);
    let (candle_count, set_candle_count) = create_signal(0);
    let (is_streaming, set_is_streaming) = create_signal(false);

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

/// 🐛 Отладочная консоль
#[component] 
fn DebugConsole() -> impl IntoView {
    let (logs, set_logs) = create_signal::<Vec<String>>(Vec::new());
    let (is_paused, set_is_paused) = create_signal(false);

    view! {
        <div class="debug-console">
            <div class="debug-header">
                <span>"🐛 Leptos Debug Console"</span>
                <button 
                    on:click=move |_| set_is_paused.update(|p| *p = !*p)
                    class="debug-btn"
                >
                    {move || if is_paused.get() { "▶️ Resume" } else { "⏸️ Pause" }}
                </button>
                <button 
                    on:click=move |_| set_logs.set(Vec::new())
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

/// 🌐 Запуск WebSocket стрима в Leptos
async fn start_websocket_stream(
    set_candles: WriteSignal<Vec<Candle>>,
    set_status: WriteSignal<String>,
) {
    set_status.set("🔌 Starting WebSocket stream...".to_string());

    let symbol = Symbol::from("BTCUSDT");
    let interval = TimeInterval::OneMinute;
    
    // Сначала загружаем исторические данные
    match crate::infrastructure::http::BinanceHttpClient::new()
        .get_recent_candles(&symbol, interval, 200).await 
    {
        Ok(historical_candles) => {
            set_candles.set(historical_candles.clone());
            set_status.set(format!("✅ Loaded {} historical candles", historical_candles.len()));
            
            // Теперь запускаем WebSocket
            let mut ws_client = BinanceWebSocketClient::new(symbol, interval);
            
            spawn_local(async move {
                let handler = move |candle: Candle| {
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
                    });
                    
                    // Обновляем статус
                    set_status.set("🌐 WebSocket LIVE • Real-time updates".to_string());
                };

                if let Err(e) = ws_client.start_stream(handler).await {
                    set_status.set(format!("❌ WebSocket error: {}", e));
                }
            });
        }
        Err(e) => {
            set_status.set(format!("❌ Failed to load historical data: {:?}", e));
        }
    }
} 