use crate::domain::chart::{Chart, value_objects::Viewport};
use crate::domain::market_data::Candle;
use leptos::RwSignal;

/// ECS component containing a reactive trading chart.
#[derive(Debug, Clone, Copy)]
pub struct ChartComponent(pub RwSignal<Chart>);

/// ECS component storing a single candle.
#[derive(Debug, Clone)]
pub struct CandleComponent(pub Candle);

/// ECS component for viewport state.
#[derive(Debug, Clone)]
pub struct ViewportComponent(pub Viewport);
