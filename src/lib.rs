// === 🦀 LEPTOS BITCOIN CHART WASM ===
// Clean Architecture v3.0 - только нужные модули!

pub mod app;
pub mod domain;
pub mod infrastructure;

// === WASM EXPORTS ===
use leptos::*;
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start_app() {
    console_error_panic_hook::set_once();

    // Log that WASM started
    web_sys::console::log_1(&"🚀 WASM module initialized!".into());

    // Initialize infrastructure services
    crate::infrastructure::initialize_infrastructure_services();

    // Mount Leptos app to body
    web_sys::console::log_1(&"🎯 Mounting Leptos app...".into());

    // Hide the loading screen first
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(loading_div) = document.get_element_by_id("loading") {
                let _ = loading_div.set_attribute("style", "display: none;");
            }
        }
    }

    leptos::mount_to_body(|| view! { <crate::app::App/> });

    web_sys::console::log_1(&"✅ Leptos app mounted!".into());
}

/// Проверка WebGPU поддержки
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn is_webgpu_supported() -> bool {
    crate::infrastructure::WebGpuRenderer::is_webgpu_supported().await
}

/// Получить производительность рендерера
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_renderer_performance() -> String {
    crate::infrastructure::rendering::renderer::with_global_renderer(|r| r.get_performance_info())
        .unwrap_or_else(|| "{\"backend\":\"WebGPU\",\"status\":\"not_ready\"}".to_string())
}

// Clean WASM exports only
