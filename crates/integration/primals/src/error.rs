use thiserror::Error;

pub type PrimalResult<T> = Result<T, PrimalError>;

/// Primal integration error types
#[derive(Debug, Error)]
pub enum PrimalError {
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    #[error("Network error: {source}")]
    Network { source: reqwest::Error },

    #[error("Authentication error: {message}")]
    Authentication { message: String },

    #[error("Service unavailable: {service}")]
    ServiceUnavailable { service: String },

    #[error("Integration error: {primal} - {message}")]
    Integration { primal: String, message: String },

    #[error("Timeout error: {operation}")]
    Timeout { operation: String },

    #[error("Validation error: {message}")]
    Validation { message: String },

    #[error("Resource error: {message}")]
    Resource { message: String },
}
