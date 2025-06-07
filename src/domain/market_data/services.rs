use crate::domain::market_data::{Candle, Price, TimeInterval};

/// Доменный сервис для анализа рыночных данных
pub struct MarketAnalysisService;

impl MarketAnalysisService {
    pub fn new() -> Self {
        Self
    }

    /// Вычисляет простую скользящую среднюю (SMA)
    pub fn calculate_sma(&self, candles: &[Candle], period: usize) -> Vec<Price> {
        if candles.len() < period {
            return Vec::new();
        }

        let mut sma_values = Vec::new();
        
        for i in (period - 1)..candles.len() {
            let sum: f32 = candles[i - period + 1..=i]
                .iter()
                .map(|candle| candle.ohlcv.close.value())
                .sum();
            
            sma_values.push(Price::from(sum / period as f32));
        }

        sma_values
    }

    /// Находит локальные максимумы и минимумы
    pub fn find_extremes(&self, candles: &[Candle], window: usize) -> (Vec<usize>, Vec<usize>) {
        if candles.len() < window * 2 + 1 {
            return (Vec::new(), Vec::new());
        }

        let mut peaks = Vec::new();
        let mut troughs = Vec::new();

        for i in window..(candles.len() - window) {
            let current_high = candles[i].ohlcv.high;
            let current_low = candles[i].ohlcv.low;

            // Проверяем максимум
            let is_peak = candles[i - window..i]
                .iter()
                .chain(candles[i + 1..=i + window].iter())
                .all(|c| c.ohlcv.high < current_high);

            if is_peak {
                peaks.push(i);
            }

            // Проверяем минимум
            let is_trough = candles[i - window..i]
                .iter()
                .chain(candles[i + 1..=i + window].iter())
                .all(|c| c.ohlcv.low > current_low);

            if is_trough {
                troughs.push(i);
            }
        }

        (peaks, troughs)
    }

    /// Вычисляет волатильность (стандартное отклонение доходности)
    pub fn calculate_volatility(&self, candles: &[Candle], period: usize) -> Option<f32> {
        if candles.len() < period + 1 {
            return None;
        }

        // Вычисляем доходности
        let returns: Vec<f32> = candles
            .windows(2)
            .map(|pair| {
                let prev_close = pair[0].ohlcv.close.value();
                let curr_close = pair[1].ohlcv.close.value();
                (curr_close - prev_close) / prev_close
            })
            .collect();

        if returns.len() < period {
            return None;
        }

        // Берем последние period доходностей
        let recent_returns = &returns[returns.len() - period..];
        
        // Вычисляем среднюю доходность
        let mean_return: f32 = recent_returns.iter().sum::<f32>() / period as f32;
        
        // Вычисляем дисперсию
        let variance: f32 = recent_returns
            .iter()
            .map(|r| (r - mean_return).powi(2))
            .sum::<f32>() / period as f32;

        Some(variance.sqrt())
    }
}

/// Доменный сервис для валидации данных
#[derive(Clone)]
pub struct DataValidationService;

impl DataValidationService {
    pub fn new() -> Self {
        Self
    }

    /// Проверяет валидность свечи
    pub fn validate_candle(&self, candle: &Candle) -> Result<(), String> {
        if !candle.ohlcv.is_valid() {
            return Err("Invalid OHLCV data".to_string());
        }

        if candle.timestamp.value() == 0 {
            return Err("Invalid timestamp".to_string());
        }

        Ok(())
    }

    /// Проверяет последовательность свечей
    pub fn validate_candle_sequence(&self, candles: &[Candle], interval: TimeInterval) -> Result<(), String> {
        if candles.len() < 2 {
            return Ok(());
        }

        for pair in candles.windows(2) {
            let curr = &pair[0];
            let next = &pair[1];

            // Проверяем временную последовательность
            if next.timestamp.value() <= curr.timestamp.value() {
                return Err("Candles are not in chronological order".to_string());
            }

            // Проверяем интервал (с небольшой погрешностью)
            let expected_interval = interval.duration_ms();
            let actual_interval = next.timestamp.value() - curr.timestamp.value();
            
            if actual_interval < expected_interval / 2 || actual_interval > expected_interval * 2 {
                return Err(format!(
                    "Invalid time interval between candles: expected ~{}, got {}",
                    expected_interval, actual_interval
                ));
            }
        }

        Ok(())
    }

    /// Находит аномальные свечи (с экстремальными значениями)
    pub fn find_anomalies(&self, candles: &[Candle], threshold: f32) -> Vec<usize> {
        if candles.is_empty() {
            return Vec::new();
        }

        let mut anomalies = Vec::new();
        
        // Вычисляем медианный объем
        let mut volumes: Vec<f32> = candles.iter().map(|c| c.ohlcv.volume.value()).collect();
        volumes.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_volume = volumes[volumes.len() / 2];

        for (i, candle) in candles.iter().enumerate() {
            // Проверяем аномальный объем
            if candle.ohlcv.volume.value() > median_volume * threshold {
                anomalies.push(i);
                continue;
            }

            // Проверяем аномальную волатильность (размах цены)
            let price_range = candle.ohlcv.high.value() - candle.ohlcv.low.value();
            let body_size = (candle.ohlcv.close.value() - candle.ohlcv.open.value()).abs();
            
            if price_range > 0.0 && body_size / price_range > threshold {
                anomalies.push(i);
            }
        }

        anomalies
    }
} 