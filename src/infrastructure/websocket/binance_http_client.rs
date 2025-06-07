use gloo::{console, net::http::Request};
use serde::{Deserialize, Serialize};
use crate::domain::{
    market_data::{
        entities::{Candle, OHLCV},
        value_objects::{Price, Volume, Timestamp, Symbol, TimeInterval},
    },
    logging::{LogComponent, get_logger},
};
use wasm_bindgen::prelude::*;

/// Binance HTTP клиент на основе gloo
pub struct BinanceHttpClient {
    base_url: String,
}

#[derive(Debug, Deserialize)]
pub struct BinanceKlineResponse(Vec<serde_json::Value>);

impl BinanceHttpClient {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.binance.com".to_string(),
        }
    }

    pub fn with_testnet() -> Self {
        Self {
            base_url: "https://testnet.binance.vision".to_string(),
        }
    }

    /// Получение исторических данных свечей
    pub async fn get_historical_klines(
        &self,
        symbol: &Symbol,
        interval: TimeInterval,
        limit: Option<usize>,
    ) -> Result<Vec<Candle>, String> {
        let limit = limit.unwrap_or(500).min(1000); // Binance limit
        
        let url = format!(
            "{}/api/v3/klines?symbol={}&interval={}&limit={}",
            self.base_url,
            symbol.value(),
            interval.to_binance_str(),
            limit
        );

        get_logger().info(
            LogComponent::Infrastructure("BinanceHTTP"),
            &format!("📡 Fetching historical data: {}", url)
        );

        let response = Request::get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {:?}", e))?;

        if !response.ok() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let data = response
            .json::<BinanceKlineResponse>()
            .await
            .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

        // Преобразуем ответ в свечи
        let mut candles = Vec::new();
        for kline_array in data.0.iter() {
            if let Some(candle) = self.parse_kline_array(kline_array) {
                candles.push(candle);
            }
        }

        get_logger().info(
            LogComponent::Infrastructure("BinanceHTTP"),
            &format!("✅ Received {} historical candles for {}", candles.len(), symbol.value())
        );

        Ok(candles)
    }

    /// Парсинг массива данных свечи от Binance
    fn parse_kline_array(&self, kline: &serde_json::Value) -> Option<Candle> {
        if let Some(array) = kline.as_array() {
            if array.len() >= 11 {
                // Binance kline format: [timestamp, open, high, low, close, volume, ...]
                let timestamp = array[0].as_u64()?;
                let open = array[1].as_str()?.parse::<f32>().ok()?;
                let high = array[2].as_str()?.parse::<f32>().ok()?;
                let low = array[3].as_str()?.parse::<f32>().ok()?;
                let close = array[4].as_str()?.parse::<f32>().ok()?;
                let volume = array[5].as_str()?.parse::<f32>().ok()?;

                let ohlcv = OHLCV::new(
                    Price::new(open),
                    Price::new(high),
                    Price::new(low),
                    Price::new(close),
                    Volume::new(volume),
                );

                let candle = Candle::new(
                    Timestamp::new(timestamp),
                    ohlcv,
                );

                return Some(candle);
            }
        }
        None
    }

    /// Получение текущей цены
    pub async fn get_current_price(&self, symbol: &Symbol) -> Result<Price, String> {
        let url = format!(
            "{}/api/v3/ticker/price?symbol={}",
            self.base_url,
            symbol.value()
        );

        let response = Request::get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {:?}", e))?;

        if !response.ok() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        #[derive(Deserialize)]
        struct PriceResponse {
            price: String,
        }

        let price_data = response
            .json::<PriceResponse>()
            .await
            .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

        let price_value = price_data.price.parse::<f32>()
            .map_err(|_| "Invalid price format")?;

        Ok(Price::new(price_value))
    }

    /// Получение 24h статистики
    pub async fn get_24hr_stats(&self, symbol: &Symbol) -> Result<String, String> {
        let url = format!(
            "{}/api/v3/ticker/24hr?symbol={}",
            self.base_url,
            symbol.value()
        );

        let response = Request::get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {:?}", e))?;

        if !response.ok() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        response.text()
            .await
            .map_err(|e| format!("Failed to get text: {:?}", e))
    }
}

impl Default for BinanceHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Экспортируемая функция для JavaScript
#[wasm_bindgen]
pub async fn test_binance_http() -> Result<(), JsValue> {
    get_logger().info(
        LogComponent::Infrastructure("BinanceHTTP"),
        "🧪 Testing Binance HTTP with gloo..."
    );
    
    // Используем HTTP client из http модуля
    use crate::infrastructure::http::BinanceHttpClient;
    let client = BinanceHttpClient::new();
    
    // Получаем исторические данные
    let symbol = Symbol::from("BTCUSDT");
    let interval = TimeInterval::OneMinute;
    
    match client.get_recent_candles(&symbol, interval, 5).await {
        Ok(candles) => {
            get_logger().info(
                LogComponent::Infrastructure("BinanceHTTP"),
                &format!("✅ HTTP test successful: got {} candles", candles.len())
            );
        }
        Err(e) => {
            get_logger().error(
                LogComponent::Infrastructure("BinanceHTTP"),
                &format!("❌ HTTP test failed: {:?}", e)
            );
            return Err(JsValue::from_str(&format!("{:?}", e)));
        }
    }
    
    get_logger().info(
        LogComponent::Infrastructure("BinanceHTTP"),
        "✅ Binance HTTP test completed"
    );
    Ok(())
} 