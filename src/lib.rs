use wasm_bindgen::prelude::*;

use crate::domain::market_data::{Symbol, TimeInterval, Candle};
use crate::infrastructure::websocket::{BinanceWebSocketClientWithCallback, BinanceHttpClient};

pub mod domain;
pub mod infrastructure;
pub mod application;  // Production-ready application layer
pub mod presentation;

/// Initialize the application with proper DDD architecture
#[wasm_bindgen(start)]
pub fn initialize() {
    // Initialize logger with infrastructure implementation
    let console_logger = Box::new(infrastructure::services::ConsoleLogger::new_development());
    domain::logging::init_logger(console_logger);
    
    // Initialize time provider with browser implementation
    let browser_time_provider = Box::new(infrastructure::services::BrowserTimeProvider::new());
    domain::logging::init_time_provider(browser_time_provider);
    
    use domain::logging::{LogComponent, get_logger};
    get_logger().info(
        LogComponent::Presentation("Initialize"),
        "🚀 DDD Architecture initialized successfully"
    );
}

/// Simple test for historical data loading
#[wasm_bindgen]
pub async fn test_historical_data() -> Result<(), JsValue> {
    #[allow(unused_unsafe)]
    unsafe {
        web_sys::console::log_1(&"🧪 Testing historical data loading...".into());
    }
    
    let http_client = BinanceHttpClient::new();
    let symbol = Symbol::from("BTCUSDT");
    let interval = TimeInterval::OneMinute;
    let limit = 200;
    
    match http_client.get_recent_candles(&symbol, interval, limit).await {
        Ok(candles) => {
            #[allow(unused_unsafe)]
            unsafe {
                web_sys::console::log_1(&format!(
                    "✅ Successfully loaded {} historical candles!",
                    candles.len()
                ).into());
            }
            
            // Log first and last candle
            if let Some(first) = candles.first() {
                #[allow(unused_unsafe)]
                unsafe {
                    web_sys::console::log_1(&format!(
                        "📊 First candle: {} O:{} H:{} L:{} C:{} V:{}",
                        first.timestamp.value(),
                        first.ohlcv.open.value(),
                        first.ohlcv.high.value(),
                        first.ohlcv.low.value(),
                        first.ohlcv.close.value(),
                        first.ohlcv.volume.value()
                    ).into());
                }
            }
            
            if let Some(last) = candles.last() {
                #[allow(unused_unsafe)]
                unsafe {
                    web_sys::console::log_1(&format!(
                        "📊 Last candle: {} O:{} H:{} L:{} C:{} V:{}",
                        last.timestamp.value(),
                        last.ohlcv.open.value(),
                        last.ohlcv.high.value(),
                        last.ohlcv.low.value(),
                        last.ohlcv.close.value(),
                        last.ohlcv.volume.value()
                    ).into());
                }
            }
            
            // Calculate price range for visualization planning
            if candles.len() > 1 {
                let prices: Vec<f32> = candles.iter().map(|c| c.ohlcv.close.value()).collect();
                let min_price = prices.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max_price = prices.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                
                #[allow(unused_unsafe)]
                unsafe {
                    web_sys::console::log_1(&format!(
                        "📈 Price range: ${:.2} - ${:.2} (${:.2} range)",
                        min_price,
                        max_price,
                        max_price - min_price
                    ).into());
                }
                
                // Show time range
                let start_time = candles.first().unwrap().timestamp.value();
                let end_time = candles.last().unwrap().timestamp.value();
                let time_span_ms = end_time - start_time;
                let time_span_minutes = time_span_ms as f64 / 1000.0 / 60.0; // minutes
                
                #[allow(unused_unsafe)]
                unsafe {
                    web_sys::console::log_1(&format!(
                        "⏰ Time span: {:.0} minutes ({:.1} hours)",
                        time_span_minutes,
                        time_span_minutes / 60.0
                    ).into());
                }
            }
            
            Ok(())
        }
        Err(e) => {
            #[allow(unused_unsafe)]
            unsafe {
                web_sys::console::error_1(&format!("❌ Failed to load historical data: {:?}", e).into());
            }
            Err(e)
        }
    }
}

/// Original WebSocket demo
#[wasm_bindgen]
pub async fn start_websocket_demo() -> Result<(), JsValue> {
    #[allow(unused_unsafe)]
    unsafe {
        web_sys::console::log_1(&"🚀 Starting WebSocket demo...".into());
    }
    
    let mut client = BinanceWebSocketClientWithCallback::new();
    
    let callback = |candle: Candle| {
        #[allow(unused_unsafe)]
        unsafe {
            web_sys::console::log_1(&format!(
                "📡 Live candle: {} O:{} H:{} L:{} C:{} V:{}",
                candle.timestamp.value(),
                candle.ohlcv.open.value(),
                candle.ohlcv.high.value(),
                candle.ohlcv.low.value(),
                candle.ohlcv.close.value(),
                candle.ohlcv.volume.value()
            ).into());
        }
    };
    
    client.connect_with_callback("btcusdt", "1m", callback)?;
    
    Ok(())
}

/// Combined demo: historical + live
#[wasm_bindgen]
pub async fn start_combined_demo() -> Result<(), JsValue> {
    #[allow(unused_unsafe)]
    unsafe {
        web_sys::console::log_1(&"🎯 Starting combined demo: Historical + Live data".into());
    }
    
    // 1. Load historical data first
    #[allow(unused_unsafe)]
    unsafe {
        web_sys::console::log_1(&"📊 Step 1: Loading historical data...".into());
    }
    test_historical_data().await?;
    
    // 2. Then connect to live WebSocket
    #[allow(unused_unsafe)]
    unsafe {
        web_sys::console::log_1(&"📡 Step 2: Connecting to live WebSocket...".into());
    }
    start_websocket_demo().await?;
    
    #[allow(unused_unsafe)]
    unsafe {
        web_sys::console::log_1(&"✅ Combined demo started successfully!".into());
    }
    
    Ok(())
} 