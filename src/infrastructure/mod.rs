pub mod websocket;
pub mod rendering;

/// Infrastructure implementations for domain abstractions
pub mod services {
    use crate::domain::logging::{Logger, LogEntry, LogLevel, TimeProvider};

    /// Console logger implementation for WASM environment
    pub struct ConsoleLogger {
        min_level: LogLevel,
    }

    impl ConsoleLogger {
        pub fn new(min_level: LogLevel) -> Self {
            Self { min_level }
        }

        pub fn new_production() -> Self {
            Self::new(LogLevel::Info)
        }

        pub fn new_development() -> Self {
            Self::new(LogLevel::Debug)
        }

        fn format_log_entry(&self, entry: &LogEntry, time_provider: &dyn TimeProvider) -> String {
            let timestamp = time_provider.format_timestamp(entry.timestamp);
            match &entry.metadata {
                Some(metadata) => {
                    format!(
                        "[{}] {} {} | {} | {}",
                        timestamp,
                        entry.level,
                        entry.component,
                        entry.message,
                        metadata
                    )
                }
                None => {
                    format!(
                        "[{}] {} {} | {}",
                        timestamp,
                        entry.level,
                        entry.component,
                        entry.message
                    )
                }
            }
        }
    }

    impl Logger for ConsoleLogger {
        fn log(&self, entry: LogEntry) {
            if entry.level >= self.min_level {
                use crate::domain::logging::get_time_provider;
                let formatted = self.format_log_entry(&entry, get_time_provider());
                
                // Use appropriate console method based on log level
                match entry.level {
                    LogLevel::Trace | LogLevel::Debug => {
                        #[allow(unused_unsafe)]
                        unsafe {
                            web_sys::console::debug_1(&formatted.into());
                        }
                    }
                    LogLevel::Info => {
                        #[allow(unused_unsafe)]
                        unsafe {
                            web_sys::console::info_1(&formatted.into());
                        }
                    }
                    LogLevel::Warn => {
                        #[allow(unused_unsafe)]
                        unsafe {
                            web_sys::console::warn_1(&formatted.into());
                        }
                    }
                    LogLevel::Error => {
                        #[allow(unused_unsafe)]
                        unsafe {
                            web_sys::console::error_1(&formatted.into());
                        }
                    }
                }
            }
        }
    }

    /// Browser-based time provider using JavaScript Date API
    pub struct BrowserTimeProvider;

    impl BrowserTimeProvider {
        pub fn new() -> Self {
            Self
        }
    }

    impl TimeProvider for BrowserTimeProvider {
        fn current_timestamp(&self) -> u64 {
            js_sys::Date::now() as u64
        }

        fn format_timestamp(&self, timestamp: u64) -> String {
            let date = js_sys::Date::new(&(timestamp as f64).into());
            format!(
                "{:02}:{:02}:{:02}.{:03}",
                date.get_hours(),
                date.get_minutes(),
                date.get_seconds(),
                date.get_milliseconds()
            )
        }
    }
}

/// UI interaction services (separate from domain logic)
pub mod ui {
    use crate::domain::{
        logging::{Logger, LogComponent, get_logger},
        errors::{InfrastructureError, UiError, PresentationError}
    };

    /// Service for updating UI elements without mixing with business logic
    #[derive(Clone)]
    pub struct UiNotificationService;

    impl UiNotificationService {
        pub fn new() -> Self {
            Self
        }

        /// Update WebSocket connection status in UI
        pub fn update_websocket_status(&self, status: &str, is_connected: bool) -> Result<(), InfrastructureError> {
            get_logger().debug(
                LogComponent::Infrastructure("UI"),
                &format!("Updating WebSocket status: {} (connected: {})", status, is_connected)
            );

            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(element) = document.get_element_by_id("ws-status") {
                        element.set_text_content(Some(&format!("WebSocket: {}", status)));
                        
                        let style_value = if is_connected {
                            "text-align: center; margin: 10px; padding: 10px; background: #006600; border-radius: 5px;"
                        } else {
                            "text-align: center; margin: 10px; padding: 10px; background: #660000; border-radius: 5px;"
                        };
                        
                        if let Err(_) = element.set_attribute("style", style_value) {
                            return Err(InfrastructureError::External(
                                crate::domain::errors::ExternalServiceError::BrowserApiError(
                                    "Failed to set element style".to_string()
                                )
                            ));
                        }
                    } else {
                        get_logger().warn(
                            LogComponent::Infrastructure("UI"),
                            "WebSocket status element 'ws-status' not found in DOM"
                        );
                    }
                } else {
                    return Err(InfrastructureError::External(
                        crate::domain::errors::ExternalServiceError::BrowserApiError(
                            "Document not available".to_string()
                        )
                    ));
                }
            } else {
                return Err(InfrastructureError::External(
                    crate::domain::errors::ExternalServiceError::BrowserApiError(
                        "Window not available".to_string()
                    )
                ));
            }

            Ok(())
        }

        /// Update chart loading status
        pub fn update_chart_status(&self, status: &str, progress: Option<u8>) -> Result<(), InfrastructureError> {
            let message = match progress {
                Some(pct) => format!("Chart: {} ({}%)", status, pct),
                None => format!("Chart: {}", status),
            };

            get_logger().debug(
                LogComponent::Infrastructure("UI"),
                &format!("Updating chart status: {}", message)
            );

            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(element) = document.get_element_by_id("chart-status") {
                        element.set_text_content(Some(&message));
                    } else {
                        get_logger().debug(
                            LogComponent::Infrastructure("UI"),
                            "Chart status element 'chart-status' not found (optional)"
                        );
                    }
                }
            }

            Ok(())
        }

        /// Update price display
        pub fn update_price_display(&self, symbol: &str, price: f32, change_percent: f32) -> Result<(), InfrastructureError> {
            get_logger().debug(
                LogComponent::Infrastructure("UI"),
                &format!("Updating price display: {} = ${:.2} ({:+.2}%)", symbol, price, change_percent)
            );

            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    // Update price
                    if let Some(price_element) = document.get_element_by_id("current-price") {
                        price_element.set_text_content(Some(&format!("${:.2}", price)));
                    }

                    // Update price change with color
                    if let Some(change_element) = document.get_element_by_id("price-change") {
                        change_element.set_text_content(Some(&format!("{:+.2}%", change_percent)));
                        
                        let color = if change_percent >= 0.0 { "#00ff00" } else { "#ff0000" };
                        let _ = change_element.set_attribute("style", &format!("color: {}", color));
                    }
                }
            }

            Ok(())
        }

        /// Show error notification in UI
        pub fn show_error_notification(&self, error_message: &str, error_type: &str) -> Result<(), InfrastructureError> {
            get_logger().error(
                LogComponent::Infrastructure("UI"),
                &format!("Showing error notification: [{}] {}", error_type, error_message)
            );

            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(error_container) = document.get_element_by_id("error-notifications") {
                        // Create error element
                        if let Ok(error_div) = document.create_element("div") {
                            error_div.set_text_content(Some(&format!("[{}] {}", error_type, error_message)));
                            let _ = error_div.set_attribute("style", 
                                "padding: 10px; margin: 5px; background: #ffeeee; border: 1px solid #ff0000; border-radius: 5px;");
                            
                            let _ = error_container.append_child(&error_div);
                        }
                    }
                }
            }

            Ok(())
        }

        /// Clear all notifications
        pub fn clear_notifications(&self) -> Result<(), InfrastructureError> {
            get_logger().debug(
                LogComponent::Infrastructure("UI"),
                "Clearing all UI notifications"
            );

            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(container) = document.get_element_by_id("error-notifications") {
                        container.set_inner_html("");
                    }
                }
            }

            Ok(())
        }
    }

    /// Trait for UI notification abstraction
    pub trait UiNotificationProvider {
        fn notify_connection_status(&self, status: &str, is_connected: bool) -> Result<(), InfrastructureError>;
        fn notify_data_update(&self, symbol: &str, price: f32, change: f32) -> Result<(), InfrastructureError>;
        fn notify_error(&self, error: &str, error_type: &str) -> Result<(), InfrastructureError>;
        fn clear_notifications(&self) -> Result<(), InfrastructureError>;
    }

    impl UiNotificationProvider for UiNotificationService {
        fn notify_connection_status(&self, status: &str, is_connected: bool) -> Result<(), InfrastructureError> {
            self.update_websocket_status(status, is_connected)
        }

        fn notify_data_update(&self, symbol: &str, price: f32, change: f32) -> Result<(), InfrastructureError> {
            self.update_price_display(symbol, price, change)
        }

        fn notify_error(&self, error: &str, error_type: &str) -> Result<(), InfrastructureError> {
            self.show_error_notification(error, error_type)
        }

        fn clear_notifications(&self) -> Result<(), InfrastructureError> {
            self.clear_notifications()
        }
    }
} 