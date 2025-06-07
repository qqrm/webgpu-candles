use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use js_sys::Array;

use crate::domain::market_data::{Symbol, TimeInterval};

/// WASM API для взаимодействия с JavaScript
/// Минимальная логика - только мост к application слою

#[wasm_bindgen]
pub struct PriceChartApi {
    // Внутреннее состояние скрыто от JS
}

#[wasm_bindgen]
impl PriceChartApi {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    /// Подключиться к реальным данным
    #[wasm_bindgen(js_name = connectToSymbol)]
    pub fn connect_to_symbol(&mut self, symbol: &str, interval: &str) -> Result<(), JsValue> {
        let symbol = Symbol::from(symbol);
        let interval = match interval {
            "1m" => TimeInterval::OneMinute,
            "5m" => TimeInterval::FiveMinutes,
            "15m" => TimeInterval::FifteenMinutes,
            "1h" => TimeInterval::OneHour,
            "1d" => TimeInterval::OneDay,
            _ => return Err(JsValue::from_str("Unsupported interval")),
        };

        // Делегируем в application слой
        // TODO: Здесь будет вызов координатора
        #[allow(unused_unsafe)]
        unsafe {
            web_sys::console::log_1(&format!(
                "API: Connecting to {} with {} interval", 
                symbol.value(), 
                interval.to_binance_str()
            ).into());
        }

        Ok(())
    }

    /// Получить количество свечей
    #[wasm_bindgen(js_name = getCandlesCount)]
    pub fn get_candles_count(&self) -> usize {
        // TODO: Получить из application слоя
        0
    }

    /// Получить последнюю цену
    #[wasm_bindgen(js_name = getLatestPrice)]
    pub fn get_latest_price(&self) -> f32 {
        // TODO: Получить из application слоя
        0.0
    }

    /// Получить скользящие средние
    #[wasm_bindgen(js_name = getMovingAverages)]
    pub fn get_moving_averages(&self) -> Array {
        // TODO: Получить из application слоя
        let result = Array::new();
        // Пример: result.push(&JsValue::from_f64(price));
        result
    }

    /// Получить волатильность
    #[wasm_bindgen(js_name = getVolatility)]
    pub fn get_volatility(&self) -> Option<f32> {
        // TODO: Получить из application слоя
        None
    }

    /// Установить размер canvas
    #[wasm_bindgen(js_name = setCanvasSize)]
    pub fn set_canvas_size(&mut self, width: u32, height: u32) -> Result<(), JsValue> {
        #[allow(unused_unsafe)]
        unsafe {
            web_sys::console::log_1(&format!(
                "API: Setting canvas size to {}x{}", 
                width, height
            ).into());
        }
        
        // TODO: Передать в application слой для обновления viewport
        Ok(())
    }

    /// Включить/выключить индикатор
    #[wasm_bindgen(js_name = toggleIndicator)]
    pub fn toggle_indicator(&mut self, indicator_name: &str, enabled: bool) -> Result<(), JsValue> {
        #[allow(unused_unsafe)]
        unsafe {
            web_sys::console::log_1(&format!(
                "API: {} indicator '{}'", 
                if enabled { "Enabling" } else { "Disabling" },
                indicator_name
            ).into());
        }

        // TODO: Передать в application слой
        Ok(())
    }
}

/// Простые функции для совместимости с существующим JS кодом
#[wasm_bindgen]
pub fn get_candles_count() -> usize {
    // Обратная совместимость
    0
}

#[wasm_bindgen]
pub fn get_latest_price() -> f32 {
    // Обратная совместимость  
    0.0
} 