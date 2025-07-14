//! Server error types and handling

use toadstool::ToadStoolError;

/// ToadStool server errors
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Server initialization failed: {0}")]
    Initialization(String),

    #[error("Runtime engine error: {0}")]
    RuntimeEngine(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhaustion(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Authorization failed: {0}")]
    Authorization(String),

    #[error("Invalid configuration: {0}")]
    Configuration(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Execution failed: {0}")]
    Execution(String),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl From<ToadStoolError> for ServerError {
    fn from(error: ToadStoolError) -> Self {
        ServerError::Internal(error.to_string())
    }
}

pub type ServerResult<T> = Result<T, ServerError>;
