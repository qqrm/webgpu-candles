use derive_more::Display;
use strum::{AsRefStr, EnumIter, EnumString};

/// Value Object - Chart type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumIter, EnumString, AsRefStr)]
pub enum ChartType {
    #[display(fmt = "Candlestick")]
    #[strum(serialize = "candlestick")]
    Candlestick,
    #[display(fmt = "Line")]
    #[strum(serialize = "line")]
    Line,
    #[display(fmt = "Area")]
    #[strum(serialize = "area")]
    Area,
    #[display(fmt = "OHLC")]
    #[strum(serialize = "ohlc")]
    OHLC,
    #[display(fmt = "Heikin")]
    #[strum(serialize = "heikin")]
    Heikin,
    #[display(fmt = "Renko")]
    #[strum(serialize = "renko")]
    Renko,
    #[display(fmt = "Point and Figure")]
    #[strum(serialize = "point-and-figure")]
    PointAndFigure,
}

/// Value Object - Viewport
#[derive(Debug, Clone, PartialEq)]
pub struct Viewport {
    pub start_time: f64,
    pub end_time: f64,
    pub min_price: f32,
    pub max_price: f32,
    pub width: u32,
    pub height: u32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            start_time: 0.0,
            end_time: 0.0,
            min_price: 0.0,
            max_price: 100.0,
            width: 800,
            height: 600,
        }
    }
}

impl Viewport {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height, ..Default::default() }
    }

    pub fn time_range(&self) -> f64 {
        self.end_time - self.start_time
    }

    pub fn price_range(&self) -> f32 {
        self.max_price - self.min_price
    }

    pub fn zoom(&mut self, factor: f32, center_x: f32) {
        let current_range = self.time_range();
        let new_range = current_range / factor as f64;
        let center_time = self.start_time + current_range * center_x as f64;

        self.start_time = center_time - new_range / 2.0;
        self.end_time = center_time + new_range / 2.0;
    }

    /// Scale prices vertically
    pub fn zoom_price(&mut self, factor: f32, center_y: f32) {
        let current_range = self.price_range();
        let new_range = current_range / factor;
        let center_price = self.max_price - current_range * center_y;

        self.min_price = center_price - new_range / 2.0;
        self.max_price = center_price + new_range / 2.0;

        if self.min_price < 0.1 {
            let shift = 0.1 - self.min_price;
            self.min_price += shift;
            self.max_price += shift;
        }
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let time_delta = self.time_range() * delta_x as f64;
        self.start_time += time_delta;
        self.end_time += time_delta;

        let price_delta = self.price_range() * delta_y;
        self.min_price += price_delta;
        self.max_price += price_delta;
    }

    /// Convert a timestamp to a screen X coordinate
    pub fn time_to_x(&self, timestamp: f64) -> f32 {
        if self.time_range() == 0.0 {
            return 0.0;
        }
        let normalized = (timestamp - self.start_time) / self.time_range();
        (normalized * self.width as f64) as f32
    }

    /// Convert a price to a screen Y coordinate
    pub fn price_to_y(&self, price: f32) -> f32 {
        if self.price_range() == 0.0 {
            return self.height as f32 / 2.0;
        }
        let normalized = (price - self.min_price) / self.price_range();
        self.height as f32 * (1.0 - normalized) // Invert Y
    }

    /// Convert a screen X coordinate back to time
    pub fn x_to_time(&self, x: f32) -> f64 {
        let normalized = x / self.width as f32;
        self.start_time + self.time_range() * normalized as f64
    }

    /// Convert a screen Y coordinate back to price
    pub fn y_to_price(&self, y: f32) -> f32 {
        let normalized = 1.0 - (y / self.height as f32); // invert Y
        self.min_price + self.price_range() * normalized
    }
}

/// Value Object - Color
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }

    pub fn from_hex(hex: u32) -> Self {
        let r = ((hex >> 16) & 0xFF) as f32 / 255.0;
        let g = ((hex >> 8) & 0xFF) as f32 / 255.0;
        let b = (hex & 0xFF) as f32 / 255.0;
        Self::rgb(r, g, b)
    }

    pub fn to_hex(&self) -> u32 {
        let r = (self.r * 255.0) as u32;
        let g = (self.g * 255.0) as u32;
        let b = (self.b * 255.0) as u32;
        (r << 16) | (g << 8) | b
    }

    pub fn with_alpha(&self, alpha: f32) -> Self {
        Self { a: alpha, ..*self }
    }

    /// Predefined colors
    pub const BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const RED: Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const TRANSPARENT: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };
}

// Constructors for color tuples
impl From<(f32, f32, f32)> for Color {
    fn from((r, g, b): (f32, f32, f32)) -> Self {
        Self::rgb(r, g, b)
    }
}

impl From<(f32, f32, f32, f32)> for Color {
    fn from((r, g, b, a): (f32, f32, f32, f32)) -> Self {
        Self::new(r, g, b, a)
    }
}

impl From<u32> for Color {
    fn from(hex: u32) -> Self {
        Self::from_hex(hex)
    }
}

// Removed ChartStyle - styling is handled directly in WebGPU renderer

// Removed unused value objects: Point, Rect, Dimensions, CursorPosition
// These are replaced by simple tuples like (f32, f32) in the actual implementation
