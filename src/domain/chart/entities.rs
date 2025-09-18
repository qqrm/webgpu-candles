use super::value_objects::{ChartType, Viewport};
use crate::domain::market_data::services::{Aggregator, IchimokuData};
use crate::domain::market_data::{Candle, CandleSeries, MovingAverageEngine, TimeInterval, Volume};
use std::collections::HashMap;

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
    aggregate_open: HashMap<TimeInterval, bool>,
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

        let mut aggregate_open = HashMap::new();
        aggregate_open.insert(TimeInterval::OneMinute, false);
        aggregate_open.insert(TimeInterval::FiveMinutes, false);
        aggregate_open.insert(TimeInterval::FifteenMinutes, false);
        aggregate_open.insert(TimeInterval::OneHour, false);
        aggregate_open.insert(TimeInterval::OneDay, false);
        aggregate_open.insert(TimeInterval::OneWeek, false);
        aggregate_open.insert(TimeInterval::OneMonth, false);

        Self {
            id,
            chart_type,
            series,
            viewport: Viewport::default(),
            indicators: Vec::new(),
            ichimoku: IchimokuData::default(),
            ma_engines,
            aggregate_open,
        }
    }

    pub fn add_candle(&mut self, candle: Candle) {
        if let Some(base) = self.series.get_mut(&TimeInterval::TwoSeconds) {
            let prev = base.count();
            base.add_candle(candle.clone());
            if base.count() > prev
                && let Some(engine) = self.ma_engines.get_mut(&TimeInterval::TwoSeconds)
            {
                engine.update_on_close(candle.ohlcv.close.value());
            }
        }
        self.update_aggregates(candle);
    }

    /// Add historical data, replacing existing values
    pub fn set_historical_data(&mut self, mut candles: Vec<Candle>) {
        // Sort by timestamp for stability
        candles.sort_by(|a, b| a.timestamp.value().cmp(&b.timestamp.value()));

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

        for open in self.aggregate_open.values_mut() {
            *open = false;
        }

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

        self.finalize_open_aggregates();
    }
    /// Add a new candle in real time
    pub fn add_realtime_candle(&mut self, candle: Candle) {
        let is_empty = self.get_candle_count() == 0;

        if let Some(base) = self.series.get_mut(&TimeInterval::TwoSeconds) {
            let prev = base.count();
            base.add_candle(candle.clone());
            if base.count() > prev
                && let Some(engine) = self.ma_engines.get_mut(&TimeInterval::TwoSeconds)
            {
                engine.update_on_close(candle.ohlcv.close.value());
            }
        }
        self.update_aggregates(candle);

        if is_empty {
            self.update_viewport_for_data();
        }
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

                let open_entry = self.aggregate_open.entry(*interval).or_insert(false);
                let mut same_bucket = false;
                let mut finalized_close = None;

                if let Some(last) = series.latest() {
                    same_bucket = last.timestamp.value() == bucket_start;
                    if !same_bucket && *open_entry {
                        finalized_close = Some(last.ohlcv.close.value());
                    }
                }

                if let Some(close) = finalized_close {
                    if let Some(engine) = self.ma_engines.get_mut(interval) {
                        engine.update_on_close(close);
                    }
                    *open_entry = false;
                }

                if same_bucket {
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
                    }
                    *open_entry = true;
                } else {
                    let new_candle =
                        Aggregator::aggregate(std::slice::from_ref(&candle), *interval)
                            .unwrap_or_else(|| candle.clone());
                    series.add_candle(new_candle);
                    *open_entry = true;
                }
            }
        }
    }

    fn finalize_open_aggregates(&mut self) {
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
            if let Some(open) = self.aggregate_open.get_mut(interval) {
                if !*open {
                    continue;
                }

                let close = self
                    .series
                    .get(interval)
                    .and_then(|s| s.latest())
                    .map(|c| c.ohlcv.close.value());

                if let Some(close) = close
                    && let Some(engine) = self.ma_engines.get_mut(interval)
                {
                    engine.update_on_close(close);
                }

                *open = false;
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
