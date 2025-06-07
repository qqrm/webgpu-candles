use crate::{
    domain::{
        chart::Chart,
        market_data::{Candle, Symbol, TimeInterval},
        logging::{LogComponent, get_logger},
    },
    infrastructure::{
        websocket::BinanceWebSocketClient,
        http::BinanceHttpClient,
    },
};
use std::sync::{Arc, Mutex};
use wasm_bindgen_futures::spawn_local;

/// Use Case для единой обработки исторических и реал-тайм данных
pub struct UnifiedDataStreamUseCase<T> {
    http_client: BinanceHttpClient,
    websocket_client: T,
    chart: Arc<Mutex<Chart>>,
    is_streaming: bool,
}

impl<T> UnifiedDataStreamUseCase<T>
where
    T: WebSocketClient + Clone + 'static,
{
    pub fn new(
        http_client: BinanceHttpClient,
        websocket_client: T,
        chart: Arc<Mutex<Chart>>,
    ) -> Self {
        Self {
            http_client,
            websocket_client,
            chart,
            is_streaming: false,
        }
    }

    /// Инициализация с историческими данными и запуск реал-тайм стрима
    pub async fn initialize_and_stream(
        &mut self,
        symbol: &Symbol,
        interval: TimeInterval,
        historical_limit: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        get_logger().info(
            LogComponent::Application("UnifiedDataStream"),
            &format!("🔄 Initializing unified data stream for {} with {} historical candles", 
                symbol.value(), historical_limit)
        );

        // 1. Загружаем исторические данные
        self.load_historical_data(symbol, interval, historical_limit).await?;

        // 2. Запускаем реал-тайм стрим
        self.start_realtime_stream(symbol, interval).await?;

        get_logger().info(
            LogComponent::Application("UnifiedDataStream"),
            "✅ Unified data stream initialized successfully"
        );

        Ok(())
    }

    /// Загрузка исторических данных
    async fn load_historical_data(
        &self,
        symbol: &Symbol,
        interval: TimeInterval,
        limit: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        get_logger().info(
            LogComponent::Application("UnifiedDataStream"),
            &format!("📡 Loading {} historical candles for {}", limit, symbol.value())
        );

        let candles = self.http_client
            .get_recent_candles(symbol, interval, limit)
            .await?;

        // Устанавливаем исторические данные в единый контейнер
        {
            let mut chart = self.chart.lock().unwrap();
            chart.set_historical_data(candles.clone());
        }

        get_logger().info(
            LogComponent::Application("UnifiedDataStream"),
            &format!("✅ Loaded {} historical candles", candles.len())
        );

        Ok(())
    }

    /// Запуск реал-тайм стрима
    async fn start_realtime_stream(
        &mut self,
        symbol: &Symbol,
        interval: TimeInterval,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_streaming {
            return Ok(());
        }

        get_logger().info(
            LogComponent::Application("UnifiedDataStream"),
            &format!("🔴 Starting real-time stream for {}-{:?}", symbol.value(), interval)
        );

        let chart_clone = Arc::clone(&self.chart);
        let symbol_clone = symbol.clone();
        let mut ws_client = self.websocket_client.clone();

        // Запускаем WebSocket в отдельной задаче
        spawn_local(async move {
            let stream_id = format!("{}@kline_{}", symbol_clone.value().to_lowercase(), 
                Self::interval_to_binance_string(interval));

            if let Err(e) = ws_client.connect_klines(&stream_id).await {
                get_logger().error(
                    LogComponent::Application("UnifiedDataStream"),
                    &format!("❌ Failed to connect WebSocket: {:?}", e)
                );
                return;
            }

            // Обрабатываем входящие данные
            while let Ok(candle_data) = ws_client.receive_candle().await {
                if let Ok(candle) = Self::parse_websocket_candle(candle_data) {
                    // Добавляем новую свечу в единый контейнер
                    {
                        let mut chart = chart_clone.lock().unwrap();
                        chart.add_realtime_candle(candle);
                    }

                    get_logger().debug(
                        LogComponent::Application("UnifiedDataStream"),
                        "📊 Added real-time candle to unified container"
                    );
                }
            }
        });

        self.is_streaming = true;
        Ok(())
    }

    /// Получить количество свечей в контейнере
    pub fn get_candle_count(&self) -> usize {
        let chart = self.chart.lock().unwrap();
        chart.get_candle_count()
    }

    /// Проверить, есть ли данные
    pub fn has_data(&self) -> bool {
        let chart = self.chart.lock().unwrap();
        chart.has_data()
    }

    /// Остановить стрим
    pub async fn stop_stream(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.is_streaming {
            return Ok(());
        }

        get_logger().info(
            LogComponent::Application("UnifiedDataStream"),
            "🛑 Stopping real-time stream"
        );

        self.websocket_client.disconnect().await?;
        self.is_streaming = false;

        Ok(())
    }

    // Вспомогательные методы
    fn interval_to_binance_string(interval: TimeInterval) -> &'static str {
        match interval {
            TimeInterval::OneSecond => "1s", // Binance не поддерживает, но fallback
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

    fn parse_websocket_candle(_data: serde_json::Value) -> Result<Candle, Box<dyn std::error::Error>> {
        // Парсинг WebSocket данных Binance в Candle
        // TODO: Реализовать парсинг JSON из WebSocket
        todo!("Implement WebSocket candle parsing")
    }
}

/// Трейт для WebSocket клиента
#[allow(async_fn_in_trait)]
pub trait WebSocketClient {
    async fn connect_klines(&mut self, stream: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn receive_candle(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>>;
    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

impl WebSocketClient for BinanceWebSocketClient {
    async fn connect_klines(&mut self, stream: &str) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Реализовать подключение к WebSocket
        get_logger().info(
            LogComponent::Infrastructure("WebSocket"),
            &format!("🔗 Connecting to stream: {}", stream)
        );
        Ok(())
    }

    async fn receive_candle(&mut self) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        // TODO: Реализовать получение данных
        todo!("Implement WebSocket receive")
    }

    async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Реализовать отключение
        get_logger().info(
            LogComponent::Infrastructure("WebSocket"),
            "🔌 WebSocket disconnected"
        );
        Ok(())
    }
} 