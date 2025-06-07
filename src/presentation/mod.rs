// pub mod wasm_api; // Временно отключен - используем unified API
pub mod unified_wasm_api;

// pub use wasm_api::*;
pub use unified_wasm_api::*;

// Презентационный слой - заглушка
pub struct PresentationLayer;

impl PresentationLayer {
    pub fn new() -> Self {
        Self
    }
} 