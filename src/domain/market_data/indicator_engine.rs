use super::{Candle, Price};
use std::collections::VecDeque;

#[derive(Debug, Clone, Default)]
pub struct MovingAveragesData {
    pub sma_20: Vec<Price>,
    pub sma_50: Vec<Price>,
    pub sma_200: Vec<Price>,
    pub ema_12: Vec<Price>,
    pub ema_26: Vec<Price>,
}

/// Engine maintaining SMA/EMA incrementally
#[derive(Debug, Clone, Default)]
pub struct MovingAverageEngine {
    pub data: MovingAveragesData,
    sma20_win: VecDeque<f64>,
    sma20_sum: f64,
    sma50_win: VecDeque<f64>,
    sma50_sum: f64,
    sma200_win: VecDeque<f64>,
    sma200_sum: f64,
    ema12_last: Option<f64>,
    ema26_last: Option<f64>,
    alpha12: f64,
    alpha26: f64,
}

impl MovingAverageEngine {
    pub fn new() -> Self {
        Self {
            data: MovingAveragesData::default(),
            sma20_win: VecDeque::with_capacity(20),
            sma20_sum: 0.0,
            sma50_win: VecDeque::with_capacity(50),
            sma50_sum: 0.0,
            sma200_win: VecDeque::with_capacity(200),
            sma200_sum: 0.0,
            ema12_last: None,
            ema26_last: None,
            alpha12: 2.0 / (12.0 + 1.0),
            alpha26: 2.0 / (26.0 + 1.0),
        }
    }

    pub fn compute_historical(&mut self, candles: &[Candle]) {
        for c in candles {
            self.update_on_close(c.ohlcv.close.value());
        }
    }

    #[inline]
    fn update_sma(
        win: &mut VecDeque<f64>,
        sum: &mut f64,
        period: usize,
        close: f64,
        out: &mut Vec<Price>,
    ) {
        *sum += close;
        win.push_back(close);
        if win.len() > period
            && let Some(v) = win.pop_front()
        {
            *sum -= v;
        }
        if win.len() == period {
            out.push(Price::from(*sum / period as f64));
        }
    }

    #[inline]
    fn preview_sma(win: &VecDeque<f64>, sum: f64, period: usize, close: f64) -> Option<Price> {
        if win.len() < period - 1 {
            None
        } else {
            let removed = if win.len() == period - 1 { 0.0 } else { *win.front().unwrap() };
            Some(Price::from((sum + close - removed) / period as f64))
        }
    }

    #[inline]
    fn update_ema(last: &mut Option<f64>, alpha: f64, close: f64, out: &mut Vec<Price>) {
        let val = match last {
            Some(prev) => alpha * close + (1.0 - alpha) * *prev,
            None => close,
        };
        *last = Some(val);
        out.push(Price::from(val));
    }

    /// Update indicators when a candle closes
    pub fn update_on_close(&mut self, close: f64) {
        Self::update_sma(
            &mut self.sma20_win,
            &mut self.sma20_sum,
            20,
            close,
            &mut self.data.sma_20,
        );
        Self::update_sma(
            &mut self.sma50_win,
            &mut self.sma50_sum,
            50,
            close,
            &mut self.data.sma_50,
        );
        Self::update_sma(
            &mut self.sma200_win,
            &mut self.sma200_sum,
            200,
            close,
            &mut self.data.sma_200,
        );
        Self::update_ema(&mut self.ema12_last, self.alpha12, close, &mut self.data.ema_12);
        Self::update_ema(&mut self.ema26_last, self.alpha26, close, &mut self.data.ema_26);
    }

    /// Preview SMA for an in-progress candle
    pub fn preview_sma_value(&self, period: usize, close: f64) -> Option<Price> {
        match period {
            20 => Self::preview_sma(&self.sma20_win, self.sma20_sum, 20, close),
            50 => Self::preview_sma(&self.sma50_win, self.sma50_sum, 50, close),
            200 => Self::preview_sma(&self.sma200_win, self.sma200_sum, 200, close),
            _ => None,
        }
    }

    pub fn data(&self) -> &MovingAveragesData {
        &self.data
    }
}
