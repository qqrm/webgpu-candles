use crate::domain::market_data::{
    Candle, OHLCV, Price, TimeInterval, Timestamp, Volume, indicator_engine::MovingAveragesData,
};

/// Ichimoku indicator components
#[derive(Debug, Clone, Default)]
pub struct IchimokuData {
    pub tenkan_sen: Vec<Price>,
    pub kijun_sen: Vec<Price>,
    pub senkou_span_a: Vec<Price>,
    pub senkou_span_b: Vec<Price>,
    pub chikou_span: Vec<Price>,
}

/// Domain service for market analysis
pub struct MarketAnalysisService;

impl Default for MarketAnalysisService {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketAnalysisService {
    pub fn new() -> Self {
        Self
    }

    /// Candle validation - production-ready
    pub fn validate_candle(&self, candle: &Candle) -> bool {
        // 1. Basic OHLC validation
        let ohlc_valid = candle.ohlcv.high.value() >= candle.ohlcv.low.value()
            && candle.ohlcv.high.value() >= candle.ohlcv.open.value()
            && candle.ohlcv.high.value() >= candle.ohlcv.close.value()
            && candle.ohlcv.low.value() <= candle.ohlcv.open.value()
            && candle.ohlcv.low.value() <= candle.ohlcv.close.value();

        // 2. Validate positive values
        let positive_values = candle.ohlcv.open.value() > 0.0
            && candle.ohlcv.high.value() > 0.0
            && candle.ohlcv.low.value() > 0.0
            && candle.ohlcv.close.value() > 0.0
            && candle.ohlcv.volume.value() >= 0.0;

        // 3. Validate reasonable bounds for BTC/USDT
        let reasonable_price_range = candle.ohlcv.low.value() > 1.0 && // Minimum $1
                                    candle.ohlcv.high.value() < 1_000_000.0; // Maximum $1M

        // 4. Validate timestamp (not more than 1 minute in the future)
        let now = js_sys::Date::now() as u64;
        let timestamp_valid = candle.timestamp.value() <= now + 60_000; // +1 minute buffer

        ohlc_valid && positive_values && reasonable_price_range && timestamp_valid
    }

    /// Calculate the Simple Moving Average (SMA)
    pub fn calculate_sma(&self, candles: &[Candle], period: usize) -> Vec<Price> {
        if candles.len() < period {
            return Vec::new();
        }

        let mut sma_values = Vec::new();

        for i in (period - 1)..candles.len() {
            let sum: f64 =
                candles[i - period + 1..=i].iter().map(|candle| candle.ohlcv.close.value()).sum();

            sma_values.push(Price::from(sum / period as f64));
        }

        sma_values
    }

    /// Calculate the Exponential Moving Average (EMA)
    pub fn calculate_ema(&self, candles: &[Candle], period: usize) -> Vec<Price> {
        if candles.len() < period {
            return Vec::new();
        }

        let mut ema_values = Vec::new();
        let alpha = 2.0 / (period as f64 + 1.0); // Smoothing factor

        // First EMA value is the simple average over the first period candles
        let first_sma: f64 =
            candles[0..period].iter().map(|candle| candle.ohlcv.close.value()).sum::<f64>()
                / period as f64;

        ema_values.push(Price::from(first_sma));

        // Compute the remaining EMA values
        for candle in candles.iter().skip(period) {
            let current_price = candle.ohlcv.close.value();
            let prev_ema = ema_values.last().unwrap().value();
            let new_ema = alpha * current_price + (1.0 - alpha) * prev_ema;

            ema_values.push(Price::from(new_ema));
        }

        ema_values
    }

    /// Calculate multiple moving averages at once
    pub fn calculate_multiple_mas(&self, candles: &[Candle]) -> MovingAveragesData {
        MovingAveragesData {
            sma_20: self.calculate_sma(candles, 20),
            sma_50: self.calculate_sma(candles, 50),
            sma_200: self.calculate_sma(candles, 200),
            ema_12: self.calculate_ema(candles, 12),
            ema_26: self.calculate_ema(candles, 26),
        }
    }

    /// Find local highs and lows
    pub fn find_extremes(&self, candles: &[Candle], window: usize) -> (Vec<usize>, Vec<usize>) {
        if candles.len() < window * 2 + 1 {
            return (Vec::new(), Vec::new());
        }

        let mut peaks = Vec::new();
        let mut troughs = Vec::new();

        for i in window..(candles.len() - window) {
            let current_high = candles[i].ohlcv.high;
            let current_low = candles[i].ohlcv.low;

            // Check for a high
            let is_peak = candles[i - window..i]
                .iter()
                .chain(candles[i + 1..=i + window].iter())
                .all(|c| c.ohlcv.high < current_high);

            if is_peak {
                peaks.push(i);
            }

            // Check for a low
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

    /// Calculate volatility (standard deviation of returns)
    pub fn calculate_volatility(&self, candles: &[Candle], period: usize) -> Option<f64> {
        if candles.len() < period + 1 {
            return None;
        }

        // Compute returns
        let returns: Vec<f64> = candles
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

        // Take the last period returns
        let recent_returns = &returns[returns.len() - period..];

        // Compute the mean return
        let mean_return: f64 = recent_returns.iter().sum::<f64>() / period as f64;

        // Compute the variance
        let variance: f64 =
            recent_returns.iter().map(|r| (r - mean_return).powi(2)).sum::<f64>() / period as f64;

        Some(variance.sqrt())
    }

    /// Calculate the Ichimoku Tenkan-sen
    pub fn calculate_tenkan_sen(&self, candles: &[Candle], period: usize) -> Vec<Price> {
        if candles.len() < period {
            return Vec::new();
        }

        let mut result = Vec::new();
        for i in (period - 1)..candles.len() {
            let slice = &candles[i + 1 - period..=i];
            let high = slice.iter().map(|c| c.ohlcv.high.value()).fold(f64::NEG_INFINITY, f64::max);
            let low = slice.iter().map(|c| c.ohlcv.low.value()).fold(f64::INFINITY, f64::min);
            result.push(Price::from((high + low) / 2.0));
        }

        result
    }

    /// Calculate the Ichimoku Kijun-sen
    pub fn calculate_kijun_sen(&self, candles: &[Candle], period: usize) -> Vec<Price> {
        self.calculate_tenkan_sen(candles, period)
    }

    /// Calculate Senkou Span A (average of Tenkan and Kijun)
    pub fn calculate_senkou_span_a(
        &self,
        candles: &[Candle],
        tenkan_period: usize,
        kijun_period: usize,
        _shift: usize,
    ) -> Vec<Price> {
        let tenkan = self.calculate_tenkan_sen(candles, tenkan_period);
        let kijun = self.calculate_kijun_sen(candles, kijun_period);
        let len = tenkan.len().min(kijun.len());
        (0..len).map(|i| Price::from((tenkan[i].value() + kijun[i].value()) / 2.0)).collect()
    }

    /// Calculate Senkou Span B
    pub fn calculate_senkou_span_b(
        &self,
        candles: &[Candle],
        period: usize,
        _shift: usize,
    ) -> Vec<Price> {
        self.calculate_tenkan_sen(candles, period)
    }

    /// Calculate the Chikou Span (closing prices shifted back)
    pub fn calculate_chikou_span(&self, candles: &[Candle], shift: usize) -> Vec<Price> {
        if candles.len() <= shift {
            return Vec::new();
        }

        candles[..candles.len() - shift].iter().map(|c| c.ohlcv.close).collect()
    }

    /// Calculate all Ichimoku components with default periods
    pub fn calculate_ichimoku(&self, candles: &[Candle]) -> IchimokuData {
        IchimokuData {
            tenkan_sen: self.calculate_tenkan_sen(candles, 9),
            kijun_sen: self.calculate_kijun_sen(candles, 26),
            senkou_span_a: self.calculate_senkou_span_a(candles, 9, 26, 26),
            senkou_span_b: self.calculate_senkou_span_b(candles, 52, 26),
            chikou_span: self.calculate_chikou_span(candles, 26),
        }
    }
}

/// Service to aggregate multiple candles into one
pub struct Aggregator;

impl Aggregator {
    /// Combine a list of candles into one for the given interval
    pub fn aggregate(candles: &[Candle], interval: TimeInterval) -> Option<Candle> {
        if candles.is_empty() {
            return None;
        }

        let open = candles.first()?.ohlcv.open;
        let close = candles.last()?.ohlcv.close;
        let high = candles.iter().map(|c| c.ohlcv.high.value()).fold(open.value(), f64::max);
        let low = candles.iter().map(|c| c.ohlcv.low.value()).fold(open.value(), f64::min);
        let volume_sum: f64 = candles.iter().map(|c| c.ohlcv.volume.value()).sum();

        let start =
            candles.first()?.timestamp.value() / interval.duration_ms() * interval.duration_ms();
        Some(Candle::new(
            Timestamp::from(start),
            OHLCV::new(open, Price::from(high), Price::from(low), close, Volume::from(volume_sum)),
        ))
    }
}

// DataValidationService removed - validation is handled in MarketAnalysisService.validate_candle()
