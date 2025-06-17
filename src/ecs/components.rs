use crate::domain::chart::{Chart, value_objects::Viewport};
use crate::domain::market_data::Candle;

/// ECS component containing a complete trading chart.
#[derive(Debug, Clone)]
pub struct ChartComponent(pub Chart);

/// ECS component storing a single candle.
#[derive(Debug, Clone)]
pub struct CandleComponent(pub Candle);

/// ECS component for viewport state.
#[derive(Debug, Clone)]
pub struct ViewportComponent(pub Viewport);
