pub use super::value_objects::{Timestamp, OHLCV, Price, Volume};
use serde::{Deserialize, Serialize};

/// Доменная сущность - Свеча
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Candle {
    pub timestamp: Timestamp,
    pub ohlcv: OHLCV,
}

impl Candle {
    pub fn new(timestamp: Timestamp, ohlcv: OHLCV) -> Self {
        Self { timestamp, ohlcv }
    }

    pub fn is_bullish(&self) -> bool {
        self.ohlcv.close > self.ohlcv.open
    }

    pub fn is_bearish(&self) -> bool {
        self.ohlcv.close < self.ohlcv.open
    }

    pub fn body_size(&self) -> Price {
        Price::from((self.ohlcv.close.value() - self.ohlcv.open.value()).abs())
    }

    pub fn wick_high(&self) -> Price {
        Price::from(self.ohlcv.high.value() - self.ohlcv.close.value().max(self.ohlcv.open.value()))
    }

    pub fn wick_low(&self) -> Price {
        Price::from(self.ohlcv.close.value().min(self.ohlcv.open.value()) - self.ohlcv.low.value())
    }
}

/// Доменная сущность - Временной ряд свечей
#[derive(Debug, Clone)]
pub struct CandleSeries {
    candles: Vec<Candle>,
    max_size: usize,
}

impl CandleSeries {
    pub fn new(max_size: usize) -> Self {
        Self {
            candles: Vec::new(),
            max_size,
        }
    }

    pub fn add_candle(&mut self, candle: Candle) {
        // Проверяем, обновляем ли мы существующую свечу или добавляем новую
        if let Some(last_candle) = self.candles.last_mut() {
            if last_candle.timestamp == candle.timestamp {
                *last_candle = candle;
                return;
            }
            
            // Проверяем хронологический порядок
            if candle.timestamp.value() < last_candle.timestamp.value() {
                // Если новая свеча старше последней, нужна вставка с сортировкой
                self.insert_candle_sorted(candle);
                return;
            }
        }

        self.candles.push(candle);
        
        // Ограничиваем размер для производительности
        if self.candles.len() > self.max_size {
            self.candles.remove(0);
        }
    }

    /// Вставка свечи с сохранением сортировки по времени
    fn insert_candle_sorted(&mut self, candle: Candle) {
        // Находим правильную позицию для вставки
        let insert_pos = self.candles
            .binary_search_by(|c| c.timestamp.value().cmp(&candle.timestamp.value()))
            .unwrap_or_else(|pos| pos);
        
        // Если свеча с таким timestamp уже существует, заменяем её
        if insert_pos < self.candles.len() && 
           self.candles[insert_pos].timestamp == candle.timestamp {
            self.candles[insert_pos] = candle;
        } else {
            self.candles.insert(insert_pos, candle);
        }
        
        // Ограничиваем размер
        if self.candles.len() > self.max_size {
            self.candles.remove(0);
        }
    }

    pub fn get_candles(&self) -> &[Candle] {
        &self.candles
    }

    pub fn latest(&self) -> Option<&Candle> {
        self.candles.last()
    }

    pub fn count(&self) -> usize {
        self.candles.len()
    }

    /// Получить последнюю цену закрытия
    pub fn get_latest_price(&self) -> Option<&Price> {
        self.candles.last().map(|candle| &candle.ohlcv.close)
    }

    /// Получить ценовой диапазон всех свечей
    pub fn price_range(&self) -> Option<(&Price, &Price)> {
        if self.candles.is_empty() {
            return None;
        }

        let mut min_price = &self.candles[0].ohlcv.low;
        let mut max_price = &self.candles[0].ohlcv.high;

        for candle in &self.candles {
            if candle.ohlcv.low.value() < min_price.value() {
                min_price = &candle.ohlcv.low;
            }
            if candle.ohlcv.high.value() > max_price.value() {
                max_price = &candle.ohlcv.high;
            }
        }

        Some((min_price, max_price))
    }
}

