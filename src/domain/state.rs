use std::sync::Arc;
use std::time::Duration;

use crate::domain::market_data::entities::Candle;

/// Domain-level immutable data.
#[derive(Clone, Debug)]
pub struct DomainState {
    pub timeframe: Duration,
    pub candles: Arc<Vec<Candle>>,
    pub indicators: Arc<Vec<f32>>,
}

impl DomainState {
    pub fn new(timeframe: Duration, candles: Arc<Vec<Candle>>) -> Self {
        Self { timeframe, candles, indicators: Arc::new(Vec::new()) }
    }
}
