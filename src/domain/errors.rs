/// Simplified error system - no over-engineering!
#[derive(Debug, Clone)]
pub enum AppError {
    NetworkError(String),
    RenderingError(String),
    ValidationError(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NetworkError(msg) => write!(f, "Network Error: {}", msg),
            AppError::RenderingError(msg) => write!(f, "Rendering Error: {}", msg),
            AppError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

// Simple convenience type aliases
pub type NetworkResult<T> = Result<T, AppError>;
pub type RenderingResult<T> = Result<T, AppError>;
