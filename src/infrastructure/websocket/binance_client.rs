use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebSocket, MessageEvent, ErrorEvent, CloseEvent};
use std::rc::Rc;
use std::cell::RefCell;

use crate::domain::{
    market_data::{repositories::MarketDataRepository, Candle, Symbol, TimeInterval},
    logging::{LogComponent, get_logger},
    errors::{InfrastructureError, RepositoryError},
};
use crate::infrastructure::ui::{UiNotificationService, UiNotificationProvider};
use super::dto::{BinanceKlineData, BinanceSubscription};

/// Binance WebSocket клиент - инфраструктурная реализация
pub struct BinanceWebSocketClient {
    websocket: Option<WebSocket>,
    url: String,
    on_candle_callback: Rc<RefCell<Option<Box<dyn Fn(Candle)>>>>,
    connected: Rc<RefCell<bool>>,
    ui_service: UiNotificationService,
}

impl Default for BinanceWebSocketClient {
    fn default() -> Self {
        Self {
            websocket: None,
            url: "wss://stream.binance.com:9443/ws".to_string(),
            on_candle_callback: Rc::new(RefCell::new(None)),
            connected: Rc::new(RefCell::new(false)),
            ui_service: UiNotificationService::new(),
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
            ui_service: UiNotificationService::new(),
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

        get_logger().info(
            LogComponent::Infrastructure("WebSocket"),
            &format!("Connecting to: {}", ws_url)
        );
        
        let _ = self.ui_service.notify_connection_status("Connecting...", false);

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
        
        get_logger().debug(
            LogComponent::Infrastructure("WebSocket"),
            "Candle callback set"
        );
    }

    fn setup_handlers(&mut self, ws: &WebSocket) -> Result<(), JsValue> {
        let connected_clone = self.connected.clone();
        let ui_service_clone = self.ui_service.clone();
        
        // Open handler
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            get_logger().info(
                LogComponent::Infrastructure("WebSocket"),
                "WebSocket connected successfully!"
            );
            *connected_clone.borrow_mut() = true;
            let _ = ui_service_clone.notify_connection_status("Connected ✅", true);
        }) as Box<dyn FnMut(JsValue)>);

        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        // Message handler with proper callback
        let callback_clone = self.on_candle_callback.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let data_str: String = txt.into();
                
                match serde_json::from_str::<BinanceKlineData>(&data_str) {
                    Ok(kline_data) => {
                        match kline_data.kline.to_domain_candle() {
                            Ok(candle) => {
                                get_logger().debug(
                                    LogComponent::Infrastructure("WebSocket"),
                                    &format!(
                                        "Received candle - {} O:{} H:{} L:{} C:{} V:{}",
                                        candle.timestamp.value(),
                                        candle.ohlcv.open.value(),
                                        candle.ohlcv.high.value(),
                                        candle.ohlcv.low.value(),
                                        candle.ohlcv.close.value(),
                                        candle.ohlcv.volume.value()
                                    )
                                );
                                
                                // Call callback properly
                                if let Some(callback) = callback_clone.borrow().as_ref() {
                                    get_logger().debug(
                                        LogComponent::Infrastructure("WebSocket"),
                                        "Calling Application Layer callback"
                                    );
                                    callback(candle);
                                } else {
                                    get_logger().warn(
                                        LogComponent::Infrastructure("WebSocket"),
                                        "No callback set, data will be lost"
                                    );
                                }
                            }
                            Err(e) => {
                                get_logger().error(
                                    LogComponent::Infrastructure("WebSocket"),
                                    &format!("Failed to convert kline: {:?}", e)
                                );
                            },
                        }
                    }
                    Err(e) => {
                        get_logger().error(
                            LogComponent::Infrastructure("WebSocket"),
                            &format!("Failed to parse JSON: {}", e)
                        );
                    },
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // Error handler
        let connected_clone_error = self.connected.clone();
        let ui_service_error = self.ui_service.clone();
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            get_logger().error(
                LogComponent::Infrastructure("WebSocket"),
                &format!("WebSocket error: {:?}", e)
            );
            *connected_clone_error.borrow_mut() = false;
            let _ = ui_service_error.notify_connection_status("Error ❌", false);
        }) as Box<dyn FnMut(ErrorEvent)>);

        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        // Close handler
        let connected_clone_close = self.connected.clone();
        let ui_service_close = self.ui_service.clone();
        let onclose_callback = Closure::wrap(Box::new(move |e: CloseEvent| {
            get_logger().info(
                LogComponent::Infrastructure("WebSocket"),
                &format!("WebSocket closed: {} - {}", e.code(), e.reason())
            );
            *connected_clone_close.borrow_mut() = false;
            let _ = ui_service_close.notify_connection_status("Disconnected", false);
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
            let _ = self.ui_service.notify_connection_status("Disconnected", false);
        }
        Ok(())
    }

    /// Send subscription for symbol
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

    /// Unsubscribe from symbol
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
    ) -> Result<Vec<Candle>, crate::domain::market_data::repositories::RepositoryError> {
        Err(crate::domain::market_data::repositories::RepositoryError::NetworkError(
            "Historical data not available via WebSocket".to_string()
        ))
    }

    fn subscribe_to_updates(
        &mut self,
        symbol: &Symbol,
        interval: TimeInterval,
        callback: Box<dyn Fn(Candle)>,
    ) -> Result<(), crate::domain::market_data::repositories::RepositoryError> {
        self.set_candle_callback(callback);
        self.connect(symbol, interval)
            .map_err(|e| crate::domain::market_data::repositories::RepositoryError::ConnectionError(
                format!("WebSocket connection failed: {:?}", e)
            ))
    }

    fn unsubscribe(&mut self) -> Result<(), crate::domain::market_data::repositories::RepositoryError> {
        self.disconnect()
            .map_err(|e| crate::domain::market_data::repositories::RepositoryError::ConnectionError(
                format!("WebSocket disconnection failed: {:?}", e)
            ))
    }
}

/// Static WebSocket client with callback system
pub struct BinanceWebSocketClientWithCallback {
    websocket: Option<WebSocket>,
    candle_callback: Rc<RefCell<Option<Box<dyn Fn(Candle)>>>>,
    connected: Rc<RefCell<bool>>,
    ui_service: UiNotificationService,
}

impl BinanceWebSocketClientWithCallback {
    pub fn new() -> Self {
        Self {
            websocket: None,
            candle_callback: Rc::new(RefCell::new(None)),
            connected: Rc::new(RefCell::new(false)),
            ui_service: UiNotificationService::new(),
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

        get_logger().info(
            LogComponent::Infrastructure("WebSocket"),
            &format!("Connecting to: {}", ws_url)
        );
        
        let _ = self.ui_service.notify_connection_status("Connecting...", false);

        let ws = WebSocket::new(&ws_url)?;
        
        // Store callback
        *self.candle_callback.borrow_mut() = Some(Box::new(callback));
        
        // Setup handlers
        let connected_clone = self.connected.clone();
        let ui_service_clone = self.ui_service.clone();
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            get_logger().info(
                LogComponent::Infrastructure("WebSocket"),
                "WebSocket connected successfully!"
            );
            *connected_clone.borrow_mut() = true;
            let _ = ui_service_clone.notify_connection_status("Connected ✅", true);
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
                                // Call callback
                                if let Some(callback) = candle_callback_clone.borrow().as_ref() {
                                    callback(candle);
                                }
                            }
                            Err(e) => {
                                get_logger().error(
                                    LogComponent::Infrastructure("WebSocket"),
                                    &format!("Failed to convert kline: {:?}", e)
                                );
                            },
                        }
                    }
                    Err(e) => {
                        get_logger().error(
                            LogComponent::Infrastructure("WebSocket"),
                            &format!("Failed to parse JSON: {}", e)
                        );
                    },
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // Error handler
        let connected_clone_error = self.connected.clone();
        let ui_service_error = self.ui_service.clone();
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            get_logger().error(
                LogComponent::Infrastructure("WebSocket"),
                &format!("WebSocket error: {:?}", e)
            );
            *connected_clone_error.borrow_mut() = false;
            let _ = ui_service_error.notify_connection_status("Error ❌", false);
        }) as Box<dyn FnMut(ErrorEvent)>);

        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        // Close handler
        let connected_clone_close = self.connected.clone();
        let ui_service_close = self.ui_service.clone();
        let onclose_callback = Closure::wrap(Box::new(move |e: CloseEvent| {
            get_logger().info(
                LogComponent::Infrastructure("WebSocket"),
                &format!("WebSocket closed: {} - {}", e.code(), e.reason())
            );
            *connected_clone_close.borrow_mut() = false;
            let _ = ui_service_close.notify_connection_status("Disconnected", false);
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
            let _ = self.ui_service.notify_connection_status("Disconnected", false);
        }
        Ok(())
    }
} 