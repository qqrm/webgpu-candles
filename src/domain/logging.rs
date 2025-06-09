use derive_more::Display;

/// Log levels with automatic Display implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display)]
pub enum LogLevel {
    #[display(fmt = "TRACE")]
    Trace = 0,
    #[display(fmt = "DEBUG")]
    Debug = 1,
    #[display(fmt = " INFO")]  
    Info = 2,
    #[display(fmt = " WARN")]
    Warn = 3,
    #[display(fmt = "ERROR")]
    Error = 4,
}

/// Log components with automatic Display implementation
#[derive(Debug, Clone, Display)]
pub enum LogComponent {
    #[display(fmt = "DOM:{}", _0)]
    Domain(&'static str),
    #[display(fmt = "APP:{}", _0)]
    Application(&'static str),
    #[display(fmt = "INF:{}", _0)]
    Infrastructure(&'static str),
    #[display(fmt = "PRE:{}", _0)]
    Presentation(&'static str),
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

/// Domain abstraction for time service
pub trait TimeProvider: Send + Sync {
    fn current_timestamp(&self) -> u64;
    fn format_timestamp(&self, timestamp: u64) -> String;
}

/// Domain abstraction for structured logging
pub trait Logger: Send + Sync {
    fn log(&self, entry: LogEntry);
    
    /// Convenience methods with default implementations
    fn trace(&self, component: LogComponent, message: &str) {
        self.log(LogEntry::new(LogLevel::Trace, component, message));
    }
    
    fn debug(&self, component: LogComponent, message: &str) {
        self.log(LogEntry::new(LogLevel::Debug, component, message));
    }
    
    fn info(&self, component: LogComponent, message: &str) {
        self.log(LogEntry::new(LogLevel::Info, component, message));
    }
    
    fn warn(&self, component: LogComponent, message: &str) {
        self.log(LogEntry::new(LogLevel::Warn, component, message));
    }
    
    fn error(&self, component: LogComponent, message: &str) {
        self.log(LogEntry::new(LogLevel::Error, component, message));
    }

    /// Log with structured metadata
    fn log_with_metadata(&self, level: LogLevel, component: LogComponent, message: &str, metadata: &str) {
        self.log(LogEntry::new_with_metadata(level, component, message, metadata));
    }
}

impl LogEntry {
    pub fn new(level: LogLevel, component: LogComponent, message: &str) -> Self {
        Self {
            timestamp: get_time_provider().current_timestamp(),
            level,
            component,
            message: message.to_string(),
            metadata: None,
        }
    }

    pub fn new_with_metadata(level: LogLevel, component: LogComponent, message: &str, metadata: &str) -> Self {
        Self {
            timestamp: get_time_provider().current_timestamp(),
            level,
            component,
            message: message.to_string(),
            metadata: Some(metadata.to_string()),
        }
    }
}

/// Global services using thread-safe statics
use std::sync::OnceLock;
static GLOBAL_LOGGER: OnceLock<Box<dyn Logger + Sync + Send>> = OnceLock::new();
static GLOBAL_TIME_PROVIDER: OnceLock<Box<dyn TimeProvider + Sync + Send>> = OnceLock::new();

/// Initialize global logger
pub fn init_logger(logger: Box<dyn Logger + Sync + Send>) {
    let _ = GLOBAL_LOGGER.set(logger);
}

/// Initialize global time provider
pub fn init_time_provider(time_provider: Box<dyn TimeProvider + Sync + Send>) {
    let _ = GLOBAL_TIME_PROVIDER.set(time_provider);
}

/// Get global logger reference
pub fn get_logger() -> &'static dyn Logger {
    GLOBAL_LOGGER.get()
        .map(|logger| logger.as_ref())
        .unwrap_or(&NoOpLogger)
}

/// Get global time provider reference
pub fn get_time_provider() -> &'static dyn TimeProvider {
    GLOBAL_TIME_PROVIDER.get()
        .map(|provider| provider.as_ref())
        .unwrap_or(&BasicTimeProvider)
}

/// No-op logger fallback
struct NoOpLogger;
impl Logger for NoOpLogger {
    fn log(&self, _entry: LogEntry) {}
}

/// Basic time provider fallback
struct BasicTimeProvider;
impl TimeProvider for BasicTimeProvider {
    fn current_timestamp(&self) -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    fn format_timestamp(&self, timestamp: u64) -> String {
        format!("{:06}", timestamp)
    }
}

/// Simplified logging macros
#[macro_export]
macro_rules! log_trace {
    ($component:expr, $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            $crate::domain::logging::get_logger().trace($component, &format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! log_debug {
    ($component:expr, $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            $crate::domain::logging::get_logger().debug($component, &format!($($arg)*));
        }
    };
}

#[macro_export]
macro_rules! log_info {
    ($component:expr, $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            $crate::domain::logging::get_logger().info($component, &format!($($arg)*));
        }
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