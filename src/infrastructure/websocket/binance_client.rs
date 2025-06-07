use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebSocket, MessageEvent, ErrorEvent, CloseEvent};
use std::rc::Rc;
use std::cell::RefCell;

use crate::domain::market_data::{repositories::MarketDataRepository, Candle, Symbol, TimeInterval};
use super::dto::{BinanceKlineData, BinanceSubscription};

// Helper function for logging
fn log(s: &str) {
    #[allow(unused_unsafe)]
    unsafe {
        web_sys::console::log_1(&s.into());
    }
}

/// Helper function to update WebSocket status in UI
fn update_ws_status(status: &str, is_connected: bool) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(element) = document.get_element_by_id("ws-status") {
                element.set_text_content(Some(&format!("WebSocket: {}", status)));
                
                let style_value = if is_connected {
                    "text-align: center; margin: 10px; padding: 10px; background: #006600; border-radius: 5px;"
                } else {
                    "text-align: center; margin: 10px; padding: 10px; background: #660000; border-radius: 5px;"
                };
                
                let _ = element.set_attribute("style", style_value);
            }
        }
    }
}

/// Binance WebSocket клиент - инфраструктурная реализация
pub struct BinanceWebSocketClient {
    websocket: Option<WebSocket>,
    url: String,
    on_candle_callback: Rc<RefCell<Option<Box<dyn Fn(Candle)>>>>,
    connected: Rc<RefCell<bool>>,
}

impl Default for BinanceWebSocketClient {
    fn default() -> Self {
        Self {
            websocket: None,
            url: "wss://stream.binance.com:9443/ws".to_string(),
            on_candle_callback: Rc::new(RefCell::new(None)),
            connected: Rc::new(RefCell::new(false)),
        }
    }
}

impl BinanceWebSocketClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_testnet() -> Self {
        Self {
            websocket: None,
            url: "wss://testnet.binance.vision/ws".to_string(),
            on_candle_callback: Rc::new(RefCell::new(None)),
            connected: Rc::new(RefCell::new(false)),
        }
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.borrow()
    }

    pub fn connect(&mut self, symbol: &Symbol, interval: TimeInterval) -> Result<(), JsValue> {
        let ws_url = format!(
            "{}/{}@kline_{}",
            self.url,
            symbol.value().to_lowercase(),
            interval.to_binance_str()
        );

        log(&format!("🔌 Infrastructure: Connecting to: {}", ws_url));
        update_ws_status("Connecting...", false);

        let ws = WebSocket::new(&ws_url)?;

        // Setup connection handlers
        self.setup_handlers(&ws)?;
        
        self.websocket = Some(ws);
        Ok(())
    }

    pub fn set_candle_callback<F>(&mut self, callback: F) 
    where
        F: Fn(Candle) + 'static,
    {
        *self.on_candle_callback.borrow_mut() = Some(Box::new(callback));
        
        log("🔧 Infrastructure: Candle callback set");
    }

    fn setup_handlers(&mut self, ws: &WebSocket) -> Result<(), JsValue> {
        let connected_clone = self.connected.clone();
        
        // Open handler
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            log("🔗 Infrastructure: WebSocket connected successfully!");
            *connected_clone.borrow_mut() = true;
            update_ws_status("Connected ✅", true);
        }) as Box<dyn FnMut(JsValue)>);

        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        // Message handler с правильным callback
        let callback_clone = self.on_candle_callback.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let data_str: String = txt.into();
                
                match serde_json::from_str::<BinanceKlineData>(&data_str) {
                    Ok(kline_data) => {
                        match kline_data.kline.to_domain_candle() {
                            Ok(candle) => {
                                log(&format!(
                                    "📡 Infrastructure: Received candle - {} O:{} H:{} L:{} C:{} V:{}",
                                    candle.timestamp.value(),
                                    candle.ohlcv.open.value(),
                                    candle.ohlcv.high.value(),
                                    candle.ohlcv.low.value(),
                                    candle.ohlcv.close.value(),
                                    candle.ohlcv.volume.value()
                                ));
                                
                                // Вызываем callback правильно
                                if let Some(callback) = callback_clone.borrow().as_ref() {
                                    log("🚀 Infrastructure: Calling Application Layer callback");
                                    callback(candle);
                                } else {
                                    log("⚠️ Infrastructure: No callback set, data will be lost");
                                }
                            }
                            Err(e) => {
                                log(&format!("❌ Infrastructure: Failed to convert kline: {:?}", e));
                            },
                        }
                    }
                    Err(e) => {
                        log(&format!("❌ Infrastructure: Failed to parse JSON: {}", e));
                    },
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // Error handler
        let connected_clone_error = self.connected.clone();
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            log(&format!("❌ Infrastructure: WebSocket error: {:?}", e));
            *connected_clone_error.borrow_mut() = false;
            update_ws_status("Error ❌", false);
        }) as Box<dyn FnMut(ErrorEvent)>);

        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        // Close handler
        let connected_clone_close = self.connected.clone();
        let onclose_callback = Closure::wrap(Box::new(move |e: CloseEvent| {
            log(&format!("🔌 Infrastructure: WebSocket closed: {} - {}", e.code(), e.reason()));
            *connected_clone_close.borrow_mut() = false;
            update_ws_status("Disconnected", false);
        }) as Box<dyn FnMut(CloseEvent)>);

        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();

        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), JsValue> {
        if let Some(ws) = &self.websocket {
            ws.close()?;
            self.websocket = None;
            *self.connected.borrow_mut() = false;
            update_ws_status("Disconnected", false);
        }
        Ok(())
    }

    /// Отправить подписку на символ
    pub fn subscribe(&self, symbol: &Symbol, interval: TimeInterval) -> Result<(), JsValue> {
        if let Some(ws) = &self.websocket {
            let subscription = BinanceSubscription::kline_subscription(
                symbol.value(), 
                interval.to_binance_str()
            );
            
            let json = serde_json::to_string(&subscription)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
            
            ws.send_with_str(&json)?;
        }
        Ok(())
    }

    /// Отписаться от символа
    pub fn unsubscribe(&self, symbol: &Symbol, interval: TimeInterval) -> Result<(), JsValue> {
        if let Some(ws) = &self.websocket {
            let unsubscription = BinanceSubscription::unsubscribe(
                symbol.value(), 
                interval.to_binance_str()
            );
            
            let json = serde_json::to_string(&unsubscription)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;
            
            ws.send_with_str(&json)?;
        }
        Ok(())
    }
}

impl MarketDataRepository for BinanceWebSocketClient {
    fn get_historical_candles(
        &self,
        _symbol: &Symbol,
        _interval: TimeInterval,
        _limit: Option<usize>,
    ) -> Result<Vec<Candle>, JsValue> {
        Err(JsValue::from_str("Historical data not available via WebSocket"))
    }

    fn subscribe_to_updates(
        &mut self,
        symbol: &Symbol,
        interval: TimeInterval,
        callback: Box<dyn Fn(Candle)>,
    ) -> Result<(), JsValue> {
        self.set_candle_callback(callback);
        self.connect(symbol, interval)
    }

    fn unsubscribe(&mut self) -> Result<(), JsValue> {
        self.disconnect()
    }
}

/// Статический WebSocket клиент с callback системой
pub struct BinanceWebSocketClientWithCallback {
    websocket: Option<WebSocket>,
    candle_callback: Rc<RefCell<Option<Box<dyn Fn(Candle)>>>>,
    connected: Rc<RefCell<bool>>,
}

impl BinanceWebSocketClientWithCallback {
    pub fn new() -> Self {
        Self {
            websocket: None,
            candle_callback: Rc::new(RefCell::new(None)),
            connected: Rc::new(RefCell::new(false)),
        }
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.borrow()
    }

    pub fn connect_with_callback<F>(&mut self, symbol: &str, interval: &str, callback: F) -> Result<(), JsValue>
    where
        F: Fn(Candle) + 'static,
    {
        let ws_url = format!(
            "wss://stream.binance.com:9443/ws/{}@kline_{}",
            symbol.to_lowercase(),
            interval
        );

        log(&format!("Connecting to: {}", ws_url));
        update_ws_status("Connecting...", false);

        let ws = WebSocket::new(&ws_url)?;
        
        // Store callback
        *self.candle_callback.borrow_mut() = Some(Box::new(callback));
        
        // Setup handlers
        let connected_clone = self.connected.clone();
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            log("WebSocket connected successfully!");
            *connected_clone.borrow_mut() = true;
            update_ws_status("Connected ✅", true);
        }) as Box<dyn FnMut(JsValue)>);

        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        // Setup message handler
        let candle_callback_clone = self.candle_callback.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let data_str: String = txt.into();
                
                match serde_json::from_str::<BinanceKlineData>(&data_str) {
                    Ok(kline_data) => {
                        match kline_data.kline.to_domain_candle() {
                            Ok(candle) => {
                                // Вызываем callback
                                if let Some(callback) = candle_callback_clone.borrow().as_ref() {
                                    callback(candle);
                                }
                            }
                            Err(e) => {
                                log(&format!("Failed to convert kline: {:?}", e));
                            },
                        }
                    }
                    Err(e) => {
                        log(&format!("Failed to parse JSON: {}", e));
                    },
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // Error handler
        let connected_clone_error = self.connected.clone();
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            log(&format!("WebSocket error: {:?}", e));
            *connected_clone_error.borrow_mut() = false;
            update_ws_status("Error ❌", false);
        }) as Box<dyn FnMut(ErrorEvent)>);

        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        // Close handler
        let connected_clone_close = self.connected.clone();
        let onclose_callback = Closure::wrap(Box::new(move |e: CloseEvent| {
            log(&format!("WebSocket closed: {} - {}", e.code(), e.reason()));
            *connected_clone_close.borrow_mut() = false;
            update_ws_status("Disconnected", false);
        }) as Box<dyn FnMut(CloseEvent)>);

        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();

        self.websocket = Some(ws);
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), JsValue> {
        if let Some(ws) = &self.websocket {
            ws.close()?;
            self.websocket = None;
            *self.connected.borrow_mut() = false;
            update_ws_status("Disconnected", false);
        }
        Ok(())
    }
} 