//! Presentation data shared by the UI and renderer, without reactive state.

use crate::domain::market_data::{Candle, Symbol};
use crate::time_utils::format_timestamp_utc;

/// 🎯 Data for the tooltip
#[derive(Clone, Debug)]
pub struct TooltipData {
    pub candle: Candle,
    pub x: f64,
    pub y: f64,
    pub formatted_text: String,
}

impl TooltipData {
    pub fn new(symbol: &Symbol, candle: Candle, x: f64, y: f64) -> Self {
        let change = candle.ohlcv.close.value() - candle.ohlcv.open.value();
        let change_pct = (change / candle.ohlcv.open.value()) * 100.0;
        let direction = if change >= 0.0 { "UP" } else { "DOWN" };
        let time_str = format_timestamp_utc(candle.timestamp.value());

        let formatted_text = format!(
            "{}  {}\nOpen       ${:.2}\nHigh       ${:.2}\nLow        ${:.2}\nClose      ${:.2}\nChange     ${:.2} ({:.2}%)\nVolume     {:.4}\n{}",
            symbol.value(),
            direction,
            candle.ohlcv.open.value(),
            candle.ohlcv.high.value(),
            candle.ohlcv.low.value(),
            candle.ohlcv.close.value(),
            change,
            change_pct,
            candle.ohlcv.volume.value(),
            time_str
        );

        Self { candle, x, y, formatted_text }
    }
}

/// State of indicator line visibility
#[derive(Debug, Clone, PartialEq)]
pub struct LineVisibility {
    pub sma_20: bool,
    pub sma_50: bool,
    pub sma_200: bool,
    pub ema_12: bool,
    pub ema_26: bool,
}

impl Default for LineVisibility {
    fn default() -> Self {
        Self { sma_20: true, sma_50: true, sma_200: true, ema_12: true, ema_26: true }
    }
}
