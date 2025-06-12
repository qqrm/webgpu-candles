//! WebGPU rendering utilities.
//!
//! Contains type definitions for GPU buffers and the main renderer used by the
//! application.

pub mod gpu_structures;
pub mod renderer;

// Re-exports for convenient access - WebGPU only! 🚀
pub use gpu_structures::*;
pub use renderer::WebGpuRenderer;
