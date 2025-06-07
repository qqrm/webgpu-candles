use thiserror::Error;

/// Root error type for the entire application
#[derive(Error, Debug, Clone)]
pub enum AppError {
    #[error("Domain Error: {0}")]
    Domain(#[from] DomainError),
    #[error("Application Error: {0}")]
    Application(#[from] ApplicationError),
    #[error("Infrastructure Error: {0}")]
    Infrastructure(#[from] InfrastructureError),
    #[error("Presentation Error: {0}")]
    Presentation(#[from] PresentationError),
}

/// Domain layer specific errors
#[derive(Error, Debug, Clone)]
pub enum DomainError {
    #[error("Validation Error: {0}")]
    Validation(#[from] ValidationError),
    #[error("Business Rule Error: {0}")]
    Business(#[from] BusinessRuleError),
    #[error("Aggregate Error: {0}")]
    Aggregate(#[from] AggregateError),
}

/// Validation errors from domain services
#[derive(Error, Debug, Clone)]
pub enum ValidationError {
    #[error("Invalid Candle: {0}")]
    InvalidCandle(String),
    #[error("Invalid Symbol: {0}")]
    InvalidSymbol(String),
    #[error("Invalid Time Interval: {0}")]
    InvalidTimeInterval(String),
    #[error("Invalid Price Range: {0}")]
    InvalidPriceRange(String),
    #[error("Invalid Sequence: {0}")]
    InvalidSequence(String),
}

/// Business rule violations
#[derive(Error, Debug, Clone)]
pub enum BusinessRuleError {
    #[error("Price {actual} is outside allowed range [{min}, {max}]")]
    PriceRangeViolation { min: f32, max: f32, actual: f32 },
    #[error("Volume {actual} is outside allowed range [{min}, {max}]")]
    VolumeRangeViolation { min: f32, max: f32, actual: f32 },
    #[error("Timestamp {actual} is in the future (max allowed: {max_allowed})")]
    TimestampFutureViolation { max_allowed: u64, actual: u64 },
    #[error("OHLC Logic Violation: {0}")]
    OhlcLogicViolation(String),
}

/// Aggregate-specific errors
#[derive(Error, Debug, Clone)]
pub enum AggregateError {
    #[error("Candle series overflow: attempted {attempted_size} but max is {max_size}")]
    CandleSeriesOverflow { max_size: usize, attempted_size: usize },
    #[error("Chart Data Inconsistency: {0}")]
    ChartDataInconsistency(String),
    #[error("Viewport Calculation Failed: {0}")]
    ViewportCalculationFailed(String),
}

/// Application layer errors
#[derive(Error, Debug, Clone)]
pub enum ApplicationError {
    #[error("Use Case Error: {0}")]
    UseCase(#[from] UseCaseError),
    #[error("Coordination Error: {0}")]
    Coordination(#[from] CoordinationError),
    #[error("Configuration Error: {0}")]
    Configuration(#[from] ConfigurationError),
}

/// Use case specific errors
#[derive(Error, Debug, Clone)]
pub enum UseCaseError {
    #[error("Data loading failed: {0}")]
    DataLoadingFailed(String),
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
    #[error("Rendering preparation failed: {0}")]
    RenderingPreparationFailed(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}

/// Coordination errors between use cases
#[derive(Error, Debug, Clone)]
pub enum CoordinationError {
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("Dependency failed: {0}")]
    DependencyFailed(String),
    #[error("State inconsistency: {0}")]
    StateInconsistency(String),
}

/// Configuration errors
#[derive(Error, Debug, Clone)]
pub enum ConfigurationError {
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Missing configuration: {0}")]
    MissingConfiguration(String),
    #[error("Environment setup failed: {0}")]
    EnvironmentSetupFailed(String),
}

/// Infrastructure layer errors
#[derive(Error, Debug, Clone)]
pub enum InfrastructureError {
    #[error("Network Error: {0}")]
    Network(#[from] NetworkError),
    #[error("Rendering Error: {0}")]
    Rendering(#[from] RenderingError),
    #[error("External Service Error: {0}")]
    External(#[from] ExternalServiceError),
}

/// Network-related errors
#[derive(Error, Debug, Clone)]
pub enum NetworkError {
    #[error("WebSocket connection failed: {0}")]
    WebSocketConnectionFailed(String),
    #[error("HTTP request failed: {0}")]
    HttpRequestFailed(String),
    #[error("Timeout error: {0}")]
    TimeoutError(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),
}

/// Rendering engine errors
#[derive(Error, Debug, Clone)]
pub enum RenderingError {
    #[error("WebGPU initialization failed: {0}")]
    WebGpuInitializationFailed(String),
    #[error("Shader compilation failed: {0}")]
    ShaderCompilationFailed(String),
    #[error("Buffer allocation failed: {0}")]
    BufferAllocationFailed(String),
    #[error("Rendering pipeline failed: {0}")]
    RenderingPipelineFailed(String),
    #[error("Canvas access failed: {0}")]
    CanvasAccessFailed(String),
}

/// External service errors
#[derive(Error, Debug, Clone)]
pub enum ExternalServiceError {
    #[error("Binance API error: {0}")]
    BinanceApiError(String),
    #[error("Web API error: {0}")]
    WebApiError(String),
    #[error("Browser API error: {0}")]
    BrowserApiError(String),
}

/// Presentation layer errors
#[derive(Error, Debug, Clone)]
pub enum PresentationError {
    #[error("WASM Binding Error: {0}")]
    WasmBinding(#[from] WasmBindingError),
    #[error("JavaScript Error: {0}")]
    JavaScript(#[from] JavaScriptError),
    #[error("UI Error: {0}")]
    UserInterface(#[from] UiError),
}

/// WASM binding errors
#[derive(Error, Debug, Clone)]
pub enum WasmBindingError {
    #[error("Serialization failed: {0}")]
    SerializationFailed(String),
    #[error("Deserialization failed: {0}")]
    DeserializationFailed(String),
    #[error("Type conversion failed: {0}")]
    TypeConversionFailed(String),
    #[error("Callback failed: {0}")]
    CallbackFailed(String),
}

/// JavaScript integration errors
#[derive(Error, Debug, Clone)]
pub enum JavaScriptError {
    #[error("Promise rejected: {0}")]
    PromiseRejected(String),
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
    #[error("Function call failed: {0}")]
    FunctionCallFailed(String),
}

/// User interface errors
#[derive(Error, Debug, Clone)]
pub enum UiError {
    #[error("Canvas not found: {0}")]
    CanvasNotFound(String),
    #[error("Invalid dimensions: {0}")]
    InvalidDimensions(String),
    #[error("Rendering failed: {0}")]
    RenderingFailed(String),
}

// Repository error conversion removed - no longer needed since repositories are deleted 