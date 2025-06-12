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

// Vertex attributes
struct VertexInput {
    @location(0) position_x: f32,
    @location(1) position_y: f32,
    @location(2) element_type: f32,
    @location(3) color_type: f32,
};

// Instance data for a candle
struct InstanceInput {
    @location(4) x: f32,
    @location(5) width: f32,
    @location(6) body_top: f32,
    @location(7) body_bottom: f32,
    @location(8) high: f32,
    @location(9) low: f32,
    @location(10) bullish: f32,
};

// Output of the vertex shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) element_type: f32,
}

@vertex
fn vs_main(vertex: VertexInput, inst: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    var x = inst.x + vertex.position_x * inst.width;
    var y: f32;
    if (vertex.element_type < 0.5) {
        y = mix(inst.body_bottom, inst.body_top, vertex.position_y);
    } else if (vertex.element_type < 1.5) {
        y = mix(inst.body_top, inst.high, vertex.position_y);
    } else {
        y = mix(inst.low, inst.body_bottom, vertex.position_y);
    }
    let position = vec4<f32>(x, y, 0.0, 1.0);
    out.clip_position = uniforms.view_proj_matrix * position;
    
    // Determine the color
    if (vertex.element_type < 0.5) {
        // Body
        if (inst.bullish > 0.5) {
            out.color = uniforms.bullish_color;
        } else {
            out.color = uniforms.bearish_color;
        }
    } else if (vertex.element_type < 2.0) {
        // Wicks
        out.color = uniforms.wick_color;
    } else if (vertex.element_type < 2.5) {
        // Indicator lines
        if (vertex.color_type < 2.5) {
            out.color = uniforms.sma20_color; // SMA 20 - red
        } else if (vertex.color_type < 3.5) {
            out.color = uniforms.sma50_color; // SMA 50 - yellow
        } else if (vertex.color_type < 4.5) {
            out.color = uniforms.sma200_color; // SMA 200 - blue
        } else if (vertex.color_type < 5.5) {
            out.color = uniforms.ema12_color; // EMA 12 - purple
        } else {
            out.color = uniforms.ema26_color; // EMA 26 - cyan
        }
    } else if (vertex.element_type < 3.5) {
        // Chart grid
        out.color = vec4<f32>(0.3, 0.3, 0.3, 0.3); // Very light gray, semi transparent
    } else if (vertex.element_type < 4.5) {
        // 💰 Current price line
        out.color = uniforms.current_price_color; // Bright yellow
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
        out.color = vec4<f32>(1.0, 0.0, 0.0, 1.0); // Red
    } else {
        // Unknown element - white
        out.color = vec4<f32>(1.0, 1.0, 1.0, 1.0);
    }
    
    out.element_type = vertex.element_type;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple fragment shader - return color from the vertex shader
    return vec4<f32>(in.color.rgb, 1.0); // Use vertex color, force alpha to 1.0
} 