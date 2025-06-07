use crate::{
    domain::{
        chart::Chart,
        market_data::{Symbol, TimeInterval},
        logging::{LogComponent, get_logger},
    },
    infrastructure::{
        websocket::BinanceWebSocketClient,
        http::BinanceHttpClient,
    },
    application::use_cases::UnifiedDataStreamUseCase,
};
use std::sync::{Arc, Mutex};

/// Сервис приложения для координации работы с графиками
pub struct ChartApplicationService {
    chart: Arc<Mutex<Chart>>,
    data_stream: Option<UnifiedDataStreamUseCase<BinanceWebSocketClient>>,
}

impl ChartApplicationService {
    pub fn new(chart_id: String) -> Self {
        let chart = Chart::new(
            chart_id,
            crate::domain::chart::value_objects::ChartType::Candlestick,
            1000, // Максимум 1000 свечей
        );

        Self {
            chart: Arc::new(Mutex::new(chart)),
            data_stream: None,
        }
    }

    /// Инициализация с историческими данными и реал-тайм стримом
    pub async fn initialize_with_unified_stream(
        &mut self,
        symbol: Symbol,
        interval: TimeInterval,
        historical_limit: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        get_logger().info(
            LogComponent::Application("ChartService"),
            &format!("🚀 Initializing chart with unified data stream for {}", symbol.value())
        );

        // Создаем клиенты
        let http_client = BinanceHttpClient::new();
        let ws_client = BinanceWebSocketClient::new(symbol.clone(), interval);

        // Создаем объединенный Use Case
        let mut unified_stream = UnifiedDataStreamUseCase::new(
            http_client,
            ws_client,
            Arc::clone(&self.chart),
        );

        // Инициализируем данные
        unified_stream
            .initialize_and_stream(&symbol, interval, historical_limit)
            .await?;

        self.data_stream = Some(unified_stream);

        get_logger().info(
            LogComponent::Application("ChartService"),
            "✅ Chart service initialized with unified data stream"
        );

        Ok(())
    }

    /// Получить доступ к графику для рендеринга
    pub fn get_chart(&self) -> Arc<Mutex<Chart>> {
        Arc::clone(&self.chart)
    }

    /// Получить статистику данных
    pub fn get_data_stats(&self) -> DataStats {
        if let Some(stream) = &self.data_stream {
            DataStats {
                total_candles: stream.get_candle_count(),
                has_data: stream.has_data(),
                is_streaming: true,
            }
        } else {
            DataStats {
                total_candles: 0,
                has_data: false,
                is_streaming: false,
            }
        }
    }

    /// Остановить стрим данных
    pub async fn stop_data_stream(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut stream) = self.data_stream.take() {
            stream.stop_stream().await?;
            get_logger().info(
                LogComponent::Application("ChartService"),
                "🛑 Data stream stopped"
            );
        }
        Ok(())
    }
}

/// Статистика данных
#[derive(Debug, Clone)]
pub struct DataStats {
    pub total_candles: usize,
    pub has_data: bool,
    pub is_streaming: bool,
} 