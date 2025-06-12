use crate::domain::{
    logging::{LogComponent, get_logger},
    market_data::{
        entities::{Candle, OHLCV},
        value_objects::{Price, Symbol, TimeInterval, Timestamp, Volume},
    },
};
use futures::StreamExt;
use gloo_net::http::Request;
use gloo_net::websocket::futures::WebSocket;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

/// Binance WebSocket client based on gloo
pub struct BinanceWebSocketClient {
    symbol: Symbol,
    interval: TimeInterval,
}

#[derive(Debug, Deserialize)]
struct BinanceKlineData {
    #[serde(rename = "k")]
    kline: KlineInfo,
}

#[derive(Debug, Deserialize)]
struct KlineInfo {
    #[serde(rename = "t")]
    open_time: u64,
    #[serde(rename = "o")]
    open: String,
    #[serde(rename = "h")]
    high: String,
    #[serde(rename = "l")]
    low: String,
    #[serde(rename = "c")]
    close: String,
    #[serde(rename = "v")]
    volume: String,
}

/// Structure for historical Binance Klines API data
#[derive(Debug, Deserialize)]
struct BinanceHistoricalKline(
    u64,                   // Open time
    String,                // Open
    String,                // High
    String,                // Low
    String,                // Close
    String,                // Volume
    serde::de::IgnoredAny, // Close time
    serde::de::IgnoredAny, // Quote asset volume
    serde::de::IgnoredAny, // Number of trades
    serde::de::IgnoredAny, // Taker buy base asset volume
    serde::de::IgnoredAny, // Taker buy quote asset volume
    serde::de::IgnoredAny, // Ignore
);

impl BinanceWebSocketClient {
    pub fn new(symbol: Symbol, interval: TimeInterval) -> Self {
        Self { symbol, interval }
    }

    /// Connect to the Binance WebSocket stream
    pub async fn connect(&mut self) -> Result<WebSocket, String> {
        let symbol_lower = self.symbol.value().to_lowercase();
        let interval_str = self.interval.to_binance_str();

        let stream_name = format!("{symbol_lower}@kline_{interval_str}");
        let url = format!("wss://stream.binance.com:9443/ws/{stream_name}");

        get_logger().info(
            LogComponent::Infrastructure("BinanceWS"),
            &format!("🔌 Connecting to Binance: {url}"),
        );

        let ws = WebSocket::open(&url).map_err(|e| format!("Failed to open WebSocket: {e:?}"))?;

        get_logger().info(
            LogComponent::Infrastructure("BinanceWS"),
            &format!("✅ Connected to Binance stream: {stream_name}"),
        );

        Ok(ws)
    }

    /// Handle a message from Binance
    pub fn parse_message(&self, data: &str) -> Result<Candle, String> {
        let kline_data: BinanceKlineData = serde_json::from_str(data)
            .map_err(|e| format!("Failed to parse Binance message: {e}"))?;

        let kline = &kline_data.kline;

        // Parse prices
        let open = kline.open.parse::<f64>().map_err(|_| "Invalid open price")?;
        let high = kline.high.parse::<f64>().map_err(|_| "Invalid high price")?;
        let low = kline.low.parse::<f64>().map_err(|_| "Invalid low price")?;
        let close = kline.close.parse::<f64>().map_err(|_| "Invalid close price")?;
        let volume = kline.volume.parse::<f64>().map_err(|_| "Invalid volume")?;

        // Create OHLCV
        let ohlcv = OHLCV::new(
            Price::new(open),
            Price::new(high),
            Price::new(low),
            Price::new(close),
            Volume::new(volume),
        );

        // Create a candle
        let candle = Candle::new(Timestamp::new(kline.open_time), ohlcv);

        Ok(candle)
    }

    /// Start the stream with a handler
    pub async fn start_stream<F>(&mut self, handler: F) -> Result<(), String>
    where
        F: FnMut(Candle) + 'static,
    {
        self.run_stream(handler, || {}).await
    }

    pub async fn start_stream_with_callback<F, R>(
        &mut self,
        handler: F,
        on_reconnect: R,
    ) -> Result<(), String>
    where
        F: FnMut(Candle) + 'static,
        R: FnMut(),
    {
        self.run_stream(handler, on_reconnect).await
    }

    async fn run_stream<F, R>(&mut self, mut handler: F, mut on_reconnect: R) -> Result<(), String>
    where
        F: FnMut(Candle) + 'static,
        R: FnMut(),
    {
        use gloo_timers::future::sleep;
        use std::time::Duration;

        let mut delay = 1u64;
        loop {
            let mut stream = match self.connect().await {
                Ok(ws) => {
                    get_logger().info(
                        LogComponent::Infrastructure("BinanceWS"),
                        "🚀 Starting Binance WebSocket stream processing...",
                    );
                    delay = 1;
                    ws
                }
                Err(e) => {
                    get_logger().error(
                        LogComponent::Infrastructure("BinanceWS"),
                        &format!("❌ Connection error: {e}"),
                    );
                    on_reconnect();
                    sleep(Duration::from_secs(delay)).await;
                    delay = (delay * 2).min(32);
                    continue;
                }
            };

            while let Some(msg) = stream.next().await {
                match msg {
                    Ok(gloo_net::websocket::Message::Text(data)) => match self.parse_message(&data)
                    {
                        Ok(candle) => {
                            get_logger().debug(
                                    LogComponent::Infrastructure("BinanceWS"),
                                    &format!(
                                        "📊 Received candle: {} - O:{:.2} H:{:.2} L:{:.2} C:{:.2} V:{:.2}",
                                        self.symbol.value(),
                                        candle.ohlcv.open.value(),
                                        candle.ohlcv.high.value(),
                                        candle.ohlcv.low.value(),
                                        candle.ohlcv.close.value(),
                                        candle.ohlcv.volume.value()
                                    ),
                                );
                            handler(candle);
                        }
                        Err(e) => {
                            get_logger().error(
                                LogComponent::Infrastructure("BinanceWS"),
                                &format!("❌ Failed to parse message: {e}"),
                            );
                        }
                    },
                    Ok(_) => {
                        // Ignore binary messages
                    }
                    Err(e) => {
                        get_logger().error(
                            LogComponent::Infrastructure("BinanceWS"),
                            &format!("❌ WebSocket error: {e:?}"),
                        );
                        break;
                    }
                }
            }

            get_logger().warn(
                LogComponent::Infrastructure("BinanceWS"),
                &format!("🔌 Reconnecting in {delay}s"),
            );
            on_reconnect();
            sleep(Duration::from_secs(delay)).await;
            delay = (delay * 2).min(32);
        }
    }

    /// 📈 Load historical data from Binance REST API
    pub async fn fetch_historical_data(&self, limit: u32) -> Result<Vec<Candle>, String> {
        let symbol_upper = self.symbol.value().to_uppercase();
        let interval_str = self.interval.to_binance_str();

        let url = format!(
            "https://api.binance.com/api/v3/klines?symbol={symbol_upper}&interval={interval_str}&limit={limit}"
        );

        get_logger().info(
            LogComponent::Infrastructure("BinanceAPI"),
            &format!("📈 Fetching {limit} historical candles from: {url}"),
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

            let candle = Candle::new(
                Timestamp::new(kline.0), // open_time
                ohlcv,
            );

            candles.push(candle);
        }

        get_logger().info(
            LogComponent::Infrastructure("BinanceAPI"),
            &format!("✅ Loaded {} historical candles for {}", candles.len(), symbol_upper),
        );

        Ok(candles)
    }

    /// 📈 Load historical data up to the specified time
    pub async fn fetch_historical_data_before(
        &self,
        end_time: u64,
        limit: u32,
    ) -> Result<Vec<Candle>, String> {
        let symbol_upper = self.symbol.value().to_uppercase();
        let interval_str = self.interval.to_binance_str();

        let url = format!(
            "https://api.binance.com/api/v3/klines?symbol={symbol_upper}&interval={interval_str}&endTime={end_time}&limit={limit}"
        );

        get_logger().info(
            LogComponent::Infrastructure("BinanceAPI"),
            &format!("📈 Fetching {limit} candles before {end_time} from: {url}"),
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

/// Simple helper to create a WebSocket connection
pub async fn create_binance_stream(
    symbol: &str,
    interval: &str,
) -> Result<BinanceWebSocketClient, String> {
    let symbol = Symbol::from(symbol);
    let interval =
        interval.parse::<TimeInterval>().map_err(|_| format!("Invalid interval: {interval}"))?;

    let client = BinanceWebSocketClient::new(symbol, interval);
    Ok(client)
}

/// Exported function for JavaScript
#[wasm_bindgen]
pub async fn test_binance_websocket() -> Result<(), JsValue> {
    get_logger().info(
        LogComponent::Infrastructure("BinanceWS"),
        "🧪 Testing Binance WebSocket with gloo...",
    );

    let mut client =
        create_binance_stream("BTCUSDT", "1m").await.map_err(|e| JsValue::from_str(&e))?;

    // Test handler
    let handler = |candle: Candle| {
        get_logger().info(
            LogComponent::Infrastructure("BinanceWS"),
            &format!("✅ Test received: ${:.2}", candle.ohlcv.close.value()),
        );
    };

    // Run for 10 seconds for testing
    if let Err(e) = client.start_stream(handler).await {
        get_logger()
            .error(LogComponent::Infrastructure("BinanceWS"), &format!("❌ Stream error: {e}"));
        return Err(JsValue::from_str(&e));
    }

    get_logger()
        .info(LogComponent::Infrastructure("BinanceWS"), "✅ Binance WebSocket test completed");
    Ok(())
}
