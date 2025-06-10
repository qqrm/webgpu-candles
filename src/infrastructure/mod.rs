pub mod websocket;
pub mod rendering;

/// Infrastructure services
pub mod services {
    use crate::domain::logging::{Logger, LogEntry, LogLevel, TimeProvider, LogComponent};
    use gloo::console;

    /// Console logger implementation using gloo
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
                    format!("[{}] {} {} | {} | {}", timestamp, entry.level, entry.component, entry.message, metadata)
                }
                None => {
                    format!("[{}] {} {} | {}", timestamp, entry.level, entry.component, entry.message)
                }
            }
        }
    }

    impl Logger for ConsoleLogger {
        fn log(&self, entry: LogEntry) {
            if entry.level >= self.min_level {
                use crate::domain::logging::get_time_provider;
                let formatted = self.format_log_entry(&entry, get_time_provider());
                
                // Use gloo console methods
                match entry.level {
                    LogLevel::Trace | LogLevel::Debug => console::debug!("{}", formatted),
                    LogLevel::Info => console::info!("{}", formatted),
                    LogLevel::Warn => console::warn!("{}", formatted),
                    LogLevel::Error => console::error!("{}", formatted),
                }
            }
        }
    }

    /// Browser-based time provider using JS Date API
    pub struct BrowserTimeProvider;

    impl Default for BrowserTimeProvider {
        fn default() -> Self {
            Self::new()
        }
    }

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

    /// Initialize infrastructure services
    pub fn initialize_infrastructure_services() {
        use crate::domain::logging::{init_logger, init_time_provider, get_logger};

        // Initialize services
        let console_logger = ConsoleLogger::new_production();
        init_logger(Box::new(console_logger));

        let time_provider = BrowserTimeProvider::new();
        init_time_provider(Box::new(time_provider));

        // Log successful initialization
        get_logger().info(
            LogComponent::Infrastructure("Services"),
            "Infrastructure services initialized successfully"
        );
    }
}

pub use rendering::*;
pub use websocket::*;
pub use services::*; 