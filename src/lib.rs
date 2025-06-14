//! Entry point for the WebGPU candles application.
//!
//! This crate follows the Clean Architecture approach as described in
//! `DOCS/ARCHITECTURE.md`. The `app` module contains the Leptos UI, `domain`
//! holds business logic, `global_state` exposes shared reactive signals, and
//! `infrastructure` implements rendering and networking services.

pub mod app;
pub mod domain;
pub mod global_state;
pub mod infrastructure;
pub mod macros;

// === WASM EXPORTS ===
use futures::lock::Mutex;
use leptos::*;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start_app() {
    console_error_panic_hook::set_once();

    // Log that WASM started
    web_sys::console::log_1(&"🚀 WASM module initialized!".into());

    // Initialize infrastructure services
    crate::infrastructure::initialize_infrastructure_services();

    // Initialize global clients
    use crate::domain::market_data::{Symbol, TimeInterval};
    use crate::infrastructure::websocket::{
        BinanceWebSocketClient, set_global_rest_client, set_global_stream_client,
    };
    let symbol = Symbol::from("BTCUSDT");
    let interval = TimeInterval::OneMinute;
    set_global_rest_client(Arc::new(Mutex::new(BinanceWebSocketClient::new(
        symbol.clone(),
        interval,
    ))));
    set_global_stream_client(Arc::new(Mutex::new(BinanceWebSocketClient::new(symbol, interval))));

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

/// Check WebGPU support
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn is_webgpu_supported() -> bool {
    crate::infrastructure::WebGpuRenderer::is_webgpu_supported().await
}

/// Get renderer performance
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_renderer_performance() -> String {
    crate::infrastructure::rendering::renderer::with_global_renderer(|r| r.get_performance_info())
        .unwrap_or_else(|| "{\"backend\":\"WebGPU\",\"status\":\"not_ready\"}".to_string())
}

/// Get GPU memory statistics
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_gpu_memory_usage() -> String {
    crate::infrastructure::rendering::renderer::with_global_renderer(|r| r.log_gpu_memory_usage())
        .unwrap_or_else(|| "{}".to_string())
}

// Clean WASM exports only
