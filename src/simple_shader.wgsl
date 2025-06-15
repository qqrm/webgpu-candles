// Uniform buffer with chart parameters
struct ChartUniforms {
    view_proj_matrix: mat4x4<f32>,
    viewport: vec4<f32>,          // width, height, min_price, max_price
    time_range: vec4<f32>,        // start_time, end_time, time_range, _padding
    bullish_color: vec4<f32>,     // bullish candle color (green)
    bearish_color: vec4<f32>,     // bearish candle color (red)
    wick_color: vec4<f32>,        // wick color (gray)
    sma20_color: vec4<f32>,       // SMA 20 color (red)
    sma50_color: vec4<f32>,       // SMA 50 color (yellow)
    sma200_color: vec4<f32>,      // SMA 200 color (blue)
    ema12_color: vec4<f32>,       // EMA 12 color (purple)
    ema26_color: vec4<f32>,       // EMA 26 color (cyan)
    current_price_color: vec4<f32>, // 💰 current price color (bright yellow)
    render_params: vec4<f32>,     // candle_width, spacing, line_width, _padding
}

@group(0) @binding(0)
var<uniform> uniforms: ChartUniforms;

// Vertex attributes for simple geometry
struct VertexInput {
    @location(0) position_x: f32,
    @location(1) position_y: f32,
    @location(2) element_type: f32,
    @location(3) color_type: f32,
};

// Vertex shader output
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) element_type: f32,
}

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Simple coordinate transform - geometry already in NDC
    let position = vec4<f32>(vertex.position_x, vertex.position_y, 0.0, 1.0);
    out.clip_position = uniforms.view_proj_matrix * position;
    
    // Determine color depending on element type
    if (vertex.element_type < 0.5) {
        // Candle body
        if (vertex.color_type > 0.5) {
            out.color = uniforms.bullish_color; // green for bullish
        } else {
            out.color = uniforms.bearish_color;  // red for bearish
        }
    } else if (vertex.element_type < 1.5) {
        // Candle wicks
        out.color = uniforms.wick_color; // gray
    } else if (vertex.element_type < 2.5) {
        // Indicator lines with dedicated colors
        if (vertex.color_type < 2.5) {
            out.color = uniforms.sma20_color;
        } else if (vertex.color_type < 3.5) {
            out.color = uniforms.sma50_color;
        } else if (vertex.color_type < 4.5) {
            out.color = uniforms.sma200_color;
        } else if (vertex.color_type < 5.5) {
            out.color = uniforms.ema12_color;
        } else if (vertex.color_type < 6.5) {
            out.color = uniforms.ema26_color;
        } else {
            out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
        }
    } else if (vertex.element_type < 3.5) {
        // Chart grid
        out.color = vec4<f32>(0.3, 0.3, 0.3, 0.3); // semi-transparent gray
    } else if (vertex.element_type < 4.5) {
        // 💰 Current price line
        out.color = uniforms.current_price_color; // bright yellow
    } else if (vertex.element_type < 5.5) {
        // 📊 Volume bars
        if (vertex.color_type > 0.5) {
            // Bullish volume - green, slightly darker
            out.color = vec4<f32>(uniforms.bullish_color.rgb * 0.6, 0.8);
        } else {
            // Bearish volume - red, slightly darker
            out.color = vec4<f32>(uniforms.bearish_color.rgb * 0.6, 0.8);
        }
    } else if (vertex.element_type > 98.0) {
        // ULTRA-SIMPLE TEST - bright red
        out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0); // red
    } else {
        // Unknown element - white
        out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }
    
    out.element_type = vertex.element_type;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple fragment shader - return color from vertex shader
    return vec4<f32>(in.color.rgb, 1.0);
} 