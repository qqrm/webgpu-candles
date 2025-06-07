use wasm_bindgen::JsValue;
use crate::domain::{
    chart::Chart,
    logging::{LogComponent, get_logger},
};
use crate::infrastructure::rendering::WebGpuRenderer;

/// WebGPU Use Case для рендеринга графиков (согласно ARCHITECTURE.md)
pub struct RenderChartUseCase {
    webgpu_renderer: Option<WebGpuRenderer>,
    webgpu_supported: bool,
}

impl RenderChartUseCase {
    /// Создание нового use case
    pub fn new() -> Self {
        Self {
            webgpu_renderer: None,
            webgpu_supported: false,
        }
    }

    /// Асинхронная инициализация с WebGPU рендерером (согласно ARCHITECTURE.md)
    pub async fn initialize_webgpu_renderer(canvas_id: String, width: u32, height: u32) -> Self {
        get_logger().info(
            LogComponent::Application("RenderChartUseCase"),
            "🚀 Initializing WebGPU-only RenderChartUseCase..."
        );

        let webgpu_supported = WebGpuRenderer::is_webgpu_supported().await;
        
        let mut use_case = Self {
            webgpu_renderer: None,
            webgpu_supported,
        };

        if webgpu_supported {
            let mut webgpu_renderer = WebGpuRenderer::new(canvas_id, width, height);
            if webgpu_renderer.initialize().await.is_ok() {
                use_case.webgpu_renderer = Some(webgpu_renderer);
                
                get_logger().info(
                    LogComponent::Application("RenderChartUseCase"),
                    "✅ WebGPU renderer initialized successfully"
                );
            } else {
                get_logger().warn(
                    LogComponent::Application("RenderChartUseCase"),
                    "⚠️ WebGPU renderer initialization failed"
                );
            }
        } else {
            get_logger().warn(
                LogComponent::Application("RenderChartUseCase"),
                "⚠️ WebGPU not supported in this browser"
            );
        }

        use_case
    }
    
    /// 🚀 Чистый WebGPU рендеринг (согласно ARCHITECTURE.md)
    pub fn render_chart(&self, chart: &Chart) -> Result<(), JsValue> {
        get_logger().info(
            LogComponent::Application("RenderChartUseCase"),
            "Executing render chart use case"
        );

        if let Some(webgpu_renderer) = &self.webgpu_renderer {
            get_logger().info(
                LogComponent::Application("RenderChartUseCase"),
                "🔥 Rendering chart via WebGPU parallel processing"
            );
            
            webgpu_renderer.render_chart_parallel(chart)
        } else {
            let error_msg = if !self.webgpu_supported {
                "WebGPU not supported or not initialized"
            } else {
                "WebGPU renderer not available"
            };
            
            get_logger().error(
                LogComponent::Application("RenderChartUseCase"),
                error_msg
            );
            
            Err(JsValue::from_str(error_msg))
        }
    }

    /// Проверить готовность WebGPU
    pub fn is_webgpu_ready(&self) -> bool {
        self.webgpu_supported && self.webgpu_renderer.is_some()
    }

    /// Получить информацию о статусе WebGPU
    pub fn get_webgpu_status(&self) -> String {
        if let Some(renderer) = &self.webgpu_renderer {
            renderer.get_performance_info()
        } else if self.webgpu_supported {
            "{\"backend\":\"WebGPU\",\"status\":\"supported_but_not_initialized\"}".to_string()
        } else {
            "{\"backend\":\"WebGPU\",\"status\":\"not_supported\"}".to_string()
        }
    }
} 