pub mod market_data;
pub mod chart;

/// Domain Events infrastructure
pub mod events {
    use crate::domain::market_data::{Candle, Symbol, TimeInterval};
    use std::fmt::Debug;

    /// Base trait for all domain events
    pub trait DomainEvent: Debug + Clone {
        fn event_type(&self) -> &'static str;
        fn timestamp(&self) -> u64 {
            js_sys::Date::now() as u64
        }
    }

    /// Events related to market data
    #[derive(Debug, Clone)]
    pub enum MarketDataEvent {
        NewCandleReceived {
            symbol: Symbol,
            interval: TimeInterval,
            candle: Candle,
        },
        HistoricalDataLoaded {
            symbol: Symbol,
            interval: TimeInterval,
            candle_count: usize,
        },
        DataValidationFailed {
            symbol: Symbol,
            reason: String,
        },
        MarketDataConnectionEstablished {
            symbol: Symbol,
            interval: TimeInterval,
        },
        MarketDataConnectionLost {
            symbol: Symbol,
            reason: String,
        },
    }

    impl DomainEvent for MarketDataEvent {
        fn event_type(&self) -> &'static str {
            match self {
                MarketDataEvent::NewCandleReceived { .. } => "NewCandleReceived",
                MarketDataEvent::HistoricalDataLoaded { .. } => "HistoricalDataLoaded",
                MarketDataEvent::DataValidationFailed { .. } => "DataValidationFailed",
                MarketDataEvent::MarketDataConnectionEstablished { .. } => "MarketDataConnectionEstablished",
                MarketDataEvent::MarketDataConnectionLost { .. } => "MarketDataConnectionLost",
            }
        }
    }

    /// Events related to chart
    #[derive(Debug, Clone)]
    pub enum ChartEvent {
        ChartDataUpdated {
            chart_id: String,
            candle_count: usize,
        },
        ViewportChanged {
            chart_id: String,
            old_range: (f32, f32),
            new_range: (f32, f32),
        },
        ChartRenderingRequested {
            chart_id: String,
        },
    }

    impl DomainEvent for ChartEvent {
        fn event_type(&self) -> &'static str {
            match self {
                ChartEvent::ChartDataUpdated { .. } => "ChartDataUpdated",
                ChartEvent::ViewportChanged { .. } => "ViewportChanged",
                ChartEvent::ChartRenderingRequested { .. } => "ChartRenderingRequested",
            }
        }
    }

    /// Event dispatcher for publishing events
    pub trait EventDispatcher {
        fn publish_market_data_event(&self, event: MarketDataEvent);
        fn publish_chart_event(&self, event: ChartEvent);
    }

    /// Simple in-memory event dispatcher
    pub struct InMemoryEventDispatcher {
        market_data_handlers: Vec<Box<dyn Fn(&MarketDataEvent)>>,
        chart_handlers: Vec<Box<dyn Fn(&ChartEvent)>>,
    }

    impl InMemoryEventDispatcher {
        pub fn new() -> Self {
            Self {
                market_data_handlers: Vec::new(),
                chart_handlers: Vec::new(),
            }
        }

        pub fn subscribe_to_market_data_events<F>(&mut self, handler: F)
        where
            F: Fn(&MarketDataEvent) + 'static,
        {
            self.market_data_handlers.push(Box::new(handler));
        }

        pub fn subscribe_to_chart_events<F>(&mut self, handler: F)
        where
            F: Fn(&ChartEvent) + 'static,
        {
            self.chart_handlers.push(Box::new(handler));
        }
    }

    impl EventDispatcher for InMemoryEventDispatcher {
        fn publish_market_data_event(&self, event: MarketDataEvent) {
            for handler in &self.market_data_handlers {
                handler(&event);
            }
        }

        fn publish_chart_event(&self, event: ChartEvent) {
            for handler in &self.chart_handlers {
                handler(&event);
            }
        }
    }
}

/// Centralized logging system for the entire application
pub mod logging {
    use std::fmt::Display;

    /// Log levels for structured logging
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum LogLevel {
        Trace = 0,
        Debug = 1,
        Info = 2,
        Warn = 3,
        Error = 4,
    }

    impl Display for LogLevel {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LogLevel::Trace => write!(f, "TRACE"),
                LogLevel::Debug => write!(f, "DEBUG"),
                LogLevel::Info => write!(f, "INFO"),
                LogLevel::Warn => write!(f, "WARN"),
                LogLevel::Error => write!(f, "ERROR"),
            }
        }
    }

    /// Component/Layer identification for logging
    #[derive(Debug, Clone)]
    pub enum LogComponent {
        Domain(&'static str),      // e.g., "MarketData", "Chart"
        Application(&'static str), // e.g., "UseCase", "Coordinator"
        Infrastructure(&'static str), // e.g., "WebSocket", "HTTP", "WebGPU"
        Presentation(&'static str), // e.g., "WASM", "API"
    }

    impl Display for LogComponent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LogComponent::Domain(name) => write!(f, "🏛️ Domain::{}", name),
                LogComponent::Application(name) => write!(f, "🎯 Application::{}", name),
                LogComponent::Infrastructure(name) => write!(f, "🔧 Infrastructure::{}", name),
                LogComponent::Presentation(name) => write!(f, "🌐 Presentation::{}", name),
            }
        }
    }

    /// Structured log entry
    #[derive(Debug, Clone)]
    pub struct LogEntry {
        pub timestamp: u64,
        pub level: LogLevel,
        pub component: LogComponent,
        pub message: String,
        pub metadata: Option<String>,
    }

    /// Centralized logger trait
    pub trait Logger: Send + Sync {
        fn log(&self, entry: LogEntry);
        
        fn trace(&self, component: LogComponent, message: &str) {
            self.log(LogEntry::new(LogLevel::Trace, component, message.to_string()));
        }
        
        fn debug(&self, component: LogComponent, message: &str) {
            self.log(LogEntry::new(LogLevel::Debug, component, message.to_string()));
        }
        
        fn info(&self, component: LogComponent, message: &str) {
            self.log(LogEntry::new(LogLevel::Info, component, message.to_string()));
        }
        
        fn warn(&self, component: LogComponent, message: &str) {
            self.log(LogEntry::new(LogLevel::Warn, component, message.to_string()));
        }
        
        fn error(&self, component: LogComponent, message: &str) {
            self.log(LogEntry::new(LogLevel::Error, component, message.to_string()));
        }

        /// Log with metadata (e.g., JSON, additional context)
        fn log_with_metadata(&self, level: LogLevel, component: LogComponent, message: &str, metadata: &str) {
            self.log(LogEntry::new_with_metadata(level, component, message.to_string(), metadata.to_string()));
        }
    }

    impl LogEntry {
        pub fn new(level: LogLevel, component: LogComponent, message: String) -> Self {
            Self {
                timestamp: js_sys::Date::now() as u64,
                level,
                component,
                message,
                metadata: None,
            }
        }

        pub fn new_with_metadata(level: LogLevel, component: LogComponent, message: String, metadata: String) -> Self {
            Self {
                timestamp: js_sys::Date::now() as u64,
                level,
                component,
                message,
                metadata: Some(metadata),
            }
        }
    }

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

        fn format_log_entry(&self, entry: &LogEntry) -> String {
            let timestamp = Self::format_timestamp(entry.timestamp);
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

        fn format_timestamp(timestamp: u64) -> String {
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

    impl Logger for ConsoleLogger {
        fn log(&self, entry: LogEntry) {
            if entry.level >= self.min_level {
                let formatted = self.format_log_entry(&entry);
                
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

    /// Global logger instance using thread-safe static
    use std::sync::OnceLock;
    static GLOBAL_LOGGER: OnceLock<Box<dyn Logger + Sync + Send>> = OnceLock::new();

    /// Initialize global logger
    pub fn init_logger(logger: Box<dyn Logger + Sync + Send>) {
        let _ = GLOBAL_LOGGER.set(logger);
    }

    /// Get global logger reference
    pub fn get_logger() -> &'static dyn Logger {
        GLOBAL_LOGGER.get()
            .map(|logger| logger.as_ref())
            .unwrap_or_else(|| {
                // Fallback to a no-op logger if not initialized
                static FALLBACK: NoOpLogger = NoOpLogger;
                &FALLBACK
            })
    }

    /// No-op logger for fallback
    struct NoOpLogger;

    impl Logger for NoOpLogger {
        fn log(&self, _entry: LogEntry) {
            // No-op
        }
    }

    /// Convenience macros for logging
    #[macro_export]
    macro_rules! log_trace {
        ($component:expr, $($arg:tt)*) => {
            $crate::domain::logging::get_logger().trace($component, &format!($($arg)*));
        };
    }

    #[macro_export]
    macro_rules! log_debug {
        ($component:expr, $($arg:tt)*) => {
            $crate::domain::logging::get_logger().debug($component, &format!($($arg)*));
        };
    }

    #[macro_export]
    macro_rules! log_info {
        ($component:expr, $($arg:tt)*) => {
            $crate::domain::logging::get_logger().info($component, &format!($($arg)*));
        };
    }

    #[macro_export]
    macro_rules! log_warn {
        ($component:expr, $($arg:tt)*) => {
            $crate::domain::logging::get_logger().warn($component, &format!($($arg)*));
        };
    }

    #[macro_export]
    macro_rules! log_error {
        ($component:expr, $($arg:tt)*) => {
            $crate::domain::logging::get_logger().error($component, &format!($($arg)*));
        };
    }
}

/// Centralized error handling for the entire application
pub mod errors {
    use std::fmt::{Display, Formatter, Result as FmtResult};

    /// Root error type for the entire application
    #[derive(Debug, Clone)]
    pub enum AppError {
        Domain(DomainError),
        Application(ApplicationError),
        Infrastructure(InfrastructureError),
        Presentation(PresentationError),
    }

    /// Domain layer specific errors
    #[derive(Debug, Clone)]
    pub enum DomainError {
        Validation(ValidationError),
        Business(BusinessRuleError),
        Aggregate(AggregateError),
    }

    /// Validation errors from domain services
    #[derive(Debug, Clone)]
    pub enum ValidationError {
        InvalidCandle(String),
        InvalidSymbol(String),
        InvalidTimeInterval(String),
        InvalidPriceRange(String),
        InvalidSequence(String),
    }

    /// Business rule violations
    #[derive(Debug, Clone)]
    pub enum BusinessRuleError {
        PriceRangeViolation { min: f32, max: f32, actual: f32 },
        VolumeRangeViolation { min: f32, max: f32, actual: f32 },
        TimestampFutureViolation { max_allowed: u64, actual: u64 },
        OhlcLogicViolation(String),
    }

    /// Aggregate-specific errors
    #[derive(Debug, Clone)]
    pub enum AggregateError {
        CandleSeriesOverflow { max_size: usize, attempted_size: usize },
        ChartDataInconsistency(String),
        ViewportCalculationFailed(String),
    }

    /// Application layer errors
    #[derive(Debug, Clone)]
    pub enum ApplicationError {
        UseCase(UseCaseError),
        Coordination(CoordinationError),
        Configuration(ConfigurationError),
    }

    /// Use case specific errors
    #[derive(Debug, Clone)]
    pub enum UseCaseError {
        DataLoadingFailed(String),
        AnalysisFailed(String),
        RenderingPreparationFailed(String),
        ConnectionFailed(String),
    }

    /// Coordination errors between use cases
    #[derive(Debug, Clone)]
    pub enum CoordinationError {
        ServiceUnavailable(String),
        DependencyFailed(String),
        StateInconsistency(String),
    }

    /// Configuration errors
    #[derive(Debug, Clone)]
    pub enum ConfigurationError {
        InvalidParameter(String),
        MissingConfiguration(String),
        EnvironmentSetupFailed(String),
    }

    /// Infrastructure layer errors
    #[derive(Debug, Clone)]
    pub enum InfrastructureError {
        Repository(RepositoryError),
        Network(NetworkError),
        Rendering(RenderingError),
        External(ExternalServiceError),
    }

    /// Repository operation errors
    #[derive(Debug, Clone)]
    pub enum RepositoryError {
        NetworkError(String),
        ParseError(String),
        ValidationError(String),
        ConnectionError(String),
        SerializationError(String),
        DeserializationError(String),
    }

    /// Network-related errors
    #[derive(Debug, Clone)]
    pub enum NetworkError {
        WebSocketConnectionFailed(String),
        HttpRequestFailed(String),
        TimeoutError(String),
        AuthenticationFailed(String),
        RateLimitExceeded(String),
    }

    /// Rendering engine errors
    #[derive(Debug, Clone)]
    pub enum RenderingError {
        WebGpuInitializationFailed(String),
        ShaderCompilationFailed(String),
        BufferAllocationFailed(String),
        RenderingPipelineFailed(String),
        CanvasAccessFailed(String),
    }

    /// External service errors
    #[derive(Debug, Clone)]
    pub enum ExternalServiceError {
        BinanceApiError(String),
        WebApiError(String),
        BrowserApiError(String),
    }

    /// Presentation layer errors
    #[derive(Debug, Clone)]
    pub enum PresentationError {
        WasmBinding(WasmBindingError),
        JavaScript(JavaScriptError),
        UserInterface(UiError),
    }

    /// WASM binding errors
    #[derive(Debug, Clone)]
    pub enum WasmBindingError {
        SerializationFailed(String),
        DeserializationFailed(String),
        TypeConversionFailed(String),
        CallbackFailed(String),
    }

    /// JavaScript integration errors
    #[derive(Debug, Clone)]
    pub enum JavaScriptError {
        PromiseRejected(String),
        InvalidParameter(String),
        FunctionCallFailed(String),
    }

    /// User interface errors
    #[derive(Debug, Clone)]
    pub enum UiError {
        CanvasNotFound(String),
        ElementNotFound(String),
        InvalidDimensions(String),
        RenderingFailed(String),
    }

    // Display implementations for better error messages
    impl Display for AppError {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                AppError::Domain(e) => write!(f, "Domain Error: {}", e),
                AppError::Application(e) => write!(f, "Application Error: {}", e),
                AppError::Infrastructure(e) => write!(f, "Infrastructure Error: {}", e),
                AppError::Presentation(e) => write!(f, "Presentation Error: {}", e),
            }
        }
    }

    impl Display for DomainError {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                DomainError::Validation(e) => write!(f, "Validation: {}", e),
                DomainError::Business(e) => write!(f, "Business Rule: {}", e),
                DomainError::Aggregate(e) => write!(f, "Aggregate: {}", e),
            }
        }
    }

    impl Display for ValidationError {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                ValidationError::InvalidCandle(msg) => write!(f, "Invalid candle: {}", msg),
                ValidationError::InvalidSymbol(msg) => write!(f, "Invalid symbol: {}", msg),
                ValidationError::InvalidTimeInterval(msg) => write!(f, "Invalid time interval: {}", msg),
                ValidationError::InvalidPriceRange(msg) => write!(f, "Invalid price range: {}", msg),
                ValidationError::InvalidSequence(msg) => write!(f, "Invalid sequence: {}", msg),
            }
        }
    }

    impl Display for BusinessRuleError {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                BusinessRuleError::PriceRangeViolation { min, max, actual } => {
                    write!(f, "Price {} is outside valid range [{}, {}]", actual, min, max)
                },
                BusinessRuleError::VolumeRangeViolation { min, max, actual } => {
                    write!(f, "Volume {} is outside valid range [{}, {}]", actual, min, max)
                },
                BusinessRuleError::TimestampFutureViolation { max_allowed, actual } => {
                    write!(f, "Timestamp {} exceeds maximum allowed {}", actual, max_allowed)
                },
                BusinessRuleError::OhlcLogicViolation(msg) => write!(f, "OHLC logic violation: {}", msg),
            }
        }
    }

    impl Display for AggregateError {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                AggregateError::CandleSeriesOverflow { max_size, attempted_size } => {
                    write!(f, "CandleSeries overflow: attempted {} exceeds max {}", attempted_size, max_size)
                },
                AggregateError::ChartDataInconsistency(msg) => write!(f, "Chart data inconsistency: {}", msg),
                AggregateError::ViewportCalculationFailed(msg) => write!(f, "Viewport calculation failed: {}", msg),
            }
        }
    }

    impl Display for ApplicationError {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                ApplicationError::UseCase(e) => write!(f, "Use Case: {:?}", e),
                ApplicationError::Coordination(e) => write!(f, "Coordination: {:?}", e),
                ApplicationError::Configuration(e) => write!(f, "Configuration: {:?}", e),
            }
        }
    }

    impl Display for InfrastructureError {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                InfrastructureError::Repository(e) => write!(f, "Repository: {:?}", e),
                InfrastructureError::Network(e) => write!(f, "Network: {:?}", e),
                InfrastructureError::Rendering(e) => write!(f, "Rendering: {:?}", e),
                InfrastructureError::External(e) => write!(f, "External: {:?}", e),
            }
        }
    }

    impl Display for PresentationError {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                PresentationError::WasmBinding(e) => write!(f, "WASM Binding: {:?}", e),
                PresentationError::JavaScript(e) => write!(f, "JavaScript: {:?}", e),
                PresentationError::UserInterface(e) => write!(f, "UI: {:?}", e),
            }
        }
    }

    /// Error conversion utilities
    impl From<DomainError> for AppError {
        fn from(error: DomainError) -> Self {
            AppError::Domain(error)
        }
    }

    impl From<ApplicationError> for AppError {
        fn from(error: ApplicationError) -> Self {
            AppError::Application(error)
        }
    }

    impl From<InfrastructureError> for AppError {
        fn from(error: InfrastructureError) -> Self {
            AppError::Infrastructure(error)
        }
    }

    impl From<PresentationError> for AppError {
        fn from(error: PresentationError) -> Self {
            AppError::Presentation(error)
        }
    }

    /// Conversion from old RepositoryError to new error system
    impl From<crate::domain::market_data::repositories::RepositoryError> for InfrastructureError {
        fn from(error: crate::domain::market_data::repositories::RepositoryError) -> Self {
            match error {
                crate::domain::market_data::repositories::RepositoryError::NetworkError(msg) => {
                    InfrastructureError::Network(NetworkError::HttpRequestFailed(msg))
                },
                crate::domain::market_data::repositories::RepositoryError::ParseError(msg) => {
                    InfrastructureError::Repository(RepositoryError::ParseError(msg))
                },
                crate::domain::market_data::repositories::RepositoryError::ValidationError(msg) => {
                    InfrastructureError::Repository(RepositoryError::ValidationError(msg))
                },
                crate::domain::market_data::repositories::RepositoryError::ConnectionError(msg) => {
                    InfrastructureError::Repository(RepositoryError::ConnectionError(msg))
                },
            }
        }
    }
} 