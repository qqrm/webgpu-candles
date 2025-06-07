use crate::domain::market_data::{Candle, Symbol, TimeInterval};
use wasm_bindgen::JsValue;

/// Интерфейс для получения рыночных данных
pub trait MarketDataRepository {
    /// Получить исторические данные
    fn get_historical_candles(
        &self,
        symbol: &Symbol,
        interval: TimeInterval,
        limit: Option<usize>,
    ) -> Result<Vec<Candle>, JsValue>;

    /// Подписаться на real-time обновления
    fn subscribe_to_updates(
        &mut self,
        symbol: &Symbol,
        interval: TimeInterval,
        callback: Box<dyn Fn(Candle)>,
    ) -> Result<(), JsValue>;

    /// Отписаться от обновлений
    fn unsubscribe(&mut self) -> Result<(), JsValue>;
}

/// Интерфейс для хранения данных
pub trait CandleStorage {
    /// Сохранить свечи
    fn store_candles(&mut self, candles: Vec<Candle>) -> Result<(), JsValue>;
    
    /// Получить свечи
    fn get_candles(&self, symbol: &Symbol, interval: TimeInterval) -> Result<Vec<Candle>, JsValue>;
    
    /// Очистить старые данные
    fn cleanup_old_data(&mut self, keep_count: usize) -> Result<(), JsValue>;
} 