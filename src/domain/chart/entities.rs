use super::value_objects::{ChartType, Viewport};
use crate::domain::market_data::services::Aggregator;
use crate::domain::market_data::{Candle, CandleSeries, TimeInterval, Volume};
use std::collections::HashMap;

/// Domain entity - Chart
#[derive(Debug, Clone)]
pub struct Chart {
    pub id: String,
    pub chart_type: ChartType,
    pub series: HashMap<TimeInterval, CandleSeries>,
    pub viewport: Viewport,
    pub indicators: Vec<Indicator>,
}

impl Chart {
    pub fn new(id: String, chart_type: ChartType, max_candles: usize) -> Self {
        let mut series = HashMap::new();
        series.insert(TimeInterval::OneMinute, CandleSeries::new(max_candles));
        series.insert(TimeInterval::FiveMinutes, CandleSeries::new(max_candles));
        series.insert(TimeInterval::FifteenMinutes, CandleSeries::new(max_candles));
        series.insert(TimeInterval::ThirtyMinutes, CandleSeries::new(max_candles));
        series.insert(TimeInterval::OneHour, CandleSeries::new(max_candles));
        series.insert(TimeInterval::OneDay, CandleSeries::new(max_candles));
        series.insert(TimeInterval::OneWeek, CandleSeries::new(max_candles));
        series.insert(TimeInterval::OneMonth, CandleSeries::new(max_candles));

        Self { id, chart_type, series, viewport: Viewport::default(), indicators: Vec::new() }
    }

    pub fn add_candle(&mut self, candle: Candle) {
        if let Some(base) = self.series.get_mut(&TimeInterval::OneMinute) {
            base.add_candle(candle.clone());
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
            .get(&TimeInterval::OneMinute)
            .map(|s| s.capacity())
            .unwrap_or(candles.len());
        for s in self.series.values_mut() {
            *s = CandleSeries::new(limit);
        }

        for candle in candles {
            if let Some(base) = self.series.get_mut(&TimeInterval::OneMinute) {
                base.add_candle(candle.clone());
            }
            self.update_aggregates(candle);
        }

        // Update the viewport
        self.update_viewport_for_data();
    }

    /// Add a new candle in real time
    pub fn add_realtime_candle(&mut self, candle: Candle) {
        let is_empty = self.get_candle_count() == 0;

        if let Some(base) = self.series.get_mut(&TimeInterval::OneMinute) {
            base.add_candle(candle.clone());
        }
        self.update_aggregates(candle);

        if is_empty {
            self.update_viewport_for_data();
        }

    }

    /// Get total number of candles
    pub fn get_candle_count(&self) -> usize {
        self.series.get(&TimeInterval::OneMinute).map(|s| s.count()).unwrap_or(0)
    }

    /// Check whether data exists
    pub fn has_data(&self) -> bool {
        self.series.get(&TimeInterval::OneMinute).map(|s| s.count() > 0).unwrap_or(false)
    }

    pub fn add_indicator(&mut self, indicator: Indicator) {
        self.indicators.push(indicator);
    }

    pub fn remove_indicator(&mut self, indicator_id: &str) {
        self.indicators.retain(|ind| ind.id != indicator_id);
    }

    /// Update the viewport based on candle data
    pub fn update_viewport_for_data(&mut self) {
        if let Some(base) = self.series.get(&TimeInterval::OneMinute) {
            if let Some((min_price, max_price)) = base.price_range() {
                // Add padding for better visualization (5% top and bottom)
                let price_range = max_price.value() - min_price.value();
                let padding = (price_range * 0.05) as f32;

                self.viewport.min_price = (min_price.value() as f32 - padding).max(0.1); // Minimum $0.1
                self.viewport.max_price = max_price.value() as f32 + padding;

                // Update the time range
                let candles = base.get_candles();
                if !candles.is_empty() {
                    self.viewport.start_time = candles.front().unwrap().timestamp.value() as f64;
                    self.viewport.end_time = candles.back().unwrap().timestamp.value() as f64;
                }
            }
        }
    }

    pub fn zoom(&mut self, factor: f32, center_x: f32) {
        self.viewport.zoom(factor, center_x);
    }

    /// Vertical zoom by price
    pub fn zoom_price(&mut self, factor: f32, center_y: f32) {
        self.viewport.zoom_price(factor, center_y);
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        self.viewport.pan(delta_x, delta_y);
    }

    pub fn get_series(&self, interval: TimeInterval) -> Option<&CandleSeries> {
        self.series.get(&interval)
    }

    pub fn get_series_for_zoom(&self, zoom: f64) -> &CandleSeries {
        let interval = if zoom >= 4.0 {
            TimeInterval::OneMinute
        } else if zoom >= 2.0 {
            TimeInterval::FiveMinutes
        } else if zoom >= 1.0 {
            TimeInterval::FifteenMinutes
        } else {
            TimeInterval::OneHour
        };

        self.series
            .get(&interval)
            .or_else(|| self.series.get(&TimeInterval::OneMinute))
            .expect("base series not found")
    }

    fn update_aggregates(&mut self, candle: Candle) {
        let intervals = [
            TimeInterval::FiveMinutes,
            TimeInterval::FifteenMinutes,
            TimeInterval::ThirtyMinutes,
            TimeInterval::OneHour,
            TimeInterval::OneDay,
            TimeInterval::OneWeek,
            TimeInterval::OneMonth,
        ];

        for interval in intervals.iter() {
            if let Some(series) = self.series.get_mut(interval) {
                let bucket_start =
                    candle.timestamp.value() / interval.duration_ms() * interval.duration_ms();

                if let Some(last) = series.latest_mut() {
                    if last.timestamp.value() == bucket_start {
                        if candle.ohlcv.high > last.ohlcv.high {
                            last.ohlcv.high = candle.ohlcv.high;
                        }
                        if candle.ohlcv.low < last.ohlcv.low {
                            last.ohlcv.low = candle.ohlcv.low;
                        }
                        last.ohlcv.close = candle.ohlcv.close;
                        last.ohlcv.volume =
                            Volume::from(last.ohlcv.volume.value() + candle.ohlcv.volume.value());
                        continue;
                    }
                }

                let new_candle = Aggregator::aggregate(&[candle.clone()], *interval)
                    .unwrap_or_else(|| candle.clone());
                series.add_candle(new_candle);
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
