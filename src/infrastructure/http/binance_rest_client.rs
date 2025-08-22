use crate::domain::logging::{LogComponent, get_logger};
use crate::domain::market_data::{
    Candle, TimeInterval,
    value_objects::{OHLCV, Price, Symbol, Timestamp, Volume},
};
use gloo_net::http::Request;

#[derive(Debug, serde::Deserialize)]
struct BinanceHistoricalKline(
    u64,
    String,
    String,
    String,
    String,
    String,
    serde::de::IgnoredAny,
    serde::de::IgnoredAny,
    serde::de::IgnoredAny,
    serde::de::IgnoredAny,
    serde::de::IgnoredAny,
    serde::de::IgnoredAny,
);

/// Simple REST client for Binance API
pub struct BinanceRestClient {
    symbol: Symbol,
    interval: TimeInterval,
}

impl BinanceRestClient {
    pub fn new(symbol: Symbol, interval: TimeInterval) -> Self {
        Self { symbol, interval }
    }

    fn base_url(&self) -> String {
        "https://api.binance.com/api/v3".to_string()
    }

    pub fn ui_klines_url_before(&self, end_time: u64, limit: u32) -> String {
        format!(
            "{}/uiKlines?symbol={}&interval={}&endTime={}&limit={}",
            self.base_url(),
            self.symbol.value().to_uppercase(),
            self.interval.to_binance_str(),
            end_time,
            limit
        )
    }

    pub fn klines_url_before(&self, end_time: u64, limit: u32) -> String {
        format!(
            "{}/klines?symbol={}&interval={}&endTime={}&limit={}",
            self.base_url(),
            self.symbol.value().to_uppercase(),
            self.interval.to_binance_str(),
            end_time,
            limit
        )
    }

    /// Fetch candles before the specified time, using uiKlines then falling back to klines
    pub async fn fetch_historical_before(
        &self,
        end_time: u64,
        limit: u32,
    ) -> Result<Vec<Candle>, String> {
        match self.fetch_from_url(self.ui_klines_url_before(end_time, limit)).await {
            Ok(c) if !c.is_empty() => Ok(c),
            _ => self.fetch_from_url(self.klines_url_before(end_time, limit)).await,
        }
    }

    async fn fetch_from_url(&self, url: String) -> Result<Vec<Candle>, String> {
        get_logger().info(
            LogComponent::Infrastructure("BinanceAPI"),
            &format!("📈 Fetching candles from: {url}"),
        );

        let response = Request::get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch historical data: {e:?}"))?;

        if !response.ok() {
            return Err(format!("HTTP error: {}", response.status()));
        }

        let klines: Vec<BinanceHistoricalKline> =
            response.json().await.map_err(|e| format!("Failed to parse JSON: {e:?}"))?;

        let mut candles = Vec::new();
        for kline in klines {
            let open = kline.1.parse::<f64>().map_err(|_| "Invalid open price")?;
            let high = kline.2.parse::<f64>().map_err(|_| "Invalid high price")?;
            let low = kline.3.parse::<f64>().map_err(|_| "Invalid low price")?;
            let close = kline.4.parse::<f64>().map_err(|_| "Invalid close price")?;
            let volume = kline.5.parse::<f64>().map_err(|_| "Invalid volume")?;

            let ohlcv = OHLCV::new(
                Price::new(open),
                Price::new(high),
                Price::new(low),
                Price::new(close),
                Volume::new(volume),
            );

            let candle = Candle::new(Timestamp::new(kline.0), ohlcv);
            candles.push(candle);
        }

        get_logger().info(
            LogComponent::Infrastructure("BinanceAPI"),
            &format!("✅ Loaded {} historical candles", candles.len()),
        );

        Ok(candles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::market_data::TimeInterval;

    #[test]
    fn test_ui_klines_url_before() {
        let client = BinanceRestClient::new(Symbol::from("BTCUSDT"), TimeInterval::OneMinute);
        let url = client.ui_klines_url_before(12345, 1000);
        assert_eq!(
            url,
            "https://api.binance.com/api/v3/uiKlines?symbol=BTCUSDT&interval=1m&endTime=12345&limit=1000"
        );
    }
}
