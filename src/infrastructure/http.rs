use crate::domain::{
    market_data::{Candle, Symbol, TimeInterval, Timestamp, OHLCV, Price, Volume},
    logging::{LogComponent, get_logger},
    errors::{InfrastructureError, NetworkError},
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use serde_json::Value;
use gloo::utils::format::JsValueSerdeExt;
use gloo::net::http::Request;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP клиент для Binance API с автогенерацией
#[derive(Clone)]
pub struct BinanceHttpClient {
    base_url: String,
}

impl Default for BinanceHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl BinanceHttpClient {
    pub fn new() -> Self {
        Self {
            base_url: "https://api.binance.com".to_string(),
        }
    }

    /// Получить исторические свечи
    pub async fn get_recent_candles(
        &self,
        symbol: &Symbol,
        interval: TimeInterval,
        limit: usize,
    ) -> Result<Vec<Candle>, InfrastructureError> {
        get_logger().info(
            LogComponent::Infrastructure("BinanceHttpClient"),
            &format!("📡 Fetching {} candles for {}-{:?}", limit, symbol.value(), interval)
        );

        let interval_str = Self::interval_to_binance_string(interval);
        let url = format!(
            "{}/api/v3/klines?symbol={}&interval={}&limit={}",
            self.base_url,
            symbol.value(),
            interval_str,
            limit
        );

        // Создаем HTTP запрос с gloo
        let response = Request::get(&url)
            .send()
            .await
            .map_err(|e| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                format!("Failed to send request: {:?}", e)
            )))?;

        if !response.ok() {
            return Err(InfrastructureError::Network(NetworkError::HttpRequestFailed(
                format!("HTTP error: {} - {}", response.status(), response.status_text())
            )));
        }

        // Получаем JSON ответ
        let data: Value = response
            .json()
            .await
            .map_err(|e| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                format!("Failed to parse JSON: {:?}", e)
            )))?;

        // Парсим свечи
        let candles = self.parse_klines_response(data)?;

        get_logger().info(
            LogComponent::Infrastructure("BinanceHttpClient"),
            &format!("✅ Successfully fetched {} candles", candles.len())
        );

        Ok(candles)
    }

    /// Парсинг ответа Binance API
    fn parse_klines_response(&self, data: Value) -> Result<Vec<Candle>, InfrastructureError> {
        let array = data.as_array()
            .ok_or_else(|| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                "Response is not an array".to_string()
            )))?;

        let mut candles = Vec::new();
        
        for item in array {
            let kline = item.as_array()
                .ok_or_else(|| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    "Kline is not an array".to_string()
                )))?;

            if kline.len() < 12 {
                continue; // Skip invalid entries
            }

            let timestamp = kline[0].as_u64()
                .ok_or_else(|| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    "Invalid timestamp".to_string()
                )))?;
            
            let open = kline[1].as_str()
                .ok_or_else(|| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    "Invalid open price".to_string()
                )))?
                .parse::<f32>()
                .map_err(|e| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    format!("Failed to parse open price: {}", e)
                )))?;
                
            let high = kline[2].as_str()
                .ok_or_else(|| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    "Invalid high price".to_string()
                )))?
                .parse::<f32>()
                .map_err(|e| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    format!("Failed to parse high price: {}", e)
                )))?;
                
            let low = kline[3].as_str()
                .ok_or_else(|| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    "Invalid low price".to_string()
                )))?
                .parse::<f32>()
                .map_err(|e| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    format!("Failed to parse low price: {}", e)
                )))?;
                
            let close = kline[4].as_str()
                .ok_or_else(|| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    "Invalid close price".to_string()
                )))?
                .parse::<f32>()
                .map_err(|e| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    format!("Failed to parse close price: {}", e)
                )))?;
                
            let volume = kline[5].as_str()
                .ok_or_else(|| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    "Invalid volume".to_string()
                )))?
                .parse::<f32>()
                .map_err(|e| InfrastructureError::Network(NetworkError::HttpRequestFailed(
                    format!("Failed to parse volume: {}", e)
                )))?;

            let candle = Candle::new(
                Timestamp::from(timestamp),
                OHLCV {
                    open: Price::from(open),
                    high: Price::from(high),
                    low: Price::from(low),
                    close: Price::from(close),
                    volume: Volume::from(volume),
                },
            );

            candles.push(candle);
        }

        Ok(candles)
    }

    /// Конвертирует TimeInterval в строку Binance API
    fn interval_to_binance_string(interval: TimeInterval) -> &'static str {
        match interval {
            TimeInterval::OneMinute => "1m",
            TimeInterval::FiveMinutes => "5m",
            TimeInterval::FifteenMinutes => "15m",
            TimeInterval::ThirtyMinutes => "30m",
            TimeInterval::OneHour => "1h",
            TimeInterval::FourHours => "4h",
            TimeInterval::OneDay => "1d",
            TimeInterval::OneWeek => "1w",
            TimeInterval::OneMonth => "1M",
        }
    }
}

/// HTTP клиент на основе gloo для WASM
pub struct GlooHttpClient {
    base_url: String,
    default_headers: HashMap<String, String>,
    timeout_ms: u32,
}

impl GlooHttpClient {
    pub fn new(base_url: String) -> Self {
        let mut default_headers = HashMap::new();
        default_headers.insert("Content-Type".to_string(), "application/json".to_string());
        default_headers.insert("Accept".to_string(), "application/json".to_string());

        Self {
            base_url,
            default_headers,
            timeout_ms: 30000, // 30 seconds
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u32) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn add_header(mut self, key: String, value: String) -> Self {
        self.default_headers.insert(key, value);
        self
    }

    /// GET запрос
    pub async fn get(&self, endpoint: &str) -> Result<String, String> {
        let url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("{}/{}", self.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'))
        };

        get_logger().debug(
            LogComponent::Infrastructure("HTTP"),
            &format!("🌐 GET: {}", url)
        );

        let mut request = Request::get(&url);

        // Добавляем заголовки
        for (key, value) in &self.default_headers {
            request = request.header(key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {:?}", e))?;

        if !response.ok() {
            let error_msg = format!("HTTP error: {} - {}", response.status(), response.status_text());
            get_logger().error(
                LogComponent::Infrastructure("HTTP"),
                &error_msg
            );
            return Err(error_msg);
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {:?}", e))?;

        get_logger().debug(
            LogComponent::Infrastructure("HTTP"),
            &format!("✅ GET response: {} bytes", text.len())
        );

        Ok(text)
    }

    /// GET запрос с автоматическим парсингом JSON
    pub async fn get_json<T>(&self, endpoint: &str) -> Result<T, String>
    where
        T: for<'de> Deserialize<'de>,
    {
        let url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!("{}/{}", self.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'))
        };

        get_logger().debug(
            LogComponent::Infrastructure("HTTP"),
            &format!("🌐 GET JSON: {}", url)
        );

        let mut request = Request::get(&url);

        // Добавляем заголовки
        for (key, value) in &self.default_headers {
            request = request.header(key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {:?}", e))?;

        if !response.ok() {
            let error_msg = format!("HTTP error: {} - {}", response.status(), response.status_text());
            get_logger().error(
                LogComponent::Infrastructure("HTTP"),
                &error_msg
            );
            return Err(error_msg);
        }

        let data = response
            .json::<T>()
            .await
            .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

        get_logger().debug(
            LogComponent::Infrastructure("HTTP"),
            "✅ GET JSON response parsed successfully"
        );

        Ok(data)
    }

    /// POST запрос
    pub async fn post<T>(&self, endpoint: &str, body: &T) -> Result<String, String>
    where
        T: Serialize,
    {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'));

        get_logger().debug(
            LogComponent::Infrastructure("HTTP"),
            &format!("🌐 POST: {}", url)
        );

        let json_body = serde_json::to_string(body)
            .map_err(|e| format!("Failed to serialize body: {}", e))?;

        let mut request = Request::post(&url);

        // Добавляем заголовки
        for (key, value) in &self.default_headers {
            request = request.header(key, value);
        }

        let response = request
            .body(json_body)
            .map_err(|e| format!("Failed to create request body: {:?}", e))?
            .send()
            .await
            .map_err(|e| format!("Request failed: {:?}", e))?;

        if !response.ok() {
            let error_msg = format!("HTTP error: {} - {}", response.status(), response.status_text());
            get_logger().error(
                LogComponent::Infrastructure("HTTP"),
                &error_msg
            );
            return Err(error_msg);
        }

        let text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {:?}", e))?;

        get_logger().debug(
            LogComponent::Infrastructure("HTTP"),
            &format!("✅ POST response: {} bytes", text.len())
        );

        Ok(text)
    }

    /// POST запрос с автоматическим парсингом JSON ответа
    pub async fn post_json<T, R>(&self, endpoint: &str, body: &T) -> Result<R, String>
    where
        T: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), endpoint.trim_start_matches('/'));

        get_logger().debug(
            LogComponent::Infrastructure("HTTP"),
            &format!("🌐 POST JSON: {}", url)
        );

        let json_body = serde_json::to_string(body)
            .map_err(|e| format!("Failed to serialize body: {}", e))?;

        let mut request = Request::post(&url);

        // Добавляем заголовки
        for (key, value) in &self.default_headers {
            request = request.header(key, value);
        }

        let response = request
            .body(json_body)
            .map_err(|e| format!("Failed to create request body: {:?}", e))?
            .send()
            .await
            .map_err(|e| format!("Request failed: {:?}", e))?;

        if !response.ok() {
            let error_msg = format!("HTTP error: {} - {}", response.status(), response.status_text());
            get_logger().error(
                LogComponent::Infrastructure("HTTP"),
                &error_msg
            );
            return Err(error_msg);
        }

        let data = response
            .json::<R>()
            .await
            .map_err(|e| format!("Failed to parse JSON response: {:?}", e))?;

        get_logger().debug(
            LogComponent::Infrastructure("HTTP"),
            "✅ POST JSON response parsed successfully"
        );

        Ok(data)
    }

    /// Проверка доступности URL
    pub async fn health_check(&self, endpoint: &str) -> bool {
        match self.get(endpoint).await {
            Ok(_) => {
                get_logger().info(
                    LogComponent::Infrastructure("HTTP"),
                    &format!("✅ Health check passed: {}", endpoint)
                );
                true
            },
            Err(e) => {
                get_logger().warn(
                    LogComponent::Infrastructure("HTTP"),
                    &format!("❌ Health check failed: {} - {}", endpoint, e)
                );
                false
            }
        }
    }
}

impl Default for GlooHttpClient {
    fn default() -> Self {
        Self::new("https://api.example.com".to_string())
    }
}

/// Утилиты для HTTP запросов
pub struct HttpUtils;

impl HttpUtils {
    /// Проверка статуса ответа
    pub fn is_success_status(status: u16) -> bool {
        (200..300).contains(&status)
    }

    /// Построение URL с параметрами
    pub fn build_url_with_params(base_url: &str, params: &HashMap<String, String>) -> String {
        if params.is_empty() {
            return base_url.to_string();
        }

        let query_string: String = params
            .iter()
            .map(|(key, value)| format!("{}={}", key, value))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}?{}", base_url, query_string)
    }

    /// Кодирование URL компонента
    pub fn url_encode(input: &str) -> String {
        // Простая реализация URL encoding для основных символов
        input
            .replace(" ", "%20")
            .replace("&", "%26")
            .replace("=", "%3D")
            .replace("?", "%3F")
            .replace("#", "%23")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_building() {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), "BTCUSDT".to_string());
        params.insert("interval".to_string(), "1m".to_string());

        let url = HttpUtils::build_url_with_params("https://api.example.com/data", &params);
        assert!(url.contains("symbol=BTCUSDT"));
        assert!(url.contains("interval=1m"));
    }

    #[test]
    fn test_url_encoding() {
        assert_eq!(HttpUtils::url_encode("hello world"), "hello%20world");
        assert_eq!(HttpUtils::url_encode("a&b=c"), "a%26b%3Dc");
    }
} 