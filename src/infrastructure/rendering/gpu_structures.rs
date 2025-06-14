use bytemuck::{Pod, Zeroable};

/// Indicator types for GPU rendering
#[derive(Debug, Clone, Copy)]
pub enum IndicatorType {
    SMA20,
    SMA50,
    SMA200,
    EMA12,
    EMA26,
    Tenkan,
    Kijun,
    SenkouA,
    SenkouB,
    Chikou,
}

/// GPU representation of a candle for the vertex buffer
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CandleVertex {
    /// X position (time in normalized coordinates)
    pub position_x: f32,
    /// Y position (price in normalized coordinates)
    pub position_y: f32,
    /// Element type: 0 = body, 1 = wick, 2 = indicator line, 3 = grid, 4 = current price line
    pub element_type: f32,
    /// Color/indicator: for candles 0/1, for indicators: 2=SMA20, 3=SMA50, 4=SMA200, 5=EMA12, 6=EMA26, 7 = current price
    pub color_type: f32,
}

impl CandleVertex {
    /// Create vertex for the candle body
    pub fn body_vertex(x: f32, y: f32, is_bullish: bool) -> Self {
        Self {
            position_x: x,
            position_y: y,
            element_type: 0.0, // body
            color_type: if is_bullish { 1.0 } else { 0.0 },
        }
    }

    /// Create vertex for the candle wick (used for grid lines)
    pub fn wick_vertex(x: f32, y: f32) -> Self {
        Self { position_x: x, position_y: y, element_type: 1.0, color_type: 0.5 }
    }

    /// Create vertex for the upper wick
    pub fn upper_wick_vertex(x: f32, y: f32) -> Self {
        Self { position_x: x, position_y: y, element_type: 1.0, color_type: 0.5 }
    }

    /// Create vertex for the lower wick
    pub fn lower_wick_vertex(x: f32, y: f32) -> Self {
        Self { position_x: x, position_y: y, element_type: 1.6, color_type: 0.5 }
    }

    /// Create vertex for an indicator line
    pub fn indicator_vertex(x: f32, y: f32, indicator_type: IndicatorType) -> Self {
        let color_type = match indicator_type {
            IndicatorType::SMA20 => 2.0,
            IndicatorType::SMA50 => 3.0,
            IndicatorType::SMA200 => 4.0,
            IndicatorType::EMA12 => 5.0,
            IndicatorType::EMA26 => 6.0,
            IndicatorType::Tenkan => 10.0,
            IndicatorType::Kijun => 11.0,
            IndicatorType::SenkouA => 12.0,
            IndicatorType::SenkouB => 13.0,
            IndicatorType::Chikou => 14.0,
        };

        Self {
            position_x: x,
            position_y: y,
            element_type: 2.0, // indicator line
            color_type,
        }
    }

    /// Create vertex for the chart grid
    pub fn grid_vertex(x: f32, y: f32) -> Self {
        Self {
            position_x: x,
            position_y: y,
            element_type: 3.0, // grid
            color_type: 0.2,   // very light gray
        }
    }

    /// 💰 Create vertex for the current price line
    pub fn current_price_vertex(x: f32, y: f32) -> Self {
        Self {
            position_x: x,
            position_y: y,
            element_type: 4.0, // current price line
            color_type: 7.0,   // special color for current price
        }
    }

    /// 📊 Create vertex for volume bars
    pub fn volume_vertex(x: f32, y: f32, is_bullish: bool) -> Self {
        Self {
            position_x: x,
            position_y: y,
            element_type: 5.0,                              // volume bar
            color_type: if is_bullish { 1.0 } else { 0.0 }, // same color as candles
        }
    }

    /// Create vertex for the Ichimoku cloud area
    pub fn ichimoku_vertex(x: f32, y: f32, bullish: bool) -> Self {
        Self {
            position_x: x,
            position_y: y,
            element_type: 6.0,
            color_type: if bullish { 8.0 } else { 9.0 },
        }
    }

    /// Vertex buffer descriptor for wgpu
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CandleVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position_x
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32,
                },
                // position_y
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<f32>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32,
                },
                // element_type
                wgpu::VertexAttribute {
                    offset: (2 * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
                // color_type
                wgpu::VertexAttribute {
                    offset: (3 * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

/// Attributes of a single candle for instanced drawing
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CandleInstance {
    /// X position in NDC coordinates
    pub x: f32,
    /// Candle width
    pub width: f32,
    /// Top of the body (max(open, close))
    pub body_top: f32,
    /// Bottom of the body (min(open, close))
    pub body_bottom: f32,
    /// Maximum price (high)
    pub high: f32,
    /// Minimum price (low)
    pub low: f32,
    /// Whether the candle is bullish (1.0/0.0)
    pub bullish: f32,
    pub _padding: f32,
}

impl CandleInstance {
    /// Instance buffer layout
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CandleInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 4,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 12,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 20,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32,
                },
                wgpu::VertexAttribute {
                    offset: 24,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

/// Uniform buffer for global rendering parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ChartUniforms {
    /// Viewport transformation matrix
    pub view_proj_matrix: [[f32; 4]; 4],
    /// Viewport dimensions (width, height, min_price, max_price)
    pub viewport: [f32; 4],
    /// Time range (start_time, end_time, time_range, _padding)
    pub time_range: [f32; 4],
    /// Colors (bullish_r, bullish_g, bullish_b, bullish_a)
    pub bullish_color: [f32; 4],
    /// Colors (bearish_r, bearish_g, bearish_b, bearish_a)
    pub bearish_color: [f32; 4],
    /// Wick color (wick_r, wick_g, wick_b, wick_a)
    pub wick_color: [f32; 4],
    /// SMA 20 color (sma20_r, sma20_g, sma20_b, sma20_a)
    pub sma20_color: [f32; 4],
    /// SMA 50 color (sma50_r, sma50_g, sma50_b, sma50_a)
    pub sma50_color: [f32; 4],
    /// SMA 200 color (sma200_r, sma200_g, sma200_b, sma200_a)
    pub sma200_color: [f32; 4],
    /// EMA 12 color (ema12_r, ema12_g, ema12_b, ema12_a)
    pub ema12_color: [f32; 4],
    /// EMA 26 color (ema26_r, ema26_g, ema26_b, ema26_a)
    pub ema26_color: [f32; 4],
    /// 💰 Current price color (current_price_r, current_price_g, current_price_b, current_price_a)
    pub current_price_color: [f32; 4],
    /// Rendering parameters (candle_width, spacing, line_width, _padding)
    pub render_params: [f32; 4],
}

impl Default for ChartUniforms {
    fn default() -> Self {
        Self::new()
    }
}

impl ChartUniforms {
    pub fn new() -> Self {
        Self {
            view_proj_matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            viewport: [800.0, 600.0, 0.0, 100.0],
            time_range: [0.0, 0.0, 0.0, 0.0],
            bullish_color: [0.455, 0.780, 0.529, 1.0], // #74c787 - buy
            bearish_color: [0.882, 0.424, 0.282, 1.0], // #e16c48 - sell
            wick_color: [0.6, 0.6, 0.6, 1.0],          // gray
            sma20_color: [1.0, 0.0, 0.0, 1.0],         // bright red
            sma50_color: [1.0, 0.8, 0.0, 1.0],         // yellow
            sma200_color: [0.2, 0.4, 0.8, 1.0],        // blue
            ema12_color: [0.8, 0.2, 0.8, 1.0],         // violet
            ema26_color: [0.0, 0.8, 0.8, 1.0],         // cyan
            current_price_color: [1.0, 1.0, 0.0, 0.8], // 💰 bright yellow with transparency
            render_params: [8.0, 2.0, 1.0, 0.0],       // width, spacing, line_width, padding
        }
    }
}

/// Geometry generator for candles
pub struct CandleGeometry;

impl CandleGeometry {
    /// Create vertices for a single candle
    #[allow(clippy::too_many_arguments)]
    pub fn create_candle_vertices(
        _timestamp: f64,
        open: f32,
        _high: f32,
        _low: f32,
        close: f32,
        x_normalized: f32,
        open_y: f32,
        high_y: f32,
        low_y: f32,
        close_y: f32,
        width: f32,
    ) -> Vec<CandleVertex> {
        let mut vertices = Vec::new();
        let is_bullish = close > open;
        let half_width = width * 0.5;

        // Determine candle body coordinates
        let body_top = if is_bullish { close_y } else { open_y };
        let body_bottom = if is_bullish { open_y } else { close_y };

        // Create a rectangle for the candle body (2 triangles = 6 vertices)
        let body_vertices = [
            // First triangle
            CandleVertex::body_vertex(x_normalized - half_width, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized + half_width, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized - half_width, body_top, is_bullish),
            // Second triangle
            CandleVertex::body_vertex(x_normalized + half_width, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized + half_width, body_top, is_bullish),
            CandleVertex::body_vertex(x_normalized - half_width, body_top, is_bullish),
        ];

        vertices.extend_from_slice(&body_vertices);

        // Slightly round the corners with extra triangles
        // Increase rounding for more pronounced candle corners
        let corner = width * 0.35;
        let rounded_corners = [
            // Top left corner
            CandleVertex::body_vertex(x_normalized - half_width, body_top - corner, is_bullish),
            CandleVertex::body_vertex(x_normalized - half_width + corner, body_top, is_bullish),
            CandleVertex::body_vertex(x_normalized - half_width, body_top, is_bullish),
            // Top right corner
            CandleVertex::body_vertex(x_normalized + half_width - corner, body_top, is_bullish),
            CandleVertex::body_vertex(x_normalized + half_width, body_top - corner, is_bullish),
            CandleVertex::body_vertex(x_normalized + half_width, body_top, is_bullish),
            // Bottom left corner
            CandleVertex::body_vertex(x_normalized - half_width, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized - half_width + corner, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized - half_width, body_bottom + corner, is_bullish),
            // Bottom right corner
            CandleVertex::body_vertex(x_normalized + half_width, body_bottom, is_bullish),
            CandleVertex::body_vertex(x_normalized + half_width, body_bottom + corner, is_bullish),
            CandleVertex::body_vertex(x_normalized + half_width - corner, body_bottom, is_bullish),
        ];

        vertices.extend_from_slice(&rounded_corners);

        // Create lines for the upper and lower wicks
        let wick_width = width * 0.1; // wick is thinner than the body
        let wick_half = wick_width * 0.5;

        // Upper wick (if present)
        if high_y > body_top {
            let upper_wick = [
                CandleVertex::wick_vertex(x_normalized - wick_half, body_top),
                CandleVertex::wick_vertex(x_normalized + wick_half, body_top),
                CandleVertex::wick_vertex(x_normalized - wick_half, high_y),
                CandleVertex::wick_vertex(x_normalized + wick_half, body_top),
                CandleVertex::wick_vertex(x_normalized + wick_half, high_y),
                CandleVertex::wick_vertex(x_normalized - wick_half, high_y),
            ];
            vertices.extend_from_slice(&upper_wick);
        }

        // Lower wick (if present)
        if low_y < body_bottom {
            let lower_wick = [
                CandleVertex::wick_vertex(x_normalized - wick_half, low_y),
                CandleVertex::wick_vertex(x_normalized + wick_half, low_y),
                CandleVertex::wick_vertex(x_normalized - wick_half, body_bottom),
                CandleVertex::wick_vertex(x_normalized + wick_half, low_y),
                CandleVertex::wick_vertex(x_normalized + wick_half, body_bottom),
                CandleVertex::wick_vertex(x_normalized - wick_half, body_bottom),
            ];
            vertices.extend_from_slice(&lower_wick);
        }

        vertices
    }

    /// 💰 Create vertices for the current price line
    pub fn create_current_price_line(current_price_y: f32, line_width: f32) -> Vec<CandleVertex> {
        let half_width = line_width * 0.5;

        // Horizontal line across the entire screen
        vec![
            CandleVertex::current_price_vertex(-1.0, current_price_y - half_width),
            CandleVertex::current_price_vertex(1.0, current_price_y - half_width),
            CandleVertex::current_price_vertex(-1.0, current_price_y + half_width),
            CandleVertex::current_price_vertex(-1.0, current_price_y + half_width),
            CandleVertex::current_price_vertex(1.0, current_price_y - half_width),
            CandleVertex::current_price_vertex(1.0, current_price_y + half_width),
        ]
    }

    /// Create vertices for an indicator line - improved algorithm for solid lines
    pub fn create_indicator_line_vertices(
        points: &[(f32, f32)], // (x_normalized, y_normalized) points
        indicator_type: IndicatorType,
        line_width: f32,
    ) -> Vec<CandleVertex> {
        if points.len() < 2 {
            return Vec::new();
        }

        let mut vertices = Vec::new();
        let half_width = (line_width * 0.3).max(0.001); // thinner line for better look

        // Create a continuous line as a triangle strip
        for i in 0..(points.len() - 1) {
            let (x1, y1) = points[i];
            let (x2, y2) = points[i + 1];

            // Compute the perpendicular vector for the correct line thickness
            let dx = x2 - x1;
            let dy = y2 - y1;
            let length = (dx * dx + dy * dy).sqrt();

            // Normalized perpendicular vector
            let (perp_x, perp_y) = if length > 0.0001 {
                (-dy / length * half_width, dx / length * half_width)
            } else {
                (0.0, half_width) // vertical line
            };

            // Create a rectangle as two triangles without gaps
            let segment_vertices = [
                // First triangle
                CandleVertex::indicator_vertex(x1 - perp_x, y1 - perp_y, indicator_type),
                CandleVertex::indicator_vertex(x1 + perp_x, y1 + perp_y, indicator_type),
                CandleVertex::indicator_vertex(x2 - perp_x, y2 - perp_y, indicator_type),
                // Second triangle
                CandleVertex::indicator_vertex(x1 + perp_x, y1 + perp_y, indicator_type),
                CandleVertex::indicator_vertex(x2 + perp_x, y2 + perp_y, indicator_type),
                CandleVertex::indicator_vertex(x2 - perp_x, y2 - perp_y, indicator_type),
            ];

            vertices.extend_from_slice(&segment_vertices);
        }

        vertices
    }

    /// Create vertices for the Ichimoku cloud (Span A/B area and lines)
    pub fn create_ichimoku_cloud(
        span_a: &[(f32, f32)],
        span_b: &[(f32, f32)],
        line_width: f32,
    ) -> Vec<CandleVertex> {
        let len = span_a.len().min(span_b.len());
        if len < 2 {
            return Vec::new();
        }

        let mut vertices = Vec::new();

        // Cloud area
        for i in 0..(len - 1) {
            let (x1a, y1a) = span_a[i];
            let (x2a, y2a) = span_a[i + 1];
            let (x1b, y1b) = span_b[i];
            let (x2b, y2b) = span_b[i + 1];
            let bullish = (y1a + y2a) / 2.0 >= (y1b + y2b) / 2.0;
            let tri = [
                CandleVertex::ichimoku_vertex(x1a, y1a, bullish),
                CandleVertex::ichimoku_vertex(x1b, y1b, bullish),
                CandleVertex::ichimoku_vertex(x2a, y2a, bullish),
                CandleVertex::ichimoku_vertex(x2a, y2a, bullish),
                CandleVertex::ichimoku_vertex(x1b, y1b, bullish),
                CandleVertex::ichimoku_vertex(x2b, y2b, bullish),
            ];
            vertices.extend_from_slice(&tri);
        }

        vertices.extend(Self::create_indicator_line_vertices(
            span_a,
            IndicatorType::SenkouA,
            line_width,
        ));
        vertices.extend(Self::create_indicator_line_vertices(
            span_b,
            IndicatorType::SenkouB,
            line_width,
        ));

        vertices
    }

    /// Create vertices for the chart grid
    pub fn create_grid_vertices(
        _viewport_width: f32,
        _viewport_height: f32,
        grid_lines_x: u32,
        grid_lines_y: u32,
    ) -> Vec<CandleVertex> {
        let mut vertices = Vec::new();
        let line_width = 0.002; // thin grid lines

        // Vertical lines
        for i in 0..=grid_lines_x {
            let x = i as f32 / grid_lines_x as f32 * 2.0 - 1.0; // normalize to [-1, 1]
            let half_width = line_width * 0.5;

            // Vertical line as a thin rectangle
            vertices.extend_from_slice(&[
                CandleVertex::wick_vertex(x - half_width, -1.0),
                CandleVertex::wick_vertex(x + half_width, -1.0),
                CandleVertex::wick_vertex(x - half_width, 1.0),
                CandleVertex::wick_vertex(x + half_width, -1.0),
                CandleVertex::wick_vertex(x + half_width, 1.0),
                CandleVertex::wick_vertex(x - half_width, 1.0),
            ]);
        }

        // Horizontal lines
        for i in 0..=grid_lines_y {
            let y = i as f32 / grid_lines_y as f32 * 2.0 - 1.0; // normalize to [-1, 1]
            let half_width = line_width * 0.5;

            // Horizontal line as a thin rectangle
            vertices.extend_from_slice(&[
                CandleVertex::wick_vertex(-1.0, y - half_width),
                CandleVertex::wick_vertex(1.0, y - half_width),
                CandleVertex::wick_vertex(-1.0, y + half_width),
                CandleVertex::wick_vertex(1.0, y - half_width),
                CandleVertex::wick_vertex(1.0, y + half_width),
                CandleVertex::wick_vertex(-1.0, y + half_width),
            ]);
        }

        vertices
    }

    /// Create a smart price grid with nice levels
    pub fn create_price_grid(
        min_price: f32,
        max_price: f32,
        chart_width: f32,
        chart_height: f32,
        time_lines: u32,
        price_lines: u32,
    ) -> Vec<CandleVertex> {
        let mut vertices = Vec::new();
        let grid_line_width = 0.001; // very thin grid lines
        let half_width = grid_line_width * 0.5;

        // Vertical lines (time grid)
        for i in 1..time_lines {
            // Skip the outer lines
            let x = (i as f32 / time_lines as f32) * chart_width - 1.0;

            // Vertical line
            vertices.extend_from_slice(&[
                CandleVertex::grid_vertex(x - half_width, -1.0),
                CandleVertex::grid_vertex(x + half_width, -1.0),
                CandleVertex::grid_vertex(x - half_width, 1.0),
                CandleVertex::grid_vertex(x + half_width, -1.0),
                CandleVertex::grid_vertex(x + half_width, 1.0),
                CandleVertex::grid_vertex(x - half_width, 1.0),
            ]);
        }

        // Horizontal lines (price grid)
        let price_range = max_price - min_price;
        let nice_step = Self::calculate_nice_price_step(price_range, price_lines);

        // Find the first nice price level
        let first_price = ((min_price / nice_step).ceil() * nice_step).max(min_price);

        let mut current_price = first_price;
        while current_price <= max_price {
            // Convert price to Y coordinate
            let y = -1.0 + ((current_price - min_price) / price_range) * chart_height;

            // Horizontal line
            vertices.extend_from_slice(&[
                CandleVertex::grid_vertex(-1.0, y - half_width),
                CandleVertex::grid_vertex(1.0, y - half_width),
                CandleVertex::grid_vertex(-1.0, y + half_width),
                CandleVertex::grid_vertex(1.0, y - half_width),
                CandleVertex::grid_vertex(1.0, y + half_width),
                CandleVertex::grid_vertex(-1.0, y + half_width),
            ]);

            current_price += nice_step;
        }

        vertices
    }

    /// Calculate a nice step for the price grid
    fn calculate_nice_price_step(price_range: f32, target_lines: u32) -> f32 {
        let raw_step = price_range / target_lines as f32;

        // Determine the order of magnitude
        let magnitude = 10.0_f32.powf(raw_step.log10().floor());

        // Normalize to the range [1, 10)
        let normalized = raw_step / magnitude;

        // Choose a nice value
        let nice_normalized = if normalized <= 1.0 {
            1.0
        } else if normalized <= 2.0 {
            2.0
        } else if normalized <= 5.0 {
            5.0
        } else {
            10.0
        };

        nice_normalized * magnitude
    }
}
