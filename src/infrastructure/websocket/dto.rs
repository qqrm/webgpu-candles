use serde::{Deserialize, Serialize};
use crate::domain::market_data::{Candle, Timestamp, OHLCV, Price, Volume};
use wasm_bindgen::JsValue;

/// DTO для данных Kline от Binance
#[derive(Debug, Deserialize)]
pub struct BinanceKlineData {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "k")]
    pub kline: BinanceKline,
}

#[derive(Debug, Deserialize)]
pub struct BinanceKline {
    #[serde(rename = "t")]
    pub start_time: u64,
    #[serde(rename = "T")]
    pub close_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "i")]
    pub interval: String,
    #[serde(rename = "f")]
    pub first_trade_id: u64,
    #[serde(rename = "L")]
    pub last_trade_id: u64,
    #[serde(rename = "o")]
    pub open_price: String,
    #[serde(rename = "c")]
    pub close_price: String,
    #[serde(rename = "h")]
    pub high_price: String,
    #[serde(rename = "l")]
    pub low_price: String,
    #[serde(rename = "v")]
    pub base_asset_volume: String,
    #[serde(rename = "n")]
    pub number_of_trades: u64,
    #[serde(rename = "x")]
    pub is_kline_closed: bool,
    #[serde(rename = "q")]
    pub quote_asset_volume: String,
    #[serde(rename = "V")]
    pub taker_buy_base_asset_volume: String,
    #[serde(rename = "Q")]
    pub taker_buy_quote_asset_volume: String,
}

impl BinanceKline {
    /// Конвертирует DTO в доменную сущность
    pub fn to_domain_candle(&self) -> Result<Candle, JsValue> {
        let timestamp = Timestamp::from(self.start_time);
        
        let open = self.open_price.parse::<f32>()
            .map_err(|e| JsValue::from_str(&format!("Parse open error: {}", e)))?;
        let high = self.high_price.parse::<f32>()
            .map_err(|e| JsValue::from_str(&format!("Parse high error: {}", e)))?;
        let low = self.low_price.parse::<f32>()
            .map_err(|e| JsValue::from_str(&format!("Parse low error: {}", e)))?;
        let close = self.close_price.parse::<f32>()
            .map_err(|e| JsValue::from_str(&format!("Parse close error: {}", e)))?;
        let volume = self.base_asset_volume.parse::<f32>()
            .map_err(|e| JsValue::from_str(&format!("Parse volume error: {}", e)))?;

        let ohlcv = OHLCV::new(
            Price::from(open as f64),
            Price::from(high as f64),
            Price::from(low as f64),
            Price::from(close as f64),
            Volume::from(volume as f64),
        );

        // Валидация данных
        if !ohlcv.is_valid() {
            return Err(JsValue::from_str("Invalid OHLCV data"));
        }

        Ok(Candle::new(timestamp, ohlcv))
    }
}

/// DTO для подписки на WebSocket
#[derive(Debug, Serialize)]
pub struct BinanceSubscription {
    pub method: String,
    pub params: Vec<String>,
    pub id: u64,
}

impl BinanceSubscription {
    pub fn kline_subscription(symbol: &str, interval: &str) -> Self {
        Self {
            method: "SUBSCRIBE".to_string(),
            params: vec![format!("{}@kline_{}", symbol.to_lowercase(), interval)],
            id: 1,
        }
    }

    pub fn unsubscribe(symbol: &str, interval: &str) -> Self {
        Self {
            method: "UNSUBSCRIBE".to_string(),
            params: vec![format!("{}@kline_{}", symbol.to_lowercase(), interval)],
            id: 2,
        }
    }
}

/// DTO для ответа на подписку
#[derive(Debug, Deserialize)]
pub struct BinanceSubscriptionResponse {
    pub result: Option<serde_json::Value>,
    pub id: u64,
}

/// DTO для ошибок WebSocket
#[derive(Debug, Deserialize)]
pub struct BinanceError {
    pub code: i32,
    pub msg: String,
}

/// DTO для 24hr ticker статистики
#[derive(Debug, Deserialize)]
pub struct BinanceTicker24hr {
    #[serde(rename = "e")]
    pub event_type: String,
    #[serde(rename = "E")]
    pub event_time: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price_change: String,
    #[serde(rename = "P")]
    pub price_change_percent: String,
    #[serde(rename = "w")]
    pub weighted_avg_price: String,
    #[serde(rename = "x")]
    pub first_trade_before_24hr: String,
    #[serde(rename = "c")]
    pub last_price: String,
    #[serde(rename = "Q")]
    pub last_quantity: String,
    #[serde(rename = "b")]
    pub best_bid_price: String,
    #[serde(rename = "B")]
    pub best_bid_quantity: String,
    #[serde(rename = "a")]
    pub best_ask_price: String,
    #[serde(rename = "A")]
    pub best_ask_quantity: String,
    #[serde(rename = "o")]
    pub open_price: String,
    #[serde(rename = "h")]
    pub high_price: String,
    #[serde(rename = "l")]
    pub low_price: String,
    #[serde(rename = "v")]
    pub total_traded_base_volume: String,
    #[serde(rename = "q")]
    pub total_traded_quote_volume: String,
    #[serde(rename = "O")]
    pub statistics_open_time: u64,
    #[serde(rename = "C")]
    pub statistics_close_time: u64,
    #[serde(rename = "F")]
    pub first_trade_id: u64,
    #[serde(rename = "L")]
    pub last_trade_id: u64,
    #[serde(rename = "n")]
    pub total_number_of_trades: u64,
} 