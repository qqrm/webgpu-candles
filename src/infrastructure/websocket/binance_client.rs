use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebSocket, MessageEvent, ErrorEvent, CloseEvent};
use std::rc::Rc;
use std::cell::RefCell;

use crate::domain::market_data::{repositories::MarketDataRepository, Candle, Symbol, TimeInterval};
use super::dto::{BinanceKlineData, BinanceSubscription};

/// Binance WebSocket клиент - инфраструктурная реализация
pub struct BinanceWebSocketClient {
    websocket: Option<WebSocket>,
    url: String,
    on_candle_callback: Option<Box<dyn Fn(Candle)>>,
    connected: bool,
}

impl Default for BinanceWebSocketClient {
    fn default() -> Self {
        Self {
            websocket: None,
            url: "wss://stream.binance.com:9443/ws".to_string(),
            on_candle_callback: None,
            connected: false,
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
            on_candle_callback: None,
            connected: false,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn connect(&mut self, symbol: &Symbol, interval: TimeInterval) -> Result<(), JsValue> {
        let ws_url = format!(
            "{}/{}@kline_{}",
            self.url,
            symbol.value().to_lowercase(),
            interval.to_binance_str()
        );

        #[allow(unused_unsafe)] 
        unsafe { web_sys::console::log_1(&format!("Connecting to: {}", ws_url).into()); }

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
        self.on_candle_callback = Some(Box::new(callback));
    }

    fn setup_handlers(&mut self, ws: &WebSocket) -> Result<(), JsValue> {
        // Open handler
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            #[allow(unused_unsafe)] 
            unsafe { web_sys::console::log_1(&"WebSocket connected successfully!".into()) };
        }) as Box<dyn FnMut(JsValue)>);

        #[allow(unused_unsafe)] 
        unsafe {
            ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();
        }

        // Message handler
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let data_str: String = txt.into();
                
                match serde_json::from_str::<BinanceKlineData>(&data_str) {
                    Ok(kline_data) => {
                        match kline_data.kline.to_domain_candle() {
                            Ok(candle) => {
                                #[allow(unused_unsafe)] 
                                unsafe { web_sys::console::log_1(&format!(
                                    "Received candle: {} O:{} H:{} L:{} C:{} V:{}",
                                    candle.timestamp.value(),
                                    candle.ohlcv.open.value(),
                                    candle.ohlcv.high.value(),
                                    candle.ohlcv.low.value(),
                                    candle.ohlcv.close.value(),
                                    candle.ohlcv.volume.value()
                                ).into()) };

                                // TODO: Здесь нужно вызвать callback, но у нас есть проблема с ownership
                                // В реальной реализации можно использовать Rc<RefCell<>> или другие паттерны
                            }
                            Err(e) => {
                                #[allow(unused_unsafe)] 
                                unsafe { web_sys::console::error_1(&format!("Failed to convert kline: {:?}", e).into()); }
                            },
                        }
                    }
                    Err(e) => {
                        #[allow(unused_unsafe)] 
                        unsafe { web_sys::console::error_1(&format!("Failed to parse JSON: {}", e).into()); }
                    },
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        #[allow(unused_unsafe)] 
        unsafe {
            ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            onmessage_callback.forget();
        }

        // Error handler
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            #[allow(unused_unsafe)] 
            unsafe { web_sys::console::error_1(&format!("WebSocket error: {:?}", e).into()); }
        }) as Box<dyn FnMut(ErrorEvent)>);

        #[allow(unused_unsafe)] 
        unsafe {
            ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
            onerror_callback.forget();
        }

        // Close handler
        let onclose_callback = Closure::wrap(Box::new(move |e: CloseEvent| {
            #[allow(unused_unsafe)] 
            unsafe { web_sys::console::log_1(&format!("WebSocket closed: {} - {}", e.code(), e.reason()).into()) };
        }) as Box<dyn FnMut(CloseEvent)>);

        #[allow(unused_unsafe)] 
        unsafe {
            ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
            onclose_callback.forget();
        }

        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), JsValue> {
        if let Some(ws) = &self.websocket {
            ws.close()?;
            self.websocket = None;
            self.connected = false;
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
        // Для WebSocket клиента исторические данные не реализованы
        // В реальном приложении это должно быть в отдельном REST API клиенте
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
}

impl BinanceWebSocketClientWithCallback {
    pub fn new() -> Self {
        Self {
            websocket: None,
            candle_callback: Rc::new(RefCell::new(None)),
        }
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

        #[allow(unused_unsafe)]
        unsafe { web_sys::console::log_1(&format!("Connecting to: {}", ws_url).into()); }

        let ws = WebSocket::new(&ws_url)?;
        
        // Store callback
        *self.candle_callback.borrow_mut() = Some(Box::new(callback));
        
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
                                #[allow(unused_unsafe)] 
                                unsafe { web_sys::console::error_1(&format!("Failed to convert kline: {:?}", e).into()); }
                            },
                        }
                    }
                    Err(e) => {
                        #[allow(unused_unsafe)] 
                        unsafe { web_sys::console::error_1(&format!("Failed to parse JSON: {}", e).into()); }
                    },
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        #[allow(unused_unsafe)] 
        unsafe {
            ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            onmessage_callback.forget();
        }

        // Setup other handlers
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            #[allow(unused_unsafe)] 
            unsafe { web_sys::console::log_1(&"WebSocket connected successfully!".into()); }
        }) as Box<dyn FnMut(JsValue)>);

        #[allow(unused_unsafe)] 
        unsafe {
            ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();
        }

        self.websocket = Some(ws);
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), JsValue> {
        if let Some(ws) = &self.websocket {
            ws.close()?;
            self.websocket = None;
        }
        Ok(())
    }
} 