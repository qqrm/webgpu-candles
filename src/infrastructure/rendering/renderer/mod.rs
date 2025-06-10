use crate::domain::market_data::Candle;
use crate::domain::{
    chart::Chart,
    logging::{LogComponent, get_logger},
};
use crate::infrastructure::rendering::gpu_structures::{CandleVertex, ChartUniforms};
use gloo::utils::document;
use js_sys;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;
use wgpu::util::DeviceExt;
thread_local! {
    static GLOBAL_RENDERER: RefCell<Option<Rc<RefCell<WebGpuRenderer>>>> = const { RefCell::new(None) };
}

/// Сохранить глобальный экземпляр рендерера
pub fn set_global_renderer(renderer: Rc<RefCell<WebGpuRenderer>>) {
    GLOBAL_RENDERER.with(|cell| {
        *cell.borrow_mut() = Some(renderer);
    });
}

/// Получить изменяемую ссылку на глобальный рендерер
pub fn with_global_renderer<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut WebGpuRenderer) -> R,
{
    GLOBAL_RENDERER.with(|cell| {
        let opt = cell.borrow_mut();
        opt.as_ref().map(|rc| f(&mut rc.borrow_mut()))
    })
}

/// Настоящий WebGPU рендерер для свечей
pub struct WebGpuRenderer {
    _canvas_id: String,
    width: u32,
    height: u32,

    // WGPU state
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    // Rendering pipeline
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    num_vertices: u32,

    // 🗄️ Кэшированные данные
    cached_vertices: Vec<CandleVertex>,
    cached_uniforms: ChartUniforms,
    cached_candle_count: usize,
    cached_zoom_level: f64,

    // 🔍 Параметры зума и панорамирования
    zoom_level: f64,
    pan_offset: f64,

    // ⏱️ Метрики производительности
    last_frame_time: f64,
    fps_samples: Vec<f64>,
}

/// Состояние видимости линий индикаторов
#[derive(Debug, Clone)]
pub struct LineVisibility {
    pub sma_20: bool,
    pub sma_50: bool,
    pub sma_200: bool,
    pub ema_12: bool,
    pub ema_26: bool,
}

impl Default for LineVisibility {
    fn default() -> Self {
        Self {
            sma_20: true,
            sma_50: true,
            sma_200: true,
            ema_12: true,
            ema_26: true,
        }
    }
}

mod geometry;
pub use geometry::{BASE_CANDLES, candle_x_position};
mod initialization;
mod performance;
mod render_loop;
