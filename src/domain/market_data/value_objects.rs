use std::cmp::Ordering;

/// Value Object - Цена
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Price(f32);

impl Price {
    pub fn from(value: f32) -> Self {
        Self(value)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

impl PartialOrd for Price {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl From<f32> for Price {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

/// Value Object - Объем
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volume(f32);

impl Volume {
    pub fn from(value: f32) -> Self {
        Self(value)
    }

    pub fn value(&self) -> f32 {
        self.0
    }
}

impl From<f32> for Volume {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

/// Value Object - Временная метка
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timestamp(u64);

impl Timestamp {
    pub fn from(value: u64) -> Self {
        Self(value)
    }

    pub fn value(&self) -> u64 {
        self.0
    }

    pub fn as_f64(&self) -> f64 {
        self.0 as f64
    }
}

impl From<u64> for Timestamp {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl From<f64> for Timestamp {
    fn from(value: f64) -> Self {
        Self(value as u64)
    }
}

/// Value Object - OHLCV данные
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OHLCV {
    pub open: Price,
    pub high: Price,
    pub low: Price,
    pub close: Price,
    pub volume: Volume,
}

impl OHLCV {
    pub fn new(open: Price, high: Price, low: Price, close: Price, volume: Volume) -> Self {
        Self {
            open,
            high,
            low,
            close,
            volume,
        }
    }

    /// Проверяет валидность OHLCV данных
    pub fn is_valid(&self) -> bool {
        self.high >= self.open
            && self.high >= self.close
            && self.high >= self.low
            && self.low <= self.open
            && self.low <= self.close
            && self.volume.value() >= 0.0
    }
}

/// Value Object - Торговый символ
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol(String);

impl Symbol {
    pub fn new(symbol: String) -> Result<Self, String> {
        if symbol.is_empty() {
            return Err("Symbol cannot be empty".to_string());
        }
        Ok(Self(symbol.to_uppercase()))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Symbol {
    fn from(value: &str) -> Self {
        Self(value.to_uppercase())
    }
}

/// Value Object - Временной интервал
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInterval {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    OneHour,
    FourHours,
    OneDay,
    OneWeek,
    OneMonth,
}

impl TimeInterval {
    pub fn to_binance_str(&self) -> &'static str {
        match self {
            Self::OneMinute => "1m",
            Self::FiveMinutes => "5m",
            Self::FifteenMinutes => "15m",
            Self::ThirtyMinutes => "30m",
            Self::OneHour => "1h",
            Self::FourHours => "4h",
            Self::OneDay => "1d",
            Self::OneWeek => "1w",
            Self::OneMonth => "1M",
        }
    }

    pub fn duration_ms(&self) -> u64 {
        match self {
            Self::OneMinute => 60 * 1000,
            Self::FiveMinutes => 5 * 60 * 1000,
            Self::FifteenMinutes => 15 * 60 * 1000,
            Self::ThirtyMinutes => 30 * 60 * 1000,
            Self::OneHour => 60 * 60 * 1000,
            Self::FourHours => 4 * 60 * 60 * 1000,
            Self::OneDay => 24 * 60 * 60 * 1000,
            Self::OneWeek => 7 * 24 * 60 * 60 * 1000,
            Self::OneMonth => 30 * 24 * 60 * 60 * 1000, // Приблизительно
        }
    }
} 