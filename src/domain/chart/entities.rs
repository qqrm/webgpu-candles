use super::value_objects::{ChartType, Viewport};
use crate::domain::market_data::services::{Aggregator, IchimokuData};
use crate::domain::market_data::{Candle, CandleSeries, MovingAverageEngine, TimeInterval, Volume};
use std::collections::{HashMap, HashSet};

/// Domain entity - Chart
#[derive(Debug, Clone)]
pub struct Chart {
    pub id: String,
    pub chart_type: ChartType,
    pub series: HashMap<TimeInterval, CandleSeries>,
    pub viewport: Viewport,
    pub indicators: Vec<Indicator>,
    pub ichimoku: IchimokuData,
    pub ma_engines: HashMap<TimeInterval, MovingAverageEngine>,
    open_buckets: HashSet<TimeInterval>,
    revision: u64,
}

impl Chart {
    pub fn new(id: String, chart_type: ChartType, max_candles: usize) -> Self {
        let mut series = HashMap::new();
        series.insert(TimeInterval::TwoSeconds, CandleSeries::new(max_candles));
        series.insert(TimeInterval::OneMinute, CandleSeries::new(max_candles));
        series.insert(TimeInterval::FiveMinutes, CandleSeries::new(max_candles));
        series.insert(TimeInterval::FifteenMinutes, CandleSeries::new(max_candles));
        series.insert(TimeInterval::OneHour, CandleSeries::new(max_candles));
        series.insert(TimeInterval::OneDay, CandleSeries::new(max_candles));
        series.insert(TimeInterval::OneWeek, CandleSeries::new(max_candles));
        series.insert(TimeInterval::OneMonth, CandleSeries::new(max_candles));

        let mut ma_engines = HashMap::new();
        ma_engines.insert(TimeInterval::TwoSeconds, MovingAverageEngine::new());
        ma_engines.insert(TimeInterval::OneMinute, MovingAverageEngine::new());
        ma_engines.insert(TimeInterval::FiveMinutes, MovingAverageEngine::new());
        ma_engines.insert(TimeInterval::FifteenMinutes, MovingAverageEngine::new());
        ma_engines.insert(TimeInterval::OneHour, MovingAverageEngine::new());
        ma_engines.insert(TimeInterval::OneDay, MovingAverageEngine::new());
        ma_engines.insert(TimeInterval::OneWeek, MovingAverageEngine::new());
        ma_engines.insert(TimeInterval::OneMonth, MovingAverageEngine::new());

        Self {
            id,
            chart_type,
            series,
            viewport: Viewport::default(),
            indicators: Vec::new(),
            ichimoku: IchimokuData::default(),
            ma_engines,
            open_buckets: HashSet::new(),
            revision: 0,
        }
    }

    /// Monotonic data revision used by the renderer to avoid hashing all candles.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    pub fn add_candle(&mut self, candle: Candle) {
        if candle.is_empty() {
            return;
        }
        if let Some(base) = self.series.get_mut(&TimeInterval::TwoSeconds) {
            let latest_ts = base.latest().map(|c| c.timestamp.value());
            let is_new_candle = latest_ts.is_none_or(|ts| candle.timestamp.value() > ts);
            base.add_candle(candle.clone());
            if is_new_candle
                && let Some(engine) = self.ma_engines.get_mut(&TimeInterval::TwoSeconds)
            {
                engine.update_on_close(candle.ohlcv.close.value());
            }
        }
        self.update_aggregates(candle);
        self.bump_revision();
    }

    /// Add historical data, replacing existing values
    pub fn set_historical_data(&mut self, mut candles: Vec<Candle>) {
        candles.retain(|candle| !candle.is_empty());
        // Sort by timestamp for stability
        candles.sort_by_key(|a| a.timestamp.value());

        // Create a new series with the original limit
        let limit = self
            .series
            .get(&TimeInterval::TwoSeconds)
            .map(|s| s.capacity())
            .unwrap_or(candles.len());
        for s in self.series.values_mut() {
            *s = CandleSeries::new(limit);
        }
        for e in self.ma_engines.values_mut() {
            *e = MovingAverageEngine::new();
        }
        self.open_buckets.clear();

        for candle in candles {
            if let Some(base) = self.series.get_mut(&TimeInterval::TwoSeconds) {
                base.add_candle(candle.clone());
                if let Some(engine) = self.ma_engines.get_mut(&TimeInterval::TwoSeconds) {
                    engine.update_on_close(candle.ohlcv.close.value());
                }
            }
            self.update_aggregates(candle);
        }

        // Update the viewport
        self.update_viewport_for_data();
        self.bump_revision();
    }

    /// Install a pre-sorted base series without deriving every exchange
    /// timeframe. This keeps the million-candle renderer stress test focused
    /// on storage, viewport selection, LOD generation and GPU submission.
    pub fn set_base_series(&mut self, mut candles: Vec<Candle>) {
        candles.retain(|candle| !candle.is_empty());
        let limit = self
            .series
            .get(&TimeInterval::TwoSeconds)
            .map(CandleSeries::capacity)
            .unwrap_or(candles.len());
        for series in self.series.values_mut() {
            *series = CandleSeries::new(limit);
        }
        for engine in self.ma_engines.values_mut() {
            *engine = MovingAverageEngine::new();
        }
        self.open_buckets.clear();
        if let Some(base) = self.series.get_mut(&TimeInterval::TwoSeconds) {
            base.replace_all(candles);
        }
        self.update_viewport_for_data();
        self.bump_revision();
    }

    /// Prepend an ordered historical batch without disturbing the current viewport.
    ///
    /// The REST API returns candles older than the first loaded bar. Rebuilding the
    /// derived series once is both cheaper and more reliable than inserting every
    /// candle through the real-time path.
    pub fn prepend_historical_data(&mut self, mut historical: Vec<Candle>) -> usize {
        historical.retain(|candle| !candle.is_empty());
        let Some(base) = self.series.get(&TimeInterval::TwoSeconds) else {
            return 0;
        };
        let before = base.count();
        let oldest_loaded = base.get_candles().front().map(|c| c.timestamp.value());

        historical.sort_by_key(|c| c.timestamp.value());
        historical.dedup_by_key(|c| c.timestamp.value());
        if let Some(oldest) = oldest_loaded {
            historical.retain(|c| c.timestamp.value() < oldest);
        }
        if historical.is_empty() {
            return 0;
        }

        let capacity = base.capacity();
        let available = capacity.saturating_sub(before);
        if historical.len() > available {
            historical.drain(..historical.len() - available);
        }
        if historical.is_empty() {
            return 0;
        }

        let viewport = self.viewport.clone();
        historical.extend(base.get_candles().iter().cloned());
        self.set_historical_data(historical);
        self.viewport = viewport;
        self.get_candle_count().saturating_sub(before)
    }
    /// Add a new candle in real time
    pub fn add_realtime_candle(&mut self, candle: Candle) {
        if candle.is_empty() {
            return;
        }
        let is_empty = self.get_candle_count() == 0;

        if let Some(base) = self.series.get_mut(&TimeInterval::TwoSeconds) {
            let latest_ts = base.latest().map(|c| c.timestamp.value());
            let is_update = latest_ts == Some(candle.timestamp.value());
            let is_new_candle = latest_ts.is_none_or(|ts| candle.timestamp.value() > ts);
            base.add_candle(candle.clone());
            if let Some(engine) = self.ma_engines.get_mut(&TimeInterval::TwoSeconds) {
                if is_new_candle {
                    engine.update_on_close(candle.ohlcv.close.value());
                } else if is_update {
                    engine.replace_last_close(candle.ohlcv.close.value());
                }
            }
        }
        self.update_aggregates(candle);

        if is_empty {
            self.update_viewport_for_data();
        }
        self.bump_revision();
    }

    /// Get total number of candles
    pub fn get_candle_count(&self) -> usize {
        self.series.get(&TimeInterval::TwoSeconds).map(|s| s.count()).unwrap_or(0)
    }

    /// Check whether data exists
    pub fn has_data(&self) -> bool {
        self.series.get(&TimeInterval::TwoSeconds).map(|s| s.count() > 0).unwrap_or(false)
    }

    pub fn add_indicator(&mut self, indicator: Indicator) {
        self.indicators.push(indicator);
    }

    pub fn remove_indicator(&mut self, indicator_id: &str) {
        self.indicators.retain(|ind| ind.id != indicator_id);
    }

    /// Update the viewport based on candle data
    pub fn update_viewport_for_data(&mut self) {
        if let Some(base) = self.series.get(&TimeInterval::TwoSeconds)
            && let Some((min_price, max_price)) = base.price_range()
        {
            // Add padding for better visualization (5% top and bottom)
            let mut min_v = min_price.value() as f32;
            let mut max_v = max_price.value() as f32;
            let price_range = (max_v - min_v).abs().max(1e-6);
            let padding = price_range * 0.05;
            min_v -= padding;
            max_v += padding;

            self.viewport.min_price = min_v.max(0.1); // Minimum $0.1
            self.viewport.max_price = max_v;

            // Update the time range
            let candles = base.get_candles();
            if !candles.is_empty() {
                self.viewport.start_time = candles.front().unwrap().timestamp.value() as f64;
                self.viewport.end_time = candles.back().unwrap().timestamp.value() as f64;
            }
        }
    }

    pub fn zoom(&mut self, factor: f32, center_x: f32) {
        self.viewport.zoom(factor, center_x);
        if let Some(series) = self.series.get(&TimeInterval::TwoSeconds)
            && let Some((first, last)) = series.time_bounds()
        {
            self.viewport.clamp_to_data(first, last);
        }
    }

    /// Vertical zoom by price
    pub fn zoom_price(&mut self, factor: f32, center_y: f32) {
        self.viewport.zoom_price(factor, center_y);
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.viewport.pan(delta_x, delta_y);
        if let Some(series) = self.series.get(&TimeInterval::TwoSeconds)
            && let Some((first, last)) = series.time_bounds()
        {
            self.viewport.clamp_to_data(first, last);
        }
    }

    pub fn get_series(&self, interval: TimeInterval) -> Option<&CandleSeries> {
        self.series.get(&interval)
    }

    fn update_aggregates(&mut self, candle: Candle) {
        let intervals = [
            TimeInterval::OneMinute,
            TimeInterval::FiveMinutes,
            TimeInterval::FifteenMinutes,
            TimeInterval::OneHour,
            TimeInterval::OneDay,
            TimeInterval::OneWeek,
            TimeInterval::OneMonth,
        ];

        for interval in intervals.iter() {
            if let Some(series) = self.series.get_mut(interval) {
                let bucket_start =
                    candle.timestamp.value() / interval.duration_ms() * interval.duration_ms();

                let latest_ts = series.latest().map(|c| c.timestamp.value());
                if latest_ts == Some(bucket_start) {
                    let mut new_close = None;
                    if let Some(last) = series.latest_mut() {
                        if candle.ohlcv.high > last.ohlcv.high {
                            last.ohlcv.high = candle.ohlcv.high;
                        }
                        if candle.ohlcv.low < last.ohlcv.low {
                            last.ohlcv.low = candle.ohlcv.low;
                        }
                        last.ohlcv.close = candle.ohlcv.close;
                        last.ohlcv.volume =
                            Volume::from(last.ohlcv.volume.value() + candle.ohlcv.volume.value());
                        new_close = Some(last.ohlcv.close.value());
                    }
                    if let Some(close) = new_close
                        && let Some(engine) = self.ma_engines.get_mut(interval)
                    {
                        engine.replace_last_close(close);
                    }
                    self.open_buckets.insert(*interval);
                    continue;
                }

                let is_new_bucket = latest_ts.is_none_or(|ts| bucket_start > ts);
                let previous_close = if is_new_bucket {
                    series.latest().map(|c| c.ohlcv.close.value())
                } else {
                    None
                };

                if is_new_bucket
                    && self.open_buckets.remove(interval)
                    && let Some(close) = previous_close
                    && let Some(engine) = self.ma_engines.get_mut(interval)
                {
                    engine.update_on_close(close);
                }

                let new_candle = Aggregator::aggregate(std::slice::from_ref(&candle), *interval)
                    .unwrap_or_else(|| candle.clone());
                series.add_candle(new_candle);
                if is_new_bucket {
                    self.open_buckets.insert(*interval);
                }
            }
        }
    }
}

/// Simplified Indicator entity - only essential fields
#[derive(Debug, Clone)]
pub struct Indicator {
    pub id: String,
    pub indicator_type: IndicatorType,
}

impl Indicator {
    pub fn new(id: String, indicator_type: IndicatorType) -> Self {
        Self { id, indicator_type }
    }
}

/// Essential indicator types only
#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorType {
    SimpleMovingAverage,
    ExponentialMovingAverage,
    MACD,
}

// Removed unused complex structures:
// - IndicatorParameters, IndicatorStyle, PriceSource, LineStyle
// - RenderLayer, RenderElement
// - CandlestickStyle, TextStyle, FontWeight, ShapeType, ShapeStyle
// These are handled directly in the WebGPU renderer for better performance

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::market_data::{OHLCV, Price, Timestamp};

    fn candle(minute: u64) -> Candle {
        let price = 100.0 + minute as f64;
        Candle::new(
            Timestamp::from_millis(minute * 60_000),
            OHLCV::new(
                Price::from(price),
                Price::from(price + 1.0),
                Price::from(price - 1.0),
                Price::from(price + 0.5),
                Volume::from(1.0),
            ),
        )
    }

    fn empty_candle(minute: u64) -> Candle {
        let price = 100.0 + minute as f64;
        Candle::new(
            Timestamp::from_millis(minute * 60_000),
            OHLCV::new(
                Price::from(price),
                Price::from(price),
                Price::from(price),
                Price::from(price),
                Volume::from(0.0),
            ),
        )
    }

    #[test]
    fn historical_prepend_preserves_viewport_and_order() {
        let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 10);
        chart.set_historical_data((3..6).map(candle).collect());
        chart.viewport.start_time = candle(4).timestamp.value() as f64;
        chart.viewport.end_time = candle(5).timestamp.value() as f64;
        let viewport = chart.viewport.clone();

        let added = chart.prepend_historical_data((0..4).map(candle).collect());

        let series = chart.get_series(TimeInterval::TwoSeconds).unwrap();
        let timestamps: Vec<_> = series.get_candles().iter().map(|c| c.timestamp.value()).collect();
        assert_eq!(added, 3);
        assert_eq!(timestamps, (0..6).map(|m| m * 60_000).collect::<Vec<_>>());
        assert_eq!(chart.viewport, viewport);
    }

    #[test]
    fn historical_prepend_keeps_nearest_data_when_capacity_is_reached() {
        let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 5);
        chart.set_historical_data((4..7).map(candle).collect());

        let added = chart.prepend_historical_data((0..4).map(candle).collect());

        let series = chart.get_series(TimeInterval::TwoSeconds).unwrap();
        let timestamps: Vec<_> = series.get_candles().iter().map(|c| c.timestamp.value()).collect();
        assert_eq!(added, 2);
        assert_eq!(timestamps, (2..7).map(|m| m * 60_000).collect::<Vec<_>>());
    }

    #[test]
    fn revision_changes_without_scanning_candles() {
        let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 10);
        let initial = chart.revision();
        chart.add_realtime_candle(candle(1));
        let after_insert = chart.revision();
        chart.add_realtime_candle(candle(1));
        assert_ne!(after_insert, initial);
        assert_ne!(chart.revision(), after_insert);
    }

    #[test]
    fn empty_candles_never_enter_the_chart() {
        let mut chart = Chart::new("test".to_string(), ChartType::Candlestick, 10);
        chart.set_historical_data(vec![candle(1), empty_candle(2), candle(3)]);
        assert_eq!(chart.get_candle_count(), 2);

        let revision = chart.revision();
        chart.add_realtime_candle(empty_candle(4));
        assert_eq!(chart.get_candle_count(), 2);
        assert_eq!(chart.revision(), revision);
    }

    #[test]
    fn base_series_bulk_load_skips_derived_timeframes() {
        let mut chart = Chart::new("stress".to_string(), ChartType::Candlestick, 10);
        chart.set_base_series((0..10).map(candle).collect());

        assert_eq!(chart.get_candle_count(), 10);
        assert_eq!(chart.get_series(TimeInterval::OneMinute).unwrap().count(), 0);
        assert_eq!(chart.viewport.start_time, candle(0).timestamp.value() as f64);
        assert_eq!(chart.viewport.end_time, candle(9).timestamp.value() as f64);
    }
}
