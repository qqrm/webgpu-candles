use wasm_bindgen::prelude::*;

use crate::domain::market_data::{Symbol, TimeInterval};
use crate::infrastructure::http::BinanceHttpClient;
use crate::domain::logging::{LogComponent, get_logger};

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
    
    get_logger().info(
        LogComponent::Presentation("Initialize"),
        "🚀 DDD Architecture initialized successfully"
    );
}

/// Simple test for historical data loading
#[wasm_bindgen]
pub async fn test_historical_data() -> Result<(), JsValue> {
    get_logger().info(
        LogComponent::Infrastructure("Test"),
        "🧪 Testing stable historical data loading..."
    );
    
    let http_client = BinanceHttpClient::new();
    let symbol = Symbol::from("BTCUSDT");
    let interval = TimeInterval::OneSecond;
    
    // Загружаем последние 200 тиков
    let limit = 200;
    
    match http_client.get_recent_candles(&symbol, interval, limit).await {
        Ok(candles) => {
            get_logger().info(
                LogComponent::Infrastructure("Test"),
                &format!("✅ Successfully loaded {} historical candles!", candles.len())
            );
            
            // Log first and last candle
            if let Some(first) = candles.first() {
                get_logger().info(
                    LogComponent::Infrastructure("Test"),
                    &format!(
                        "📊 First candle: {} O:{} H:{} L:{} C:{} V:{}",
                        first.timestamp.value(),
                        first.ohlcv.open.value(),
                        first.ohlcv.high.value(),
                        first.ohlcv.low.value(),
                        first.ohlcv.close.value(),
                        first.ohlcv.volume.value()
                    )
                );
            }
            
            if let Some(last) = candles.last() {
                get_logger().info(
                    LogComponent::Infrastructure("Test"),
                    &format!(
                        "📊 Last candle: {} O:{} H:{} L:{} C:{} V:{}",
                        last.timestamp.value(),
                        last.ohlcv.open.value(),
                        last.ohlcv.high.value(),
                        last.ohlcv.low.value(),
                        last.ohlcv.close.value(),
                        last.ohlcv.volume.value()
                    )
                );
            }
            
            // Calculate price range for visualization planning
            if candles.len() > 1 {
                let prices: Vec<f32> = candles.iter().map(|c| c.ohlcv.close.value()).collect();
                let min_price = prices.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max_price = prices.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                
                get_logger().info(
                    LogComponent::Infrastructure("Test"),
                    &format!(
                        "📈 Price range: ${:.2} - ${:.2} (${:.2} range)",
                        min_price,
                        max_price,
                        max_price - min_price
                    )
                );
                
                // Show time range
                let start_time = candles.first().unwrap().timestamp.value();
                let end_time = candles.last().unwrap().timestamp.value();
                let time_span_ms = end_time - start_time;
                let time_span_minutes = time_span_ms as f64 / 1000.0 / 60.0; // minutes
                
                get_logger().info(
                    LogComponent::Infrastructure("Test"),
                    &format!(
                        "⏰ Time span: {:.0} minutes ({:.1} hours)",
                        time_span_minutes,
                        time_span_minutes / 60.0
                    )
                );
            }
            
            Ok(())
        }
        Err(e) => {
            get_logger().error(
                LogComponent::Infrastructure("Test"),
                &format!("❌ Failed to load historical data: {:?}", e)
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