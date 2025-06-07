pub mod webgpu;
pub mod webgpu_renderer;
pub mod gpu_structures;
pub mod candle_renderer;

// Re-exports for convenient access - WebGPU only! 🚀
pub use webgpu::*;
pub use webgpu_renderer::WebGpuRenderer;
pub use gpu_structures::*;
pub use candle_renderer::*; 