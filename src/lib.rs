use wasm_bindgen::prelude::*;
use leptos::*;

pub mod domain;
pub mod infrastructure;
pub mod application;
pub mod presentation;
pub mod app;

// Экспорт unified API (для совместимости)
pub use presentation::unified_wasm_api::*;

// Главный компонент Leptos
pub use app::App;

use domain::logging::{LogComponent, get_logger};

/// 🦀 Leptos инициализация приложения
#[wasm_bindgen]
pub fn hydrate() {
    // Настройка panic hook для лучших ошибок
    console_error_panic_hook::set_once();
    
    // Инициализируем логгер
    initialize_logging();
    
    get_logger().info(
        LogComponent::Presentation("LeptosInit"),
        "🚀 Leptos Bitcoin Chart App starting..."
    );

    // Вставляем CSS стили
    inject_styles();

    // Монтируем Leptos приложение
    leptos::mount_to_body(|| {
        view! { <App/> }
    });
}

/// Initialize DDD logging architecture
fn initialize_logging() {
    let console_logger = Box::new(infrastructure::services::ConsoleLogger::new_development());
    domain::logging::init_logger(console_logger);
    
    let browser_time_provider = Box::new(infrastructure::services::BrowserTimeProvider::new());
    domain::logging::init_time_provider(browser_time_provider);
    
    get_logger().info(
        LogComponent::Presentation("Initialize"),
        "🚀 DDD Architecture initialized successfully"
    );
}

/// 🎨 CSS стили встроенные в Rust
fn inject_styles() {
    let css = r#"
        :root {
            --bg-dark: #2c3e50;
            --bg-card: #34495e;
            --text-primary: #ffffff;
            --text-secondary: #bdc3c7;
            --accent-green: #4ade80;
            --accent-red: #ef4444;
            --border-color: #4a5d73;
        }

        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg-dark);
            color: var(--text-primary);
            line-height: 1.6;
        }

        .bitcoin-chart-app {
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }

        .header {
            text-align: center;
            margin-bottom: 30px;
            padding: 20px;
            background: var(--bg-card);
            border-radius: 15px;
            border: 1px solid var(--border-color);
        }

        .header h1 {
            font-size: 2.5rem;
            font-weight: 700;
            margin-bottom: 10px;
        }

        .price-info {
            display: flex;
            justify-content: center;
            gap: 30px;
            margin-top: 15px;
            flex-wrap: wrap;
        }

        .price-item {
            text-align: center;
        }

        .price-value {
            font-size: 1.5rem;
            font-weight: bold;
            color: var(--accent-green);
        }

        .price-label {
            font-size: 0.9rem;
            color: var(--text-secondary);
        }

        .chart-container {
            background: var(--bg-card);
            border-radius: 15px;
            padding: 25px;
            border: 1px solid var(--border-color);
            margin-bottom: 20px;
            text-align: center;
        }

        .status {
            margin-top: 15px;
            color: var(--accent-green);
            font-weight: bold;
        }

        .debug-console {
            background: var(--bg-card);
            border-radius: 15px;
            border: 1px solid var(--border-color);
            overflow: hidden;
        }

        .debug-header {
            background: #1a1a1a;
            padding: 10px 15px;
            font-weight: bold;
            color: var(--accent-green);
            display: flex;
            justify-content: space-between;
            align-items: center;
            flex-wrap: wrap;
        }

        .debug-btn {
            border: none;
            padding: 5px 10px;
            border-radius: 4px;
            cursor: pointer;
            font-size: 12px;
            font-weight: bold;
            background: var(--accent-green);
            color: white;
            margin-left: 5px;
            transition: opacity 0.2s;
        }

        .debug-btn:hover {
            opacity: 0.8;
        }

        .debug-log {
            height: 200px;
            overflow-y: auto;
            padding: 10px;
            font-family: 'Courier New', monospace;
            font-size: 12px;
            line-height: 1.4;
            background: #1a1a1a;
            color: #00ff88;
        }

        .log-line {
            padding: 2px 0;
            border-bottom: 1px solid rgba(255,255,255,0.1);
        }

        /* Responsive design */
        @media (max-width: 768px) {
            .bitcoin-chart-app {
                padding: 10px;
            }
            
            .price-info {
                gap: 15px;
            }
            
            .header h1 {
                font-size: 2rem;
            }
            
            canvas {
                width: 100% !important;
                height: 400px !important;
            }
        }
    "#;

    // Добавляем CSS в head
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(head) = document.head() {
                let style = document.create_element("style").unwrap();
                style.set_text_content(Some(css));
                let _ = head.append_child(&style);
            }
        }
    }
}

// Совместимость с существующим API
#[wasm_bindgen(start)]
pub fn initialize() {
    initialize_logging();
}

/// Test function для совместимости
#[wasm_bindgen]
pub async fn test_historical_data() -> Result<(), JsValue> {
    use crate::domain::market_data::{Symbol, TimeInterval};
    use crate::infrastructure::http::BinanceHttpClient;
    
    get_logger().info(
        LogComponent::Infrastructure("Test"),
        "🧪 Testing historical data from Leptos..."
    );
    
    let http_client = BinanceHttpClient::new();
    let symbol = Symbol::from("BTCUSDT");
    let interval = TimeInterval::OneMinute;
    
    match http_client.get_recent_candles(&symbol, interval, 5).await {
        Ok(candles) => {
            get_logger().info(
                LogComponent::Infrastructure("Test"),
                &format!("✅ Leptos test: loaded {} candles", candles.len())
            );
            Ok(())
        }
        Err(e) => {
            get_logger().error(
                LogComponent::Infrastructure("Test"),
                &format!("❌ Leptos test failed: {:?}", e)
            );
            Err(JsValue::from_str(&format!("{:?}", e)))
        }
    }
}

/// Original WebSocket demo
#[wasm_bindgen]
pub async fn start_websocket_demo() -> Result<(), JsValue> {
    get_logger().info(
        LogComponent::Infrastructure("Demo"),
        "🚀 Starting WebSocket demo..."
    );
    
    // Note: WebSocket client functionality is now in the infrastructure layer
    // This demo is simplified for the current architecture
    
    get_logger().info(
        LogComponent::Infrastructure("Demo"),
        "📡 WebSocket demo functionality moved to application layer"
    );
    
    Ok(())
}

/// Combined demo: historical + live
#[wasm_bindgen]
pub async fn start_combined_demo() -> Result<(), JsValue> {
    get_logger().info(
        LogComponent::Infrastructure("Demo"),
        "🎯 Starting combined demo: Historical + Live data"
    );
    
    // 1. Load historical data first
    get_logger().info(
        LogComponent::Infrastructure("Demo"),
        "📊 Step 1: Loading historical data..."
    );
    test_historical_data().await?;
    
    // 2. Then connect to live WebSocket
    get_logger().info(
        LogComponent::Infrastructure("Demo"),
        "📡 Step 2: Connecting to live WebSocket..."
    );
    start_websocket_demo().await?;
    
    get_logger().info(
        LogComponent::Infrastructure("Demo"),
        "✅ Combined demo started successfully!"
    );
    
    Ok(())
} 