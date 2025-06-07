use std::cell::RefCell;
use wasm_bindgen::JsValue;
use crate::domain::{
    chart::Chart,
    logging::{LogComponent, get_logger},
};
use crate::infrastructure::rendering::WebGpuRenderer;

/// Глобальный координатор приложения для управления рендерингом и состоянием
pub struct ChartCoordinator {
    webgpu_renderer: Option<WebGpuRenderer>,
    chart: Option<Chart>,
    canvas_id: String,
    width: u32,
    height: u32,
    is_initialized: bool,
}

impl ChartCoordinator {
    pub fn new(canvas_id: String, width: u32, height: u32) -> Self {
        get_logger().info(
            LogComponent::Application("ChartCoordinator"),
            "Creating new chart coordinator"
        );

        Self {
            webgpu_renderer: None,
            chart: None,
            canvas_id,
            width,
            height,
            is_initialized: false,
        }
    }

    /// Асинхронная инициализация с WebGPU рендерером (согласно ARCHITECTURE.md)
    pub async fn initialize_webgpu_renderer(&mut self) -> Result<(), JsValue> {
        get_logger().info(
            LogComponent::Application("ChartCoordinator"),
            "🚀 Initializing WebGPU coordinator..."
        );

        // Проверяем поддержку WebGPU
        let webgpu_supported = WebGpuRenderer::is_webgpu_supported().await;
        
        if webgpu_supported {
            let mut webgpu_renderer = WebGpuRenderer::new(
                self.canvas_id.clone(), 
                self.width, 
                self.height
            );

            if webgpu_renderer.initialize().await.is_ok() {
                self.webgpu_renderer = Some(webgpu_renderer);
                self.is_initialized = true;
                
                get_logger().info(
                    LogComponent::Application("ChartCoordinator"),
                    "✅ WebGPU coordinator initialized successfully"
                );
            } else {
                get_logger().warn(
                    LogComponent::Application("ChartCoordinator"),
                    "⚠️ WebGPU initialization failed, falling back to CPU rendering"
                );
            }
        } else {
            get_logger().warn(
                LogComponent::Application("ChartCoordinator"),
                "⚠️ WebGPU not supported in this browser"
            );
        }

        Ok(())
    }

    /// Рендеринг графика через WebGPU (согласно ARCHITECTURE.md)
    pub fn render_chart(&self) -> Result<(), JsValue> {
        if !self.is_initialized {
            return Err(JsValue::from_str("Chart coordinator not initialized"));
        }

        if let Some(chart) = &self.chart {
            if let Some(webgpu_renderer) = &self.webgpu_renderer {
                get_logger().info(
                    LogComponent::Application("ChartCoordinator"),
                    "🔥 Rendering chart via WebGPU coordinator"
                );
                
                webgpu_renderer.render_chart_parallel(chart)
            } else {
                Err(JsValue::from_str("WebGPU renderer not available"))
            }
        } else {
            Err(JsValue::from_str("No chart data available for rendering"))
        }
    }

    /// Установить данные графика
    pub fn set_chart(&mut self, chart: Chart) {
        get_logger().info(
            LogComponent::Application("ChartCoordinator"),
            &format!("Chart data updated: {} candles", chart.get_candle_count())
        );
        self.chart = Some(chart);
    }

    /// Получить ссылку на график
    pub fn get_chart(&self) -> Option<&Chart> {
        self.chart.as_ref()
    }

    /// Получить мутабельную ссылку на график
    pub fn get_chart_mut(&mut self) -> Option<&mut Chart> {
        self.chart.as_mut()
    }

    /// Обновить размеры canvas
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        
        if let Some(renderer) = &mut self.webgpu_renderer {
            renderer.set_dimensions(width, height);
        }

        get_logger().info(
            LogComponent::Application("ChartCoordinator"),
            &format!("Canvas resized to {}x{}", width, height)
        );
    }

    /// Проверить статус инициализации
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    /// Получить информацию о производительности
    pub fn get_performance_info(&self) -> String {
        if let Some(renderer) = &self.webgpu_renderer {
            renderer.get_performance_info()
        } else {
            "{\"backend\":\"none\",\"status\":\"not_initialized\"}".to_string()
        }
    }
}

// Глобальный экземпляр координатора (thread-local для WASM)
thread_local! {
    pub static GLOBAL_COORDINATOR: RefCell<Option<ChartCoordinator>> = RefCell::new(None);
}

/// Инициализация глобального координатора
pub fn initialize_global_coordinator(canvas_id: String, width: u32, height: u32) {
    GLOBAL_COORDINATOR.with(|global| {
        let coordinator = ChartCoordinator::new(canvas_id, width, height);
        *global.borrow_mut() = Some(coordinator);
    });
}

/// Получение ссылки на глобальный координатор для чтения
pub fn with_global_coordinator<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&ChartCoordinator) -> R,
{
    GLOBAL_COORDINATOR.with(|global| {
        global.borrow().as_ref().map(f)
    })
}

/// Получение мутабельной ссылки на глобальный координатор
pub fn with_global_coordinator_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut ChartCoordinator) -> R,
{
    GLOBAL_COORDINATOR.with(|global| {
        global.borrow_mut().as_mut().map(f)
    })
} 