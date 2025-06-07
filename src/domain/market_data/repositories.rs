use crate::domain::market_data::{Candle, Symbol, TimeInterval};

/// Domain error types for repository operations
#[derive(Debug, Clone)]
pub enum RepositoryError {
    NetworkError(String),
    ParseError(String),
    ValidationError(String),
    ConnectionError(String),
}

/// Интерфейс для получения рыночных данных
pub trait MarketDataRepository {
    /// Получить исторические данные
    fn get_historical_candles(
        &self,
        symbol: &Symbol,
        interval: TimeInterval,
        limit: Option<usize>,
    ) -> Result<Vec<Candle>, RepositoryError>;

    /// Подписаться на real-time обновления
    fn subscribe_to_updates(
        &mut self,
        symbol: &Symbol,
        interval: TimeInterval,
        callback: Box<dyn Fn(Candle)>,
    ) -> Result<(), RepositoryError>;

    /// Отписаться от обновлений
    fn unsubscribe(&mut self) -> Result<(), RepositoryError>;
}

/// Интерфейс для хранения данных
pub trait CandleStorage {
    /// Сохранить свечи
    fn store_candles(&mut self, candles: Vec<Candle>) -> Result<(), RepositoryError>;
    
    /// Получить свечи
    fn get_candles(&self, symbol: &Symbol, interval: TimeInterval) -> Result<Vec<Candle>, RepositoryError>;
    
    /// Очистить старые данные
    fn cleanup_old_data(&mut self, keep_count: usize) -> Result<(), RepositoryError>;
} 