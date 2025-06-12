pub use super::value_objects::{OHLCV, Price, Timestamp, Volume};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Domain entity - Candle
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

/// Domain entity - Candle series
#[derive(Debug, Clone)]
pub struct CandleSeries {
    candles: VecDeque<Candle>,
    max_size: usize,
}

impl CandleSeries {
    pub fn new(max_size: usize) -> Self {
        Self { candles: VecDeque::new(), max_size }
    }

    pub fn add_candle(&mut self, candle: Candle) {
        // Check whether to update the existing candle or add a new one
        if let Some(last_candle) = self.candles.back_mut() {
            if last_candle.timestamp == candle.timestamp {
                *last_candle = candle;
                return;
            }

            // Ensure chronological order
            if candle.timestamp.value() < last_candle.timestamp.value() {
                // If the new candle is older than the last, insert it sorted
                self.insert_candle_sorted(candle);
                return;
            }
        }

        self.candles.push_back(candle);

        // Limit size for performance
        if self.candles.len() > self.max_size {
            self.candles.pop_front();
        }
    }

    /// Insert a candle while keeping time order
    fn insert_candle_sorted(&mut self, candle: Candle) {
        // Find the correct insertion position
        let insert_pos = self
            .candles
            .iter()
            .position(|c| c.timestamp.value() >= candle.timestamp.value())
            .unwrap_or(self.candles.len());

        // Replace the candle if one with the same timestamp exists
        if insert_pos < self.candles.len() && self.candles[insert_pos].timestamp == candle.timestamp
        {
            self.candles[insert_pos] = candle;
        } else {
            self.candles.insert(insert_pos, candle);
        }

        // Limit the size
        if self.candles.len() > self.max_size {
            self.candles.pop_front();
        }
    }

    pub fn get_candles(&self) -> &VecDeque<Candle> {
        &self.candles
    }

    pub fn latest(&self) -> Option<&Candle> {
        self.candles.back()
    }

    pub fn latest_mut(&mut self) -> Option<&mut Candle> {
        self.candles.back_mut()
    }

    pub fn count(&self) -> usize {
        self.candles.len()
    }

    /// Maximum number of candles in the series
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Capacity of the series (maximum candle count)
    pub fn capacity(&self) -> usize {
        self.max_size
    }

    /// Get the last closing price
    pub fn get_latest_price(&self) -> Option<&Price> {
        self.candles.back().map(|candle| &candle.ohlcv.close)
    }

    /// Get the price range of all candles
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
