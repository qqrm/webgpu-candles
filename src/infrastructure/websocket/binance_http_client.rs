use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

use crate::domain::market_data::{Candle, Symbol, TimeInterval, Timestamp, OHLCV, Price, Volume};

// Helper function for logging
fn log(s: &str) {
    #[allow(unused_unsafe)]
    unsafe {
        web_sys::console::log_1(&s.into());
    }
}

/// HTTP клиент для получения исторических данных из Binance REST API
pub struct BinanceHttpClient {
    base_url: String,
}

impl Default for BinanceHttpClient {
    fn default() -> Self {
        Self {
            base_url: "https://api.binance.com".to_string(),
        }
    }
}

impl BinanceHttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_testnet() -> Self {
        Self {
            base_url: "https://testnet.binance.vision".to_string(),
        }
    }

    /// Получить исторические данные свечей
    pub async fn get_historical_klines(
        &self,
        symbol: &Symbol,
        interval: TimeInterval,
        limit: Option<usize>,
        start_time: Option<u64>,
        end_time: Option<u64>,
    ) -> Result<Vec<Candle>, JsValue> {
        let mut url = format!(
            "{}/api/v3/klines?symbol={}&interval={}",
            self.base_url,
            symbol.value().to_uppercase(),
            interval.to_binance_str()
        );

        if let Some(limit) = limit {
            url.push_str(&format!("&limit={}", limit.min(1000))); // Binance max limit is 1000
        }

        if let Some(start_time) = start_time {
            url.push_str(&format!("&startTime={}", start_time));
        }

        if let Some(end_time) = end_time {
            url.push_str(&format!("&endTime={}", end_time));
        }

        log(&format!("📡 HTTP: Fetching historical data from: {}", url));

        let mut opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_mode(RequestMode::Cors);

        let request = Request::new_with_str_and_init(&url, &opts)?;
        request.headers().set("Accept", "application/json")?;

        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
        let resp: Response = resp_value.dyn_into().unwrap();

        if !resp.ok() {
            return Err(JsValue::from_str(&format!(
                "HTTP request failed: {} {}",
                resp.status(),
                resp.status_text()
            )));
        }

        let json = JsFuture::from(resp.json()?).await?;
        
        // Use js_sys to convert JsValue to Vec<Vec<JsValue>>
        let klines_array = js_sys::Array::from(&json);
        let mut candles = Vec::new();

        log(&format!("📊 HTTP: Received {} historical klines", klines_array.length()));

        for i in 0..klines_array.length() {
            let kline_js = klines_array.get(i);
            match self.parse_kline_js_array(&kline_js) {
                Ok(candle) => candles.push(candle),
                Err(e) => {
                    log(&format!("⚠️ HTTP: Failed to parse candle {}: {:?}", i, e));
                    continue;
                }
            }
        }

        log(&format!("✅ HTTP: Successfully parsed {} candles", candles.len()));
        Ok(candles)
    }

    fn parse_kline_js_array(&self, kline_js: &JsValue) -> Result<Candle, JsValue> {
        let array = js_sys::Array::from(kline_js);
        
        if array.length() < 6 {
            return Err(JsValue::from_str("Array too short"));
        }

        let timestamp = array.get(0).as_f64()
            .ok_or_else(|| JsValue::from_str("Invalid timestamp"))? as u64;

        let open = array.get(1).as_string()
            .ok_or_else(|| JsValue::from_str("Invalid open price"))?
            .parse::<f32>()
            .map_err(|e| JsValue::from_str(&format!("Failed to parse open: {}", e)))?;
            
        let high = array.get(2).as_string()
            .ok_or_else(|| JsValue::from_str("Invalid high price"))?
            .parse::<f32>()
            .map_err(|e| JsValue::from_str(&format!("Failed to parse high: {}", e)))?;
            
        let low = array.get(3).as_string()
            .ok_or_else(|| JsValue::from_str("Invalid low price"))?
            .parse::<f32>()
            .map_err(|e| JsValue::from_str(&format!("Failed to parse low: {}", e)))?;
            
        let close = array.get(4).as_string()
            .ok_or_else(|| JsValue::from_str("Invalid close price"))?
            .parse::<f32>()
            .map_err(|e| JsValue::from_str(&format!("Failed to parse close: {}", e)))?;
            
        let volume = array.get(5).as_string()
            .ok_or_else(|| JsValue::from_str("Invalid volume"))?
            .parse::<f32>()
            .map_err(|e| JsValue::from_str(&format!("Failed to parse volume: {}", e)))?;

        let ohlcv = OHLCV::new(
            Price::from(open),
            Price::from(high),
            Price::from(low),
            Price::from(close),
            Volume::from(volume),
        );

        Ok(Candle::new(Timestamp::from(timestamp), ohlcv))
    }

    /// Получить последние N свечей
    pub async fn get_recent_candles(
        &self,
        symbol: &Symbol,
        interval: TimeInterval,
        count: usize,
    ) -> Result<Vec<Candle>, JsValue> {
        self.get_historical_klines(symbol, interval, Some(count), None, None).await
    }

    /// Получить свечи за определенный период
    pub async fn get_candles_for_period(
        &self,
        symbol: &Symbol,
        interval: TimeInterval,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<Candle>, JsValue> {
        self.get_historical_klines(symbol, interval, None, Some(start_time), Some(end_time)).await
    }
} 