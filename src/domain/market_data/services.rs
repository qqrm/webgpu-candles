use crate::domain::market_data::{Candle, Price};

/// Структура для хранения данных скользящих средних
#[derive(Debug, Clone)]
pub struct MovingAveragesData {
    pub sma_20: Vec<Price>,
    pub sma_50: Vec<Price>, 
    pub sma_200: Vec<Price>,
    pub ema_12: Vec<Price>,
    pub ema_26: Vec<Price>,
}

/// Доменный сервис для анализа рыночных данных
pub struct MarketAnalysisService;

impl MarketAnalysisService {
    pub fn new() -> Self {
        Self
    }

    /// Валидация свечи - production-ready validation
    pub fn validate_candle(&self, candle: &Candle) -> bool {
        // 1. Базовая валидация OHLC логики
        let ohlc_valid = candle.ohlcv.high.value() >= candle.ohlcv.low.value() &&
                        candle.ohlcv.high.value() >= candle.ohlcv.open.value() &&
                        candle.ohlcv.high.value() >= candle.ohlcv.close.value() &&
                        candle.ohlcv.low.value() <= candle.ohlcv.open.value() &&
                        candle.ohlcv.low.value() <= candle.ohlcv.close.value();

        // 2. Валидация положительных значений
        let positive_values = candle.ohlcv.open.value() > 0.0 &&
                             candle.ohlcv.high.value() > 0.0 &&
                             candle.ohlcv.low.value() > 0.0 &&
                             candle.ohlcv.close.value() > 0.0 &&
                             candle.ohlcv.volume.value() >= 0.0;

        // 3. Валидация разумных пределов для BTC/USDT
        let reasonable_price_range = candle.ohlcv.low.value() > 1.0 && // Минимум $1
                                    candle.ohlcv.high.value() < 1_000_000.0; // Максимум $1M

        // 4. Валидация timestamp (не может быть в будущем более чем на 1 минуту)
        let now = js_sys::Date::now() as u64;
        let timestamp_valid = candle.timestamp.value() <= now + 60_000; // +1 минута буфер

        ohlc_valid && positive_values && reasonable_price_range && timestamp_valid
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

    /// Вычисляет экспоненциальную скользящую среднюю (EMA)
    pub fn calculate_ema(&self, candles: &[Candle], period: usize) -> Vec<Price> {
        if candles.len() < period {
            return Vec::new();
        }

        let mut ema_values = Vec::new();
        let alpha = 2.0 / (period as f32 + 1.0); // Сглаживающий коэффициент
        
        // Первое значение EMA = простое среднее за первые period свечей
        let first_sma: f32 = candles[0..period]
            .iter()
            .map(|candle| candle.ohlcv.close.value())
            .sum::<f32>() / period as f32;
        
        ema_values.push(Price::from(first_sma));
        
        // Вычисляем остальные значения EMA
        for i in period..candles.len() {
            let current_price = candles[i].ohlcv.close.value();
            let prev_ema = ema_values.last().unwrap().value();
            let new_ema = alpha * current_price + (1.0 - alpha) * prev_ema;
            
            ema_values.push(Price::from(new_ema));
        }

        ema_values
    }

    /// Вычисляет несколько скользящих средних одновременно
    pub fn calculate_multiple_mas(&self, candles: &[Candle]) -> MovingAveragesData {
        MovingAveragesData {
            sma_20: self.calculate_sma(candles, 20),
            sma_50: self.calculate_sma(candles, 50),
            sma_200: self.calculate_sma(candles, 200),
            ema_12: self.calculate_ema(candles, 12),
            ema_26: self.calculate_ema(candles, 26),
        }
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

    /// Валидация свечи с подробным описанием ошибки
    pub fn validate_candle(&self, candle: &Candle) -> Result<(), String> {
        // 1. Базовая валидация OHLC логики
        if candle.ohlcv.high.value() < candle.ohlcv.low.value() {
            return Err("High price cannot be lower than low price".to_string());
        }
        
        if candle.ohlcv.high.value() < candle.ohlcv.open.value() {
            return Err("High price cannot be lower than open price".to_string());
        }
        
        if candle.ohlcv.high.value() < candle.ohlcv.close.value() {
            return Err("High price cannot be lower than close price".to_string());
        }
        
        if candle.ohlcv.low.value() > candle.ohlcv.open.value() {
            return Err("Low price cannot be higher than open price".to_string());
        }
        
        if candle.ohlcv.low.value() > candle.ohlcv.close.value() {
            return Err("Low price cannot be higher than close price".to_string());
        }

        // 2. Валидация положительных значений
        if candle.ohlcv.open.value() <= 0.0 {
            return Err("Open price must be positive".to_string());
        }
        if candle.ohlcv.high.value() <= 0.0 {
            return Err("High price must be positive".to_string());
        }
        if candle.ohlcv.low.value() <= 0.0 {
            return Err("Low price must be positive".to_string());
        }
        if candle.ohlcv.close.value() <= 0.0 {
            return Err("Close price must be positive".to_string());
        }
        if candle.ohlcv.volume.value() < 0.0 {
            return Err("Volume cannot be negative".to_string());
        }

        // 3. Валидация разумных пределов для BTC/USDT
        if candle.ohlcv.low.value() < 1.0 {
            return Err("Price is unreasonably low (< $1)".to_string());
        }
        if candle.ohlcv.high.value() > 1_000_000.0 {
            return Err("Price is unreasonably high (> $1M)".to_string());
        }

        // 4. Валидация timestamp (не может быть в будущем более чем на 1 минуту)
        let now = js_sys::Date::now() as u64;
        if candle.timestamp.value() > now + 60_000 {
            return Err("Timestamp is too far in the future".to_string());
        }

        Ok(())
    }

    /// Валидация последовательности свечей
    pub fn validate_candle_sequence(&self, candles: &[Candle]) -> Result<(), String> {
        if candles.is_empty() {
            return Ok(());
        }

        for i in 1..candles.len() {
            let prev = &candles[i - 1];
            let current = &candles[i];

            // Проверяем хронологический порядок
            if current.timestamp.value() <= prev.timestamp.value() {
                return Err(format!(
                    "Candles are not in chronological order at index {}",
                    i
                ));
            }

            // Проверяем разумные промежутки времени (не более 1 часа между свечами)
            let time_diff = current.timestamp.value() - prev.timestamp.value();
            if time_diff > 3_600_000 {
                // 1 час в миллисекундах
                return Err(format!(
                    "Time gap too large between candles at index {}: {} ms",
                    i, time_diff
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