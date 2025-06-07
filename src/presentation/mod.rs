pub mod wasm_api;

pub use wasm_api::*;

// Презентационный слой - заглушка
pub struct PresentationLayer;

impl PresentationLayer {
    pub fn new() -> Self {
        Self
    }
} 