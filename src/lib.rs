// === 🦀 LEPTOS BITCOIN CHART WASM ===
// Clean Architecture v3.0 - только нужные модули!

pub mod domain;
pub mod infrastructure; 
pub mod app;

// === WASM EXPORTS ===
use leptos::*;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    
    // Initialize infrastructure services
    crate::infrastructure::initialize_infrastructure_services();
    
    // Mount Leptos app
    leptos::mount_to_body(|| view! { <crate::app::App/> });
}

// Export main for compatibility
#[wasm_bindgen]
pub fn main() {
    hydrate();
}

/// Проверка WebGPU поддержки
#[wasm_bindgen]
pub async fn is_webgpu_supported() -> bool {
    crate::infrastructure::WebGpuRenderer::is_webgpu_supported().await
}

/// Получить производительность рендерера
#[wasm_bindgen]
pub fn get_renderer_performance() -> String {
    // Заглушка - возвращаем статическую информацию
    "{\"backend\":\"WebGPU\",\"status\":\"ready\",\"fps\":60}".to_string()
}

// Clean WASM exports only 